#!/usr/bin/python3
import sys
from bin import main
from bin.Daemon import Daemon
from bin.Engine import work
if __name__ == '__main__':
    sys.excepthook = work.new_hook

    daemon = Daemon('/home/chromeuser/bilibiliupload/watch_process.pid')
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
        main()
    else:
        print('usage: %s start|stop|restart' % sys.argv[0])
        sys.exit(2)
