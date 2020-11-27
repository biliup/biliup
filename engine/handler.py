import common
import engine
from common import logger
from common.event import Event
from engine import *
from engine.downloader import download, check_url
from engine.plugins.upload import UploadBase
from engine.uploader import upload

# 初始化事件管理器
event_manager = common.event.EventManager(context)


@event_manager.register(DOWNLOAD, block=True)
def process(name, url):
    date = common.time_now()
    try:
        # p = multiprocessing.Process(target=download, args=(name, url))
        # p.start()
        # p.join()
        download(name, url)
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
        for url in check_url(plugin):
            yield Event(TO_MODIFY, args=(url,))

    @event_manager.register(TO_MODIFY)
    def modify(self, url):
        if not url:
            return logger.debug('无人直播')
        if self.url_status[url] == 1:
            return logger.debug('已开播正在下载')
        if self.url_status[url] == 2:
            return logger.debug('正在上传稍后下载')
        name = inverted_index[url]
        logger.debug(f'{name}刚刚开播，去下载')
        self.url_status[url] = 1
        return Event(DOWNLOAD, args=(name, url))

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
