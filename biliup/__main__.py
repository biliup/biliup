#!/usr/bin/python3
# coding:utf8
import argparse
import asyncio
import logging.config
import shutil

import stream_gears

import biliup.common.reload
# from biliup.config import config
from biliup import __version__, IS_FROZEN, LOG_CONF
from biliup.common.Daemon import Daemon
from biliup.common.log import DebugLevelFilter


def arg_parser():
    daemon = Daemon('watch_process.pid', lambda: main(args))
    parser = argparse.ArgumentParser(description='Stream download and upload, not only for bilibili.')
    parser.add_argument('--version', action='version', version=f"v{__version__}")
    parser.add_argument('-H', help='web api host [default: 0.0.0.0]', dest='host')
    if IS_FROZEN:
        parser.add_argument(
            '-P',
            help='web api port (REQUIRED)',
            dest='port',
            required=True,
            type=int
        )
    else:
        parser.add_argument('-P', help='web api port [default: 19159]', default=19159, dest='port')
    parser.add_argument('--no-http', action='store_true', help='disable web api')
    parser.add_argument('--static-dir', help='web static files directory for custom ui')
    parser.add_argument('--password', help='web ui password ,default username is biliup', dest='password')
    parser.add_argument('-v', '--verbose', action="store_const", const=logging.DEBUG, help="Increase output verbosity")
    parser.add_argument('--config', type=argparse.FileType(mode='rb'),
                        help='Location of the configuration file (default "./config.yaml")')
    parser.add_argument('--no-access-log', action='store_true', help='disable web access log')
    subparsers = parser.add_subparsers(help='Windows does not support this sub-command.')
    # create the parser for the "start" command
    parser_start = subparsers.add_parser('start', help='Run as a daemon process.')
    parser_start.set_defaults(func=daemon.start)
    parser_stop = subparsers.add_parser('stop', help='Stop daemon according to "watch_process.pid".')
    parser_stop.set_defaults(func=daemon.stop)
    parser_restart = subparsers.add_parser('restart')
    parser_restart.set_defaults(func=daemon.restart)
    parser.set_defaults(func=lambda: asyncio.run(main(args)))
    args = parser.parse_args()

    if args.verbose:
        LOG_CONF['loggers']['biliup']['level'] = args.verbose
        LOG_CONF['root']['level'] = args.verbose
    logging.config.dictConfig(LOG_CONF)
    logging.getLogger('httpx').addFilter(DebugLevelFilter())

    args.func()


async def main(args):
    # from biliup.app import event_manager

    # event_manager.start()

    # 启动时删除临时文件夹
    shutil.rmtree('./cache/temp', ignore_errors=True)
    from biliup.common.util import loop

    await loop.run_in_executor(None, stream_gears.main_loop)



class GracefulExit(SystemExit):
    code = 1


if __name__ == '__main__':
    arg_parser()
