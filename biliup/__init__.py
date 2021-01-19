import asyncio
import sys
from .engine import main
from .common.Daemon import Daemon


def _main():
    daemon = Daemon('watch_process.pid')
    if len(sys.argv) == 2:
        if 'start' == sys.argv[1]:
            daemon.start()
        elif 'stop' == sys.argv[1]:
            daemon.stop()
        elif 'restart' == sys.argv[1]:
            daemon.restart()
        else:
            print('unknown command')
            sys.exit(2)
        sys.exit(0)
    elif len(sys.argv) == 1:
        asyncio.run(main())
    else:
        print('usage: %s start|stop|restart' % sys.argv[0])
        sys.exit(2)
