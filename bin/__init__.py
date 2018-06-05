import os
from bin.AutoUpload import autoreload
from bin.Engine import links_id
from multiprocessing.pool import Pool
from multiprocessing import Queue
from bin.eventdriven.event import process
from bin.eventdriven import Timer, event, event_queue, put_event
from bin.eventdriven import eventType
import logging.config
log_file_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'configlog.ini')
logging.config.fileConfig(log_file_path)


def get_queue(q):
    process.q = q


def main():

    queue = Queue()
    pool = Pool(3, initializer=get_queue, initargs=(queue,))

    # 初始化事件管理器
    event_manager = event.EventEngine(pool)

    # 批量注册事件
    eventType.Batch(event_manager, links_id).register()

    # 添加事件队列
    event_queue(links_id, queue)
    # 定时推送事件
    timer = Timer(func=put_event, args=(event_manager, queue), interval=40)

    # 模块更新自动重启
    autoreload(event_manager, timer)

    event_manager.start()
    timer.start()

