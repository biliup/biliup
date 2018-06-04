import os

from bin.Engine import links_id
from multiprocessing.pool import Pool
from multiprocessing import Queue
from bin.eventdriven.event import process
from bin.eventdriven import Putevent, event
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

    # 定时推送事件
    put = Putevent(event_manager, links_id, queue)
    put.timer(interval=40)

    # 关闭事件管理器
    # event_manager.stop()
