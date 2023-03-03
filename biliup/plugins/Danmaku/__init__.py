# 部分弹幕功能代码来自项目：https://github.com/IsoaSFlus/danmaku，感谢大佬
# 快手弹幕代码来源及思路：https://github.com/py-wuhao/ks_barrage，感谢大佬
# 仅抓取用户弹幕，不包括入场提醒、礼物赠送等。

import asyncio
import os
import re
import ssl
import time
from functools import partial

import lxml.etree as etree
import aiofiles
import aiohttp

from biliup.plugins.Danmaku.douyu import Douyu
from biliup.plugins.Danmaku.huya import Huya
from biliup.plugins.Danmaku.bilibili import Bilibili

__all__ = ['DanmakuClient']


class DanmakuClient:
    def __init__(self, url, filename):
        self.__starttime = time.time()
        self.__filename = os.path.splitext(filename)[0] + '.xml'
        self.__filename_video_suffix = filename
        self.__url = ''
        self.__site = None
        self.__hs = None
        self.__ws = None
        self.__stop = False
        self.__dm_queue = None
        self.__link_status = True

        if 'http://' == url[:7] or 'https://' == url[:8]:
            self.__url = url
        else:
            self.__url = 'http://' + url
        for u, s in {'douyu.com': Douyu, 'huya.com': Huya, 'live.bilibili.com': Bilibili}.items():
            if re.match(r'^(?:http[s]?://)?.*?%s/(.+?)$' % u, url):
                self.__site = s
                self.__u = u
                break
        if self.__site is None:
            print('Invalid link!')
            exit()

    async def init_ws(self):
        ws_url, reg_datas = await self.__site.get_ws_info(self.__url)
        ctx = ssl.create_default_context()
        ctx.set_ciphers('DEFAULT')
        self.__ws = await self.__hs.ws_connect(ws_url, ssl_context=ctx)
        if reg_datas:
            for reg_data in reg_datas:
                await self.__ws.send_bytes(reg_data)
        if not os.path.exists(self.__filename):
            async with aiofiles.open(self.__filename, mode='w') as f:
                await f.write("<?xml version='1.0' encoding='UTF-8'?>\n"
                              "<i xmlns:ns0='http://www.w3.org/1999/XSL/Transform'>\n"
                              "</i>"
                              )

    async def heartbeats(self):
        while not self.__stop and self.__site.heartbeat:
            await asyncio.sleep(self.__site.heartbeatInterval)
            try:
                await self.__ws.send_bytes(self.__site.heartbeat)
            except Exception as Error:
                print(f"捕获到异常：{Error}")
        await self.__hs.close()

    async def fetch_danmaku(self):
        # while not self.__stop:
            await asyncio.sleep(1)
            await self.init_ws()
            await asyncio.sleep(1)
            async for msg in self.__ws:
                if self.__stop:
                    await self.__dm_queue.put(None)
                    return
                self.__link_status = True
                ms = self.__site.decode_msg(msg.data)
                for m in ms:
                    await self.__dm_queue.put(m)

    async def print_danmaku(self):
        parser = etree.XMLParser(recover=True)
        tree = etree.parse(self.__filename, parser=parser)
        root = tree.getroot()
        msg_i = 0
        msg_col = {'0': '16777215', '1': '16717077', '2': '2000880', '3': '8046667', '4': '16744192', '5': '10172916',
                   '6': '16738740'}

        def write_file(filename):
            with open(filename, "wb") as f:
                etree.indent(root, "\t")
                tree.write(f, encoding="UTF-8", xml_declaration=True, pretty_print=True)

        while not self.__stop:
            try:
                m = await self.__dm_queue.get()
                if m is None:
                    return
                # print(m)
                if m['msg_type'] == 'danmaku':
                    d = etree.SubElement(root, 'd')
                    if 'col' in m:
                        color = msg_col[m["col"]]
                    elif 'color' in m:
                        color = m["color"]
                    else:
                        color = '16777215'
                    msg_time = format(time.time() - self.__starttime, '.3f')
                    d.set('p', f"{msg_time},1,25,{color},0,0,0,0")
                    d.text = m["content"]

                    if msg_i >= 5:
                        loop = asyncio.get_running_loop()
                        await loop.run_in_executor(None, partial(write_file, self.__filename))
                        msg_i = 0
                    else:
                        msg_i = msg_i + 1

            except Exception as Error:
                print(f"捕获到异常：{Error}")

    async def start(self):
        self.__dm_queue = asyncio.Queue()
        self.__hs = aiohttp.ClientSession()
        await self.init_ws()
        await asyncio.gather(
            self.heartbeats(),
            self.fetch_danmaku(),
            self.print_danmaku(),
        )
        print("弹幕录制结束")

    def stop(self):
        self.__stop = True
        if not (os.path.exists(f"{self.__filename_video_suffix}.part") or
                os.path.exists(f"{self.__filename_video_suffix}")):
            os.remove(self.__filename)



# 虎牙直播：https://www.huya.com/lpl
# 斗鱼直播：https://www.douyu.com/9999