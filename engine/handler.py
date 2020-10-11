import multiprocessing
import engine
import common
from engine import *
from engine.downloader import download, check
from common import logger
from common.event import Event
from engine.plugins.base_adapter import UploadBase
from engine.uploader import upload

# 初始化事件管理器
event_manager = common.event.EventManager()


@event_manager.register(DOWNLOAD, block=True)
def process(name, url):
    try:
        data = {"url": url, "date": common.time_now()}
        p = multiprocessing.Process(target=download, args=(name, url))
        p.start()
        p.join()
        # download(name, url)
        upload("bilibili", name, data)
    finally:
        return Event(BE_MODIFIED, args=(url,))


@event_manager.register(UPLOAD, block=True)
def process_upload(name, url):
    try:
        data = {"url": url, "date": common.time_now()}
        upload("bilibili", name, data)
    finally:
        return Event(BE_MODIFIED, args=(url,))


@event_manager.server(urls, url_status, url_status_base)
class KernelFunc:
    def __init__(self, _urls, _url_status, _url_status_base):
        self.urls = _urls
        self.url_status = _url_status
        self.url_status_base = _url_status_base

    @event_manager.register(CHECK, block=True)
    def batch_check(self):
        live = check(self.urls, "batch")
        return Event(CHECK_UPLOAD, args=(live,)), Event(TO_MODIFY, args=(live,))

    @event_manager.register(CHECK, block=True)
    def singleton_check(self):
        live = check(self.urls, "single")
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
                name = engine.find_name(live)
                logger.debug(f'{name}刚刚开播，去下载')
                event_manager.send_event(Event(DOWNLOAD, args=(name, live)))

            live_d[live] = 1
        self.url_status.update(live_d)
        # url_status = {**url_status_base, **live_d}

    def free(self, list_url):
        status_num = list(map(lambda x: self.url_status.get(x), list_url))
        # if 1 in status_num or 2 in status_num:
        #     return False
        # else:
        #     return True
        return not (1 in status_num or 2 in status_num)

    @event_manager.register(CHECK_UPLOAD)
    def free_upload(self, _urls):
        logger.debug(_urls)
        for title, v in engine.links_id.items():
            url = v[0]
            if self.free(v) and UploadBase.filter_file(title):
                event_manager.send_event(Event(UPLOAD, args=(title, url)))
                self.url_status[url] = 2

    @event_manager.register(BE_MODIFIED)
    def revise(self, url):
        if url:
            # 更新字典
            # url_status = {**url_status, **{url: 0}}
            self.url_status.update({url: 0})
