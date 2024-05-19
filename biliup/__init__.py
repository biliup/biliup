import logging
import platform
import sys

__version__ = "0.4.63"

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
            'class': 'biliup.common.log.SafeRotatingFileHandler',
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

if (3, 10, 6) > sys.version_info >= (3, 8) and platform.system() == 'Windows':
    # fix 'Event loop is closed' RuntimeError in Windows
    from asyncio import proactor_events
    from biliup.common.tools import silence_event_loop_closed

    proactor_events._ProactorBasePipeTransport.__del__ = silence_event_loop_closed(
        proactor_events._ProactorBasePipeTransport.__del__)
