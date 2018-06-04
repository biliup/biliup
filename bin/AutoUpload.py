from threading import Thread
import sys
import os
import time
import subprocess
import logging
import psutil


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


def get_p_children(pid, _recursive=True):
    try:
        parent = psutil.Process(pid)
    except psutil.NoSuchProcess:
        return None
    children = parent.children(recursive=_recursive)
    return children


class Autoreload(object):
    def __init__(self, interval=10):
        # self.p = _process  # 被监控子进程
        self.__thread = Thread(target=self.start_change_detector, args=(interval,))

    def __call__(self, *args, **kwargs):
        self.__thread.start()

    @staticmethod
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

    def _is_any_file_changed(self, mtimes):
        """Return 1 if there is any source file of sys.modules changed,
        otherwise 0. mtimes is dict to store the last modify time for
        comparing."""
        for filename in self._iter_module_files():
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

    @staticmethod
    def _work_free():
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

    def _restart_subp(self, interval=10):
        while True:
            time.sleep(interval)
            if self._work_free():
                # logger.info('重启进程')
                # pid = self.p.pid
                # children = get_p_children(pid)
                #
                # os.kill(pid, SIGTERM)
                #
                # for process in children:
                #     # print(process)
                #     process.terminate()
                path = os.path.join(os.getcwd(), 'Bilibili.py')
                args = [path, 'restart']
                subprocess.Popen(args)
                # 同属于一个进程组所以不能用killpg
                # os.killpg(os.getpgid(pid), SIGTERM)
                return

    def start_change_detector(self, interval):
        """Check file state ervry interval. If any change is detected, exit this
        process with a special code, so that deamon will to restart a new process.
        """
        mtimes = {}
        while 1:
            if self._is_any_file_changed(mtimes):
                self._restart_subp()
                return
            time.sleep(interval)


def autoreload():
    detector = Autoreload(10)
    detector()
