# 部分弹幕功能代码来自项目：https://github.com/IsoaSFlus/danmaku，感谢大佬
# 快手弹幕代码来源及思路：https://github.com/py-wuhao/ks_barrage，感谢大佬
# 仅抓取用户弹幕，不包括入场提醒、礼物赠送等。

import asyncio
import queue
import threading
import time
from concurrent.futures import ThreadPoolExecutor

from biliup.plugins.Danmaku import src

# stop_queue = asyncio.Queue()
# url_o = ''
# filename = ''


# def thread_danmaku(loop):
#     asyncio.set_event_loop(loop)
#     dmc = src.DanmakuClient(url_o, stop_queue, filename)
#     loop.run_until_complete(dmc.start())
#
#
# async def danmaku_main(url):
#     global url_o
#     url_o = url
#     executor1 = ThreadPoolExecutor(max_workers=1)
#     loop = asyncio.new_event_loop()
#     loop.run_in_executor(executor1, thread_danmaku, loop)
#     input("输入回车停止运行：\n")
#     await stop_queue.put(True)


class Danmaku(threading.Thread):
    def __init__(self, filename, url):
        threading.Thread.__init__(self)
        self.stop_queue = asyncio.Queue()
        self.url_ = url
        self.filename = f"{time.strftime(filename)}.xml"

    def run(self):
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        dmc = src.DanmakuClient(self.url_, self.filename, self.stop_queue)
        loop.create_task(dmc.start())
        loop.run_forever()

    async def stop(self):
        await self.stop_queue.put(True)


# 虎牙直播：https://www.huya.com/lpl
# 斗鱼直播：https://www.douyu.com/9999
