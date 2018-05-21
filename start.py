from Engine import *
from eventdriven import *

if __name__ == '__main__':
    from threading import Thread
    from multiprocessing import Manager, Process
    import sys

    sys.excepthook = work.new_hook
    # 进程通信
    manager = Manager()
    d = manager.dict(links_id)
    queue = manager.Queue()

    # 监控
    t = Thread(target=work.monitoring, args=(queue,))
    t.start()

    eventManager = event.EventEngine()
    # 注册事件
    evr = eventType.RegisterEvent(eventManager, d)
    evr.creator(queue)

    # 定时添加事件
    timer = Putevent(eventManager, d)
    timer.put()
    eventManager.stop()
    print('引擎停止')
