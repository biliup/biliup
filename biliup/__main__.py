#!/usr/bin/python3
# coding:utf8
import platform
import sys
import asyncio

from .common.Daemon import Daemon
from . import main, __version__


def _main():
    daemon = Daemon('watch_process.pid', main)
    if platform.system() != 'Windows' and len(sys.argv) == 2:
        if 'start' == sys.argv[1]:
            daemon.start()
        elif 'stop' == sys.argv[1]:
            daemon.stop()
        elif 'restart' == sys.argv[1]:
            daemon.restart()
        elif '--version' == sys.argv[1]:
            print(__version__)
        else:
            print('unknown command')
            sys.exit(2)
        sys.exit(0)
    elif platform.system() == 'Windows' or len(sys.argv) == 1:
        asyncio.run(main())
    else:
        print('usage: %s start|stop|restart' % sys.argv[0])
        sys.exit(2)


if __name__ == '__main__':
    _main()
