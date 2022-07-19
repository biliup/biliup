#!/usr/bin/python3
# coding:utf8
import argparse
import asyncio
import logging.config
import platform
import stream_gears

from . import __version__, LOG_CONF
from .common.Daemon import Daemon
from .common.reload import AutoReload
import biliup.common.reload
from .common.timer import Timer
from biliup.config import config
from .engine.event import Event


def arg_parser():
    daemon = Daemon('watch_process.pid', lambda: main(args))
    parser = argparse.ArgumentParser(description='Stream download and upload, not only for bilibili.')
    parser.add_argument('--version', action='version', version=f"v{__version__}")
    parser.add_argument('-H', help='web api host [default: 0.0.0.0]', dest='host')
    parser.add_argument('-P', help='web api port [default: 19159]', default=19159, dest='port')
    parser.add_argument('--http', action='store_true', help='enable web api')
    parser.add_argument('--static-dir', help='web static files directory for custom ui')
    parser.add_argument('--password', help='web ui password ,default username is biliup', dest='password')
    parser.add_argument('-v', '--verbose', action="store_const", const=logging.DEBUG, help="Increase output verbosity")
    parser.add_argument('--config', type=argparse.FileType(mode='rb'),
                        help='Location of the configuration file (default "./config.yaml")')
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
    if args.http:
        config.create_without_config_input(args.config)
    else:
        config.load(args.config)
    LOG_CONF.update(config.get('LOGGING', {}))
    if args.verbose:
        LOG_CONF['loggers']['biliup']['level'] = args.verbose
        LOG_CONF['root']['level'] = args.verbose
    logging.config.dictConfig(LOG_CONF)
    if platform.system() == 'Windows':
        return asyncio.run(main(args))
    args.func()


async def main(args):
    from .handler import CHECK_UPLOAD, CHECK, event_manager

    event_manager.start()

    async def check_timer():
        event_manager.send_event(Event(CHECK_UPLOAD))
        for k in event_manager.context['checker'].keys():
            event_manager.send_event(Event(CHECK, (k,)))

    wait = config.get('event_loop_interval') if config.get('event_loop_interval') else 40
    # 初始化定时器
    timer = Timer(func=check_timer, interval=wait)

    interval = config.get('check_sourcecode') if config.get('check_sourcecode') else 15
    if args.http:
        import biliup.web
        runner, site = await biliup.web.service(args, event_manager)
        detector = AutoReload(event_manager, timer, runner.cleanup, interval=interval)
        biliup.common.reload.global_reloader = detector
        await asyncio.gather(detector.astart(), timer.astart(), site.start(), return_exceptions=True)
    else:
        # 模块更新自动重启
        detector = AutoReload(event_manager, timer, interval=interval)
        await asyncio.gather(detector.astart(), timer.astart(), return_exceptions=True)


if __name__ == '__main__':
    arg_parser()
