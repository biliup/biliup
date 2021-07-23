#!/usr/bin/python3
# coding:utf8
import argparse
import logging.config
import platform
import asyncio
import sys

from biliup import config
from .common.Daemon import Daemon
from . import main, __version__

LOG_CONF = {
    'version': 1,
    'formatters': {
        'verbose': {
            'format': "%(asctime)s %(filename)s[line:%(lineno)d](Pid:%(process)d "
                      "Tname:%(threadName)s) %(levelname)s %(message)s",
            # 'datefmt': "%Y-%m-%d %H:%M:%S"
        },
        'simple': {
            'format': '%(filename)s%(lineno)d[%(levelname)s]Tname:%(threadName)s %(message)s'
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
            'formatter': 'verbose'
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


def arg_parser():
    daemon = Daemon('watch_process.pid', main)
    parser = argparse.ArgumentParser(description='Stream download and upload, not only for bilibili.')
    parser.add_argument('--version', action='version', version=f"v{__version__}")
    parser.add_argument('-v', '--verbose', action="store_const", const=logging.DEBUG, help="Increase output verbosity")
    parser.add_argument('--config', type=argparse.FileType(encoding='UTF-8'),
                        help='Location of the configuration file (default "./config.yaml")')
    subparsers = parser.add_subparsers(help='Windows does not support this sub-command.')
    # create the parser for the "start" command
    parser_start = subparsers.add_parser('start', help='Run as a daemon process.')
    parser_start.set_defaults(func=daemon.start)
    parser_stop = subparsers.add_parser('stop', help='Stop daemon according to "watch_process.pid".')
    parser_stop.set_defaults(func=daemon.stop)
    parser_restart = subparsers.add_parser('restart')
    parser_restart.set_defaults(func=daemon.restart)
    parser.set_defaults(func=lambda: asyncio.run(main()))
    args = parser.parse_args()
    config.load(args.config)
    LOG_CONF.update(config.get('LOGGING', {}))
    if args.verbose:
        LOG_CONF['loggers']['biliup']['level'] = args.verbose
        LOG_CONF['root']['level'] = args.verbose
    logging.config.dictConfig(LOG_CONF)
    if platform.system() == 'Windows':
        return asyncio.run(main())
    args.func()


if __name__ == '__main__':
    arg_parser()
