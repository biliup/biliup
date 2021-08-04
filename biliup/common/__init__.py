import logging
from datetime import datetime, timezone, timedelta
import sys


def time_now(fmt='%Y.%m.%d'):
    if fmt is None:
        fmt = '%Y.%m.%d'
    utc_dt = datetime.utcnow().replace(tzinfo=timezone.utc)
    bj_dt = utc_dt.astimezone(timezone(timedelta(hours=8)))
    # now = bj_dt.strftime('%Y{0}%m{1}%d{2}').format(*'...')
    now = bj_dt.strftime(fmt.encode('unicode-escape').decode())
    return now.encode().decode('unicode-escape')
# logging.SafeRotatingFileHandler = SafeRotatingFileHandler
# log_file_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'configlog.ini')
# logging.config.fileConfig(log_file_path)


def new_hook(t, v, tb):
    logging.getLogger('biliup').error("Uncaught exception:", exc_info=(t, v, tb))


sys.excepthook = new_hook
