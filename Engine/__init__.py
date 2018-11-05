from functools import partial
from multiprocessing.pool import Pool

import yaml

import common
from Engine.downloader import Extractor
from Engine.kernel import CallBack, modify, process, revise, free_upload, \
    callback2, all_check
from common.DesignPattern import Service
from common.event import Event
from common.reload import autoreload
from common.timer import Timer

CHECK = 'check'
TO_MODIFY = 'to_modify'
DOWNLOAD_UPLOAD = 'download_upload'
BE_MODIFIED = 'be_modified'
UPLOAD = 'upload'

# print(Urls, url_status, url_status_base)

# def get_queue(q):
#     process.q = q
with open(r'config.yaml', encoding='utf-8') as stream:
    config = yaml.load(stream)
    links_id = config['links_id']
    user_name = config['user_name']
    pass_word = config['pass_word']
    chromedrive_path = config['chromedrive_path']


def getmany():
    urls = []
    urlstatus = {}
    for k, v in links_id.items():
        urls += v
        for url in v:
            urlstatus[url] = 0
    return urls, urlstatus, urlstatus.copy()


Urls, url_status, url_status_base = getmany()
batches, onebyone = Extractor().sorted_checker(Urls)


def main():
    # pool = Pool(3, initializer=get_queue, initargs=(queue,))
    pool = Pool(3)
    service_p = partial(Service, pool)

    # 初始化事件管理器
    event_manager = common.event.EventManager()

    # 初始化定时器
    timer_ = Timer(func=event_manager.send_event, args=(Event(CHECK),), interval=40)

    # 模块更新自动重启
    autoreload(event_manager, timer_, interval=15)

    # 监听器
    # check = Event(CHECK)
    # ty = Event(TO_MODIFY)
    # dd = Event(DOWNLOAD_UPLOAD)
    # bd = Event(BE_MODIFIED)
    # ud = Event(UPLOAD)
    callback_2 = partial(callback2, event_manager)

    service_check = service_p(all_check, callback_2)
    modify_p = partial(modify, event_manager)
    service_download = service_p(process, CallBack(event_manager, Event(BE_MODIFIED)).send)
    # service_upload = service_p(free_upload, None)
    upload_p = partial(free_upload, event_manager)

    # 批量注册事件
    # Engine.downloader.Batch(event_manager, links_id).register()

    # 绑定事件和监听器响应函数
    event_manager.add_event_listener(CHECK, service_check.start)
    event_manager.add_event_listener(TO_MODIFY, modify_p)
    event_manager.add_event_listener(DOWNLOAD_UPLOAD, service_download.start)
    event_manager.add_event_listener(BE_MODIFIED, revise)
    event_manager.add_event_listener(UPLOAD, upload_p)

    event_manager.start()
    timer_.start()

    # 添加事件队列
    # event_queue(links_id, queue)
    # 定时推送事件


__all__ = ['downloader', 'upload', 'plugins', 'main', 'links_id']
