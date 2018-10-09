import time
import requests
import yaml

import Engine
from Engine.downloader import Extractor, download
from Engine.upload import Upload
from common import logger
from common.event import Event

with open(r'config.yaml', encoding='utf-8') as stream:
    config = yaml.load(stream)
    links_id = config['links_id']
    user_name = config['user_name']
    pass_word = config['pass_word']
    chromedrive_path = config['chromedrive_path']


class CheckAll(object):
    def __init__(self, urls):
        self.urls = urls

    def check(self):
        live = []
        batches, onebyone = Extractor().sorted_checker(self.urls)
        for batch in batches:
            try:
                res = batch.check()
                if res:
                    live += res
            except requests.exceptions.SSLError as sslerr:
                logger.info(batch.__class__.__module__+',requests.exceptions.SSLError:'+str(sslerr))
            except requests.exceptions.ConnectionError as connerr:
                # logger.exception(batch.__class__.__module__)
                logger.info(batch.__class__.__module__ + ',requests.exceptions.SSLError:' + str(connerr))

            # except:
            #     logger.exception()

        for one in onebyone:
            for url in one.__plugin__.url_list:
                if one.__plugin__('检测' + url, url).check_stream():
                    live += [url]
                if url != one.__plugin__.url_list[-1]:
                    print('歇息会')
                    time.sleep(15)
        return live


# 用来包装需要多进程的函数（多进程执行避免主进程阻塞）
class Service:
    def __init__(self, pool, func, callback):
        # 事件处理进程池
        self.__pool = pool
        self.__func = func
        self.callback = callback

    def __run(self, args):
        self.__pool.apply_async(func=self.__func, args=args, callback=self.callback)
        # self.__pool.apply(func=self.__func, args=args)

    def start(self, event):
        args = event.args
        self.__run(args)

    def stop(self):
        self.__pool.close()
        self.__pool.join()


class CallBack(object):
    def __init__(self, event_manager, event):
        self.event_manager = event_manager
        self.event = event

    def send(self, result):
        # if result:
        self.event.args = (result,)
        self.event_manager.send_event(self.event)


def find_name(url):
    for name in links_id:
        if url in links_id[name]:
            return name


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
            if url_status[live] == 1:
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
        url_status.update(live_d)
        # url_status = {**url_status_base, **live_d}
    else:
        print('无人直播')


def process(name, url, mod):
    try:
        now = Engine.work.time_now()
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
    status_num = list(map(lambda x: url_status.get(x), list_url))
    # print(status_num)
    if 1 in status_num or 2 in status_num:
        return False
    else:
        return True


def free_upload(event_manager, urls):
    # urls, = urls.args
    # print(urlstatus)
    # try:
    # print(urls)
    for title, v in links_id.items():
        # names = list(map(find_name, urls))
        url = v[0]
        # if title not in names and url_status[url] == 0 and Upload(title, url).filter_file():
        if free(v) and Upload(title).filter_file():
            event_d = Event('download_upload')
            event_d.args = (title, url, 'up')

            event_manager.send_event(event_d)
            url_status[url] = 2
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
        url_status.update({url: 0})
        # print('更新字典')
        # print(url_status)


def getmany():
    urls = []
    urlstatus = {}
    for k, v in links_id.items():
        urls += v
        for url in v:
            urlstatus[url] = 0
    return urls, urlstatus, urlstatus.copy()


Urls, url_status, url_status_base = getmany()

if __name__ == '__main__':
    print(list(map(lambda x: x, [])))
