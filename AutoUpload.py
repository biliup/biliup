#!/usr/bin/python3
import sys, os, time, atexit, subprocess, logging, psutil
from signal import SIGTERM
import Bilibili

logger = logging.getLogger('log01')


def has_extension(fname_list, *extension):
    array = []
    for fname in fname_list:
        result = list(map(fname.endswith, extension))
        if True in result:
            array.append(True)
        else:
            array.append(False)
    if True in array:
        return True
    return False

def _iter_module_files():
    """Iterator to module's source filename of sys.modules (built-in
    excluded).
    """
    for module in list(sys.modules.values()):
        filename = getattr(module, '__file__', None)
        if filename:
            if filename[-4:] in ('.pyo', '.pyc'):
                filename = filename[:-1]
            yield filename


def get_p_children(pid, _recursive=True):
    try:
        parent = psutil.Process(pid)
    except psutil.NoSuchProcess:
        return None
    children = parent.children(recursive=_recursive)
    return children


class Autoreload(object):
    def __init__(self, _process):
        self.p = _process  # 被监控子进程

    @staticmethod
    def _is_any_file_changed(mtimes):
        """Return 1 if there is any source file of sys.modules changed,
        otherwise 0. mtimes is dict to store the last modify time for
        comparing."""
        for filename in _iter_module_files():
            try:
                mtime = os.stat(filename).st_mtime
            except IOError:
                continue
            old_time = mtimes.get(filename, None)
            if old_time is None:
                mtimes[filename] = mtime
            elif mtime > old_time:
                logger.info('模块已更新')
                return 1
        return 0

    def _restart_subp(self, interval=10):
        while True:
            time.sleep(interval)
            if self._work_free():
                # logger.info('重启进程')
                pid = self.p.pid
                children = get_p_children(pid)

                os.kill(pid, SIGTERM)

                for process in children:
                    # print(process)
                    process.terminate()
                # os.killpg(os.getpgid(pid), SIGTERM)
                return

    def start_change_detector(self, interval=10):
        """Check file state ervry interval. If any change is detected, exit this
        process with a special code, so that deamon will to restart a new process.
        """
        mtimes = {}
        while 1:
            if Autoreload._is_any_file_changed(mtimes):
                self._restart_subp()
                return
            time.sleep(interval)

    def _work_free(self):
        # wp = psutil.Process(self.p.pid)
        # more_children = wp.children(recursive=True)
        # children = wp.children()
        # if len(more_children) == len(children):
        #     logger.info('进程空闲')
        #     return True
        # return False

        fname_list = os.listdir('.')
        if has_extension(fname_list, '.mp4', '.part', '.flv'):
            return False
        return True


# python模拟linux的守护进程
class Daemon(object):
    def __init__(self, pidfile, change_currentdirectory=False, stdin='/dev/null', stdout='/dev/null',
                 stderr='/dev/null'):
        # 需要获取调试信息，改为stdin='/dev/stdin', stdout='/dev/stdout', stderr='/dev/stderr'，以root身份运行。
        self.stdin = stdin
        self.stdout = stdout
        self.stderr = stderr
        self.pidfile = pidfile
        self.cd = change_currentdirectory

    def _daemonize(self):
        try:
            pid = os.fork()  # 第一次fork，生成子进程，脱离父进程
            if pid > 0:
                sys.exit(0)  # 退出主进程
        except OSError as e:
            sys.stderr.write('fork #1 failed: %d (%s)\n' % (e.errno, e.strerror))
            sys.exit(1)

        if self.cd:
            os.chdir("/")  # 修改工作目录
        os.setsid()  # 设置新的会话连接
        os.umask(0)  # 重新设置文件创建权限

        try:
            pid = os.fork()  # 第二次fork，禁止进程打开终端
            if pid > 0:
                sys.exit(0)
        except OSError as e:
            sys.stderr.write('fork #2 failed: %d (%s)\n' % (e.errno, e.strerror))
            sys.exit(1)

        # 重定向文件描述符
        sys.stdout.flush()
        sys.stderr.flush()
        # with open(self.stdin, 'r') as si, open(self.stdout, 'a+') as so, open(self.stderr, 'ab+', 0) as se:
        si = open(self.stdin, 'r')
        so = open(self.stdout, 'a+')
        se = open(self.stderr, 'ab+', 0)
        os.dup2(si.fileno(), sys.stdin.fileno())
        os.dup2(so.fileno(), sys.stdout.fileno())
        os.dup2(se.fileno(), sys.stderr.fileno())

        # 注册退出函数，根据文件pid判断是否存在进程
        atexit.register(self.delpid)
        pid = str(os.getpid())
        with open(self.pidfile, 'w+') as f:
            f.write('%s\n' % pid)
            # file(self.pidfile, 'w+').write('%s\n' % pid)

    def delpid(self):
        os.remove(self.pidfile)

    def start(self):
        # 检查pid文件是否存在以探测是否存在进程
        try:
            pf = open(self.pidfile, 'r')
            pid = int(pf.read().strip())
            pf.close()
        except IOError:
            pid = None

        if pid:
            message = 'pidfile %s already exist. Daemon already running!\n'
            sys.stderr.write(message % self.pidfile)
            sys.exit(1)

        # 启动监控
        self._daemonize()
        self._run()

    def stop(self):
        # 从pid文件中获取pid
        try:
            pf = open(self.pidfile, 'r')
            pid = int(pf.read().strip())
            pf.close()
        except IOError:
            pid = None

        if not pid:  # 重启不报错
            message = 'pidfile %s does not exist. Daemon not running!\n'
            sys.stderr.write(message % self.pidfile)
            return

        # 杀进程
        try:
            while 1:
                os.killpg(os.getpgid(pid), SIGTERM)
                time.sleep(0.1)
                # os.system('hadoop-daemon.sh stop datanode')
                # os.system('hadoop-daemon.sh stop tasktracker')
                # os.remove(self.pidfile)
        except OSError as err:
            err = str(err)
            if err.find('No such process') > 0:
                if os.path.exists(self.pidfile):
                    os.remove(self.pidfile)
            else:
                print(str(err))
                sys.exit(1)

    def restart(self):
        self.stop()
        self.start()

    def _run(self):
        """ run your fun"""
        while True:
            args = [os.path.abspath(Bilibili.__file__)]
            p = subprocess.Popen(args)
            logger.info('成功启动进程')
            ard = Autoreload(p)
            ard.start_change_detector()
            logger.info('重启进程')


if __name__ == '__main__':
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
    else:
        print('usage: %s start|stop|restart' % sys.argv[0])
        sys.exit(2)
