from Engine import *
from eventdriven import *

if __name__ == '__main__':
    from multiprocessing import Manager, Process
    import sys

    sys.excepthook = work.new_hook
    # 进程通信
    manager = Manager()
    d = manager.dict(links_id)
    queue = manager.Queue()

    # 监控
    p0 = Process(target=work.monitoring, args=(queue,))
    p0.start()

    eventManager = event.EventEngine()
    # 注册事件
    evr = eventType.RegisterEvent(eventManager, d)
    evr.creator(queue)

    # 定时添加事件
    timer = Putevent(eventManager, d)
    timer.put()
    eventManager.stop()
    print('引擎停止')
