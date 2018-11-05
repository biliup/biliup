import time
import requests
import Engine
import common
from Engine.downloader import download
from Engine.upload import Upload
from common import logger
from common.event import Event


def all_check():
    live = []
    for batch in Engine.batches:
        try:
            res = batch.check()
            if res:
                live += res
        except requests.exceptions.ReadTimeout as timeout:
            logger.error(batch.__class__.__module__ + ',ReadTimeout:' + str(timeout))
        except requests.exceptions.SSLError as sslerr:
            logger.error(batch.__class__.__module__ + ',SSLError:' + str(sslerr))
        except requests.exceptions.ConnectionError as connerr:
            logger.error(batch.__class__.__module__ + ',ConnectionError:' + str(connerr))
        except requests.exceptions.RequestException:
            logger.exception(batch.__class__.__module__)

    for one in Engine.onebyone:
        for url in one.__plugin__.url_list:
            try:
                if one.__plugin__('检测' + url, url).check_stream():
                    live += [url]
            except requests.exceptions.ReadTimeout as timeout:
                logger.error(one.__plugin__.__module__ + ',ReadTimeout:' + str(timeout))
            except requests.exceptions.ConnectTimeout as timeout:
                logger.error(one.__plugin__.__module__ + ',ConnectTimeout:' + str(timeout))
            except requests.exceptions.ConnectionError as connerr:
                logger.error(one.__plugin__.__module__ + ',ConnectionError:' + str(connerr))
            except requests.exceptions.ChunkedEncodingError as ceer:
                logger.error(one.__plugin__.__module__ + ',ChunkedEncodingError:' + str(ceer))
            except requests.exceptions.RequestException:
                logger.exception(one.__plugin__.__module__)

            if url != one.__plugin__.url_list[-1]:
                logger.debug('歇息会')
                time.sleep(15)
    return live


class CallBack(object):
    def __init__(self, event_manager, event):
        self.event_manager = event_manager
        self.event = event

    def send(self, result):
        # if result:
        self.event.args = (result,)
        self.event_manager.send_event(self.event)


def callback2(event_manager, result):
    CallBack(event_manager, Event('to_modify')).send(result)
    CallBack(event_manager, Event('upload')).send(result)


def modify(event_manager, live_m):
    live_m, = live_m.args
    live_d = {}
    # global url_status
    # print(live_m)
    if live_m:
        for live in live_m:
            # print(live)
            if Engine.url_status[live] == 1:
                pass
                # print('已开播正在下载')
            else:
                # print('刚刚开播，去下载')

                name = find_name(live)

                event_d = Event('download_upload')
                event_d.args = (name, live, 'dl')

                event_manager.send_event(event_d)
                # self.__run(live)
            live_d[live] = 1
        Engine.url_status.update(live_d)
        # url_status = {**url_status_base, **live_d}
    else:
        logger.debug('无人直播')


def process(name, url, mod):
    try:
        now = common.time_now()
        if mod == 'dl':
            download(name, url)
            Upload(name).start(url, now)
        elif mod == 'up':
            Upload(name).start(url, now)
        else:
            return url
    finally:
        return url


def free(list_url):
    status_num = list(map(lambda x: Engine.url_status.get(x), list_url))
    # print(status_num)
    if 1 in status_num or 2 in status_num:
        return False
    else:
        return True


def free_upload(event_manager, urls):
    # urls, = urls.args
    # print(urlstatus)
    # try:
    logger.debug(urls)
    for title, v in Engine.links_id.items():
        # names = list(map(find_name, urls))
        url = v[0]
        # if title not in names and url_status[url] == 0 and Upload(title, url).filter_file():
        if free(v) and Upload(title).filter_file():
            event_d = Event('download_upload')
            event_d.args = (title, url, 'up')

            event_manager.send_event(event_d)
            Engine.url_status[url] = 2
            # print('up')

            # Upload(title, url).start()

    # except:
    #     logger.exception()
    # print('寻找结束')


def revise(url):
    # if type(url) is dict:
    #     url = url['result']
    url, = url.args
    if url:
        # global url_status
        # url_status = {**url_status, **{url: 0}}
        Engine.url_status.update({url: 0})
        # print('更新字典')
        # print(url_status)


def find_name(url):
    for name in Engine.links_id:
        if url in Engine.links_id[name]:
            return name
