#!/usr/bin/python3
from Engine import *
from eventdriven import *

if __name__ == '__main__':
    from threading import Thread
    from multiprocessing import Manager
    import sys

    sys.excepthook = work.new_hook
    # 进程通信
    manager = Manager()
    d = manager.dict(links_id)
    queue = manager.Queue()

    # 监控文件、进程
    t = Thread(target=work.monitoring, args=(queue,))
    t.start()

    # 初始化事件管理器
    eventManager = event.EventEngine()

    # 批量注册事件
    eventType.Batch(eventManager, d, queue).register()

    # 定时推送事件
    put = Putevent(eventManager, d)
    put.timer(interval=40)

    # 关闭事件管理器
    eventManager.stop()

