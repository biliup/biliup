#!/usr/bin/python3
# coding:utf8
import argparse
import asyncio
import logging.config
import platform
import shutil

import biliup.common.reload
from biliup.config import config
from biliup.database.db import SessionLocal, init
from . import __version__, LOG_CONF
from .common.Daemon import Daemon
from .common.reload import AutoReload
from .common.log import DebugLevelFilter


def arg_parser():
    daemon = Daemon('watch_process.pid', lambda: main(args))
    parser = argparse.ArgumentParser(description='Stream download and upload, not only for bilibili.')
    parser.add_argument('--version', action='version', version=f"v{__version__}")
    parser.add_argument('-H', help='web api host [default: 0.0.0.0]', dest='host')
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
    biliup.common.reload.program_args = args.__dict__
    # 初始化数据库
    with SessionLocal() as db:
        if init(args.no_http):
            try:
                config.load(args.config)
                config.save_to_db(db)
            except FileNotFoundError:
                print(f'新版本不依赖配置文件,请访问 WebUI 修改配置')
        config.load_from_db(db)
    # db.remove()
    LOG_CONF.update(config.get('LOGGING', {}))
    if args.verbose:
        LOG_CONF['loggers']['biliup']['level'] = args.verbose
        LOG_CONF['root']['level'] = args.verbose
    logging.config.dictConfig(LOG_CONF)
    logging.getLogger('httpx').addFilter(DebugLevelFilter())
    if platform.system() == 'Windows':
        return asyncio.run(main(args))
    args.func()


async def main(args):
    from .app import event_manager

    event_manager.start()

    # 启动时删除临时文件夹
    shutil.rmtree('./cache/temp', ignore_errors=True)

    interval = config.get('check_sourcecode', 15)

    if not args.no_http:
        import biliup.web
        runner = await biliup.web.service(args)
        detector = AutoReload(event_manager, runner.cleanup, interval=interval)
        biliup.common.reload.global_reloader = detector
        await detector.astart()
    else:
        import biliup.common.reload
        detector = AutoReload(event_manager, interval=interval)
        biliup.common.reload.global_reloader = detector
        await asyncio.gather(detector.astart())


class GracefulExit(SystemExit):
    code = 1


if __name__ == '__main__':
    arg_parser()
