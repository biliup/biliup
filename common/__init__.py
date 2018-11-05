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
from datetime import datetime, timezone, timedelta

log_file_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'configlog.ini')
logging.config.fileConfig(log_file_path)
logger = logging.getLogger('log01')


def time_now():
    utc_dt = datetime.utcnow().replace(tzinfo=timezone.utc)
    bj_dt = utc_dt.astimezone(timezone(timedelta(hours=8)))
    # now = bj_dt.strftime('%Y{0}%m{1}%d{2}').format(*'...')
    now = bj_dt.strftime('%Y{0}%m{1}%d').format(*'..')
    return now


def new_hook(t, v, tb):
    logger.error("Uncaught exception：", exc_info=(t, v, tb))