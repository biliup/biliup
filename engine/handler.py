import multiprocessing
import time
import requests
import engine
import common
from engine import CHECK, BE_MODIFIED, DOWNLOAD_UPLOAD, TO_MODIFY, UPLOAD, urls, url_status, url_status_base
from engine.downloader import download, Extractor
from engine.upload import Upload
from common import logger
from common.event import Event

# 初始化事件管理器

event_manager = common.event.EventManager()


@event_manager.register(DOWNLOAD_UPLOAD, block=True)
def process(name, url, mod):
    try:
        now = common.time_now()
        if mod == 'dl':
            p = multiprocessing.Process(target=download, args=(name, url))
            p.start()
            p.join()
            # download(name, url)
            Upload(name).start(url, now)
        elif mod == 'up':
            Upload(name).start(url, now)
        else:
            return url
    finally:
        event = Event(BE_MODIFIED)
        event.args = (url,)
        # return url
        return event


@event_manager.server(urls, url_status, url_status_base)
class KernelFunc:
    def __init__(self, _urls, _url_status, _url_status_base):
        self.urls = _urls
        self.url_status = _url_status
        self.url_status_base = _url_status_base
        self.batches, self.onebyone = Extractor().sorted_checker(_urls)

    @event_manager.register(CHECK, block=True)
    def all_check(self):
        live = []
        try:
            for batch in self.batches:
                res = batch.check()
                if res:
                    live.extend(res)

            for one in self.onebyone:
                for url in one.url_list:

                    if one('检测' + url, url).check_stream():
                        live.append(url)

                    if url != one.url_list[-1]:
                        logger.debug('歇息会')
                        time.sleep(15)
        except requests.exceptions.ReadTimeout as timeout:
            logger.error("ReadTimeout:" + str(timeout))
        except requests.exceptions.SSLError as sslerr:
            logger.error("SSLError:" + str(sslerr))
        except requests.exceptions.ConnectTimeout as timeout:
            logger.error("ConnectTimeout:" + str(timeout))
        except requests.exceptions.ConnectionError as connerr:
            logger.error("ConnectionError:" + str(connerr))
        except requests.exceptions.ChunkedEncodingError as ceer:
            logger.error("ChunkedEncodingError:" + str(ceer))
        except requests.exceptions.RequestException:
            logger.exception("unknown")
        finally:
            event_t = Event(TO_MODIFY)
            event_t.args = (live,)
            event_u = Event(UPLOAD)
            event_u.args = (live,)
            return event_t, event_u

    @event_manager.register(engine.TO_MODIFY)
    def modify(self, live_m):
        live_d = {}
        # print(live_m)
        if live_m:
            event = []
            for live in live_m:
                if self.url_status[live] == 1:
                    # 已开播正在下载
                    # print('已开播正在下载')
                    pass
                else:
                    name = engine.find_name(live)
                    logger.debug(name + '刚刚开播，去下载')
                    event_d = Event(DOWNLOAD_UPLOAD)
                    event_d.args = (name, live, 'dl')
                    event.append(event_d)

                live_d[live] = 1
            self.url_status.update(live_d)
            # url_status = {**url_status_base, **live_d}
            return tuple(event)

        else:
            logger.debug('无人直播')

    def free(self, list_url):
        status_num = list(map(lambda x: self.url_status.get(x), list_url))
        # print(status_num)
        if 1 in status_num or 2 in status_num:
            return False
        else:
            return True

    @event_manager.register(engine.UPLOAD)
    def free_upload(self, _urls):
        logger.debug(_urls)
        event = []
        for title, v in engine.links_id.items():
            # names = list(map(find_name, urls))
            url = v[0]
            # if title not in names and url_status[url] == 0 and Upload(title, url).filter_file():
            if self.free(v) and Upload(title).filter_file():
                event_d = Event(DOWNLOAD_UPLOAD)
                event_d.args = (title, url, 'up')
                event.append(event_d)

                # self.event_manager.send_event(event_d)
                self.url_status[url] = 2
                # print('up')
        return tuple(event)
        # Upload(title, url).start()

        # except:
        #     logger.exception()
        # print('寻找结束')

    @event_manager.register(engine.BE_MODIFIED)
    def revise(self, url):
        if url:
            # url_status = {**url_status, **{url: 0}}
            self.url_status.update({url: 0})
            # print('更新字典')
            # print(url_status)
