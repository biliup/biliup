import multiprocessing
import engine
import common
from engine import *
from engine.downloader import download, batch_check, singleton_check
from common import logger
from common.event import Event
from engine.plugins import BatchCheckBase
from engine.plugins.base_adapter import UploadBase
from engine.uploader import upload

# 初始化事件管理器
event_manager = common.event.EventManager(context)


@event_manager.register(DOWNLOAD, block=True)
def process(name, url):
    date = common.time_now()
    try:
        p = multiprocessing.Process(target=download, args=(name, url))
        p.start()
        p.join()
        # download(name, url)
    finally:
        return Event(UPLOAD, (name, url, date))


@event_manager.register(UPLOAD, block=True)
def process_upload(name, url, date):
    yield Event(BE_MODIFIED, (url, 2))
    try:
        data = {"url": url, "date": date}
        upload("bili_web", name, data)
    finally:
        yield Event(BE_MODIFIED, args=(url, 0))


@event_manager.server()
class KernelFunc:
    def __init__(self, urls, url_status: dict):
        self.urls = urls
        self.url_status = url_status
        self.__raw_streamer_status = url_status.copy()

    @event_manager.register(CHECK, block=True)
    def singleton_check(self, platform):
        plugin = checker[platform]
        if isinstance(plugin, BatchCheckBase):
            live = batch_check(plugin)
        else:
            live = singleton_check(plugin)
        return Event(TO_MODIFY, args=(live,))

    @event_manager.register(TO_MODIFY)
    def modify(self, live_m):
        if not live_m:
            return logger.debug('无人直播')
        live_d = {}
        for live in live_m:
            if self.url_status[live] == 1:
                logger.debug('已开播正在下载')
            else:
                name = inverted_index[live]
                logger.debug(f'{name}刚刚开播，去下载')
                yield Event(DOWNLOAD, args=(name, live))

            live_d[live] = 1
        self.url_status.update(live_d)
        # self.url_status = {**self.__raw_streamer_status, **live_d}

    def free(self, list_url):
        status_num = list(map(lambda x: self.url_status.get(x), list_url))
        return not (1 in status_num or 2 in status_num)

    @event_manager.register(CHECK_UPLOAD)
    def free_upload(self):
        for title, urls in engine.streamer_url.items():
            if self.free(urls) and UploadBase.filter_file(title):
                yield Event(UPLOAD, args=(title, urls[0], common.time_now()))

    @event_manager.register(BE_MODIFIED)
    def revise(self, url, status):
        if url:
            # 更新字典
            # url_status = {**url_status, **{url: 0}}
            self.url_status.update({url: status})
