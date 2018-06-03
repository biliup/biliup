#!/usr/bin/python3
from Engine import *
from eventdriven import *
from eventdriven.event import process


def get_queue(q):
    process.q = q


if __name__ == '__main__':
    from multiprocessing.pool import Pool
    from multiprocessing import Queue
    import sys

    sys.excepthook = work.new_hook

    queue = Queue()
    pool = Pool(3, initializer=get_queue, initargs=(queue,))

    # 初始化事件管理器
    eventManager = event.EventEngine(pool)

    # 批量注册事件
    eventType.Batch(eventManager, links_id).register()

    # 定时推送事件
    put = Putevent(eventManager, links_id, queue)
    put.timer(interval=40)

    # 关闭事件管理器
    eventManager.stop()
