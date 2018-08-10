import logging.config
import os


# def event_queue(_dict, queue):
#     for eventtype in _dict:
#         __event = common.event.Event(eventtype)
#         queue.put(__event)
#
#
# def put_event(event_manager, q):
#     _event = q.get()
#     event_manager.put(_event)


log_file_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'configlog.ini')
logging.config.fileConfig(log_file_path)
logger = logging.getLogger('log01')
