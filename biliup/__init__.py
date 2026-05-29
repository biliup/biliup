import logging
import sys
from importlib.metadata import version

__version__ = version("biliup")


LOG_CONF = {
    'version': 1,
    'formatters': {
        'verbose': {
            'format': "%(asctime)s %(filename)s[line:%(lineno)d](Pid:%(process)d "
                      "Tname:%(threadName)s) %(levelname)s %(message)s",
            # 'datefmt': "%Y-%m-%d %H:%M:%S"
        },
        'simple': {
            'format': '%(asctime)s %(filename)s%(lineno)d[%(levelname)s]Tname:%(threadName)s %(message)s'
        },
    },
    'handlers': {
        'console': {
            'level': logging.DEBUG,
            'class': 'logging.StreamHandler',
            'stream': sys.stdout,
            'formatter': 'simple'
        },
        'file': {
            'level': logging.DEBUG,
            'class': 'logging.handlers.TimedRotatingFileHandler',
            'when': 'W0',
            'interval': 1,
            'backupCount': 1,
            'filename': 'ds_update.log',
            'formatter': 'verbose',
            'encoding': 'utf-8'
        }
    },
    'root': {
        'handlers': ['console'],
        'level': logging.INFO,
    },
    'loggers': {
        'biliup': {
            'handlers': ['file'],
            'level': logging.INFO,
        },
    }
}

IS_FROZEN = False
if getattr(sys, 'frozen', False) and hasattr(sys, '_MEIPASS'):
    import multiprocessing
    multiprocessing.freeze_support()
    IS_FROZEN = True
    print('running in a PyInstaller bundle')
