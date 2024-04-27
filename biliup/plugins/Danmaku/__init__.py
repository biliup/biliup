# 部分弹幕功能代码来自项目：https://github.com/IsoaSFlus/danmaku，感谢大佬
# 快手弹幕代码来源及思路：https://github.com/py-wuhao/ks_barrage，感谢大佬
# 部分斗鱼录播修复代码与思路来源于：https://github.com/SmallPeaches/DanmakuRender，感谢大佬
# 仅抓取用户弹幕，不包括入场提醒、礼物赠送等。

import asyncio
import logging
import os
import re
import ssl
import threading
import time
from asyncio import AbstractEventLoop
from typing import Optional

import aiohttp
import lxml.etree as etree

from biliup.plugins.Danmaku.bilibili import Bilibili
from biliup.plugins.Danmaku.douyin import Douyin
from biliup.plugins.Danmaku.douyu import Douyu
from biliup.plugins.Danmaku.huya import Huya
from biliup.plugins.Danmaku.twitcasting import Twitcasting
from biliup.plugins.Danmaku.twitch import Twitch

logger = logging.getLogger('biliup')


class DanmakuClient:
    class WebsocketErrorException(Exception):
        pass

    def __init__(self, url, file_name, content=None):
        # TODO 录制任务产生的上下文信息 传递太麻烦了 需要改
        self.__content = content if content is not None else {}
        self.__file_name = file_name
        self.__fmt_file_name = None
        self.__start_time = None
        self.__url = ''
        self.__site = None
        self.__hs = None
        self.__ws = None
        self.__dm_queue = None
        self.__record_task: Optional[asyncio.Task] = None
        self.__print_task: Optional[asyncio.Task] = None
        self.__loop: Optional[AbstractEventLoop] = None

        if 'http://' == url[:7] or 'https://' == url[:8]:
            self.__url = url
        else:
            self.__url = 'http://' + url
        for u, s in {'douyu.com': Douyu,
                     'huya.com': Huya,
                     'live.bilibili.com': Bilibili,
                     'twitch.tv': Twitch,
                     'douyin.com': Douyin,
                     'twitcasting.tv': Twitcasting
                     }.items():
            if re.match(r'^(?:http[s]?://)?.*?%s/(.+?)$' % u, url):
                self.__site = s
                self.__u = u
                break

        if self.__site is None:
            # 抛出异常由外部处理 exit()会导致进程退出
            raise Exception(f"{DanmakuClient.__name__}:{self.__url}: 不支持录制弹幕")

    async def __init_ws(self):
        try:
            ws_url, reg_datas = await self.__site.get_ws_info(self.__url, self.__content)
            ctx = ssl.create_default_context()
            ctx.set_ciphers('DEFAULT')
            self.__ws = await self.__hs.ws_connect(ws_url, ssl_context=ctx, headers=getattr(self.__site, 'headers', {}))
            for reg_data in reg_datas:
                if type(reg_data) == str:
                    await self.__ws.send_str(reg_data)
                else:
                    await self.__ws.send_bytes(reg_data)
        except asyncio.CancelledError:
            raise
        except:
            raise self.WebsocketErrorException()

    async def __heartbeats(self):
        if self.__site.heartbeat is not None:
            while self.__site.heartbeat:
                # 每隔这么长时间发送一次心跳包
                await asyncio.sleep(self.__site.heartbeatInterval)
                # 发送心跳包
                if type(self.__site.heartbeat) == str:
                    await self.__ws.send_str(self.__site.heartbeat)
                else:
                    await self.__ws.send_bytes(self.__site.heartbeat)

    async def __fetch_danmaku(self):
        while True:
            # 使用 async for msg in self.__ws
            # 会导致在连接断开时 需要很长时间15min或者更多才能检测到
            msg = await self.__ws.receive()

            if msg.type in [aiohttp.WSMsgType.CLOSED, aiohttp.WSMsgType.ERROR]:
                # 连接关闭的异常 会到外层统一处理
                raise self.WebsocketErrorException()

            try:
                result = self.__site.decode_msg(msg.data)

                if isinstance(result, tuple):
                    ms, ack = result
                    if ack is not None:
                        # 发送ack包
                        if type(ack) == str:
                            await self.__ws.send_str(ack)
                        else:
                            await self.__ws.send_bytes(ack)
                else:
                    ms = result

                for m in ms:
                    await self.__dm_queue.put(m)
            except asyncio.CancelledError:
                raise
            except:
                logger.exception(f"{DanmakuClient.__name__}:{self.__url}: 弹幕接收异常")
                continue
                # await asyncio.sleep(10) 无需等待 直接获取下一条websocket消息
                # 这里出现异常只会是 decode_msg 的问题

    async def __print_danmaku(self, file_name):
        root = etree.Element("root")
        etree.indent(root, "\t")
        tree = etree.ElementTree(root, parser=etree.XMLParser(recover=True))
        msg_i = 0
        msg_col = {'0': '16777215', '1': '16717077', '2': '2000880', '3': '8046667', '4': '16744192',
                   '5': '10172916',
                   '6': '16738740'}

        def write_file(filename):
            try:
                with open(filename, "wb") as f:
                    tree.write(f, encoding="UTF-8", xml_declaration=True, pretty_print=True)
            except Exception as e:
                logger.warning(f"{DanmakuClient.__name__}:{self.__url}: 弹幕写入异常 - {e}")

        try:
            while True:
                m = await self.__dm_queue.get()
                if m.get('msg_type') == 'danmaku':
                    try:
                        d = etree.SubElement(root, 'd')
                        if 'col' in m:
                            color = msg_col[m["col"]]
                        elif 'color' in m:
                            color = m["color"]
                        else:
                            color = '16777215'
                        msg_time = format(time.time() - self.__start_time, '.3f')
                        d.set('p', f"{msg_time},1,25,{color},0,0,0,0")
                        d.text = m["content"]
                    except:
                        logger.exception(f"{DanmakuClient.__name__}:{self.__url}:弹幕处理异常")
                        # 异常后略过本次弹幕
                        continue

                    if msg_i >= 5:
                        # 收到五条弹幕后写入 减少io
                        # 可能会写入失败 会在下次五条或者任务被取消时重新尝试写入
                        write_file(file_name)
                        msg_i = 0
                    else:
                        msg_i = msg_i + 1
        finally:
            # 发生异常(被取消)时写入 避免丢失未写入
            write_file(file_name)

    def start(self):
        init_event = threading.Event()

        async def __init():
            logger.info(f'开始弹幕录制: {self.__url}')
            self.__loop = asyncio.get_event_loop()
            self.__record_task = asyncio.create_task(self.__run())
            self.segment()
            init_event.set()
            try:
                await self.__record_task
            except asyncio.CancelledError:
                pass
            logger.info(f'结束弹幕录制: {self.__url}')

        threading.Thread(target=asyncio.run, args=(__init(),)).start()
        # 等待初始化完成避免未初始化完成的时候就停止任务
        init_event.wait()

    def segment(self, new_prev_file_name=None):
        if self.__record_task and self.__loop:
            if new_prev_file_name and self.__fmt_file_name and new_prev_file_name != self.__fmt_file_name:
                try:
                    if os.path.exists(self.__fmt_file_name):
                        os.rename(self.__fmt_file_name, new_prev_file_name)
                        logger.info(
                            f"{DanmakuClient.__name__}:{self.__url}: 更名 {self.__fmt_file_name} 为 {new_prev_file_name}")
                except:
                    logger.error(
                        f"{DanmakuClient.__name__}:{self.__url}: 更名 {self.__fmt_file_name} 为 {new_prev_file_name} 失败",
                        exc_info=True)
            self.__fmt_file_name = time.strftime(self.__file_name.encode("unicode-escape").decode()).encode().decode(
                "unicode-escape") + '.xml'
            self.__start_time = time.time()
            if self.__print_task:
                self.__print_task.cancel()
            self.__print_task = self.__loop.create_task(self.__print_danmaku(self.__fmt_file_name))

    def stop(self):
        if self.__record_task is not None:
            self.__record_task.cancel()
            self.__record_task = None
            self.__loop = None

    async def __run(self):
        self.__dm_queue = asyncio.Queue()
        self.__hs = aiohttp.ClientSession()
        try:
            while True:
                danmaku_tasks = None
                try:
                    await self.__init_ws()
                    danmaku_tasks = [asyncio.create_task(self.__heartbeats()),
                                     asyncio.create_task(self.__fetch_danmaku())]
                    await asyncio.gather(*danmaku_tasks)
                except asyncio.CancelledError:
                    raise
                except self.WebsocketErrorException:
                    # 连接断开等30秒重连
                    # 在关闭之前一直重试
                    logger.warning(f"{DanmakuClient.__name__}:{self.__url}: 弹幕连接异常,将在 30 秒后重试")
                except:
                    # 记录异常不到外部处理
                    logger.exception(f"{DanmakuClient.__name__}:{self.__url}: 弹幕异常,将在 30 秒后重试")
                finally:
                    if danmaku_tasks is not None:
                        for danmaku_task in danmaku_tasks:
                            danmaku_task.cancel()
                        await asyncio.gather(*danmaku_tasks, return_exceptions=True)
                    if self.__ws is not None and not self.__ws.closed:
                        await self.__ws.close()
                await asyncio.sleep(30)
        finally:
            self.__print_task.cancel()
            await asyncio.gather(self.__print_task, return_exceptions=True)
            await self.__hs.close()

# 虎牙直播：https://www.huya.com/lpl
# 斗鱼直播：https://www.douyu.com/9999
