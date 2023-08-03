import asyncio
import logging
import subprocess
import sys
import os
from .timer import Timer

logger = logging.getLogger('biliup')

global global_reloader


def has_extension(fname_list, *extension):
    for fname in fname_list:
        result = list(map(fname.endswith, extension))
        if True in result:
            return True
    return False


class AutoReload(Timer):
    def __init__(self, *watched, interval=10):
        super().__init__(interval)
        self.watched = watched
        self.mtimes = {}
        self.triggered = False

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

    def _is_any_file_changed(self):
        """Return 1 if there is any source file of sys.modules changed,
        otherwise 0. mtimes is dict to store the last modify time for
        comparing."""
        for filename in self._iter_module_files():
            try:
                mtime = os.stat(filename).st_mtime
            except IOError:
                continue
            old_time = self.mtimes.get(filename, None)
            if old_time is None:
                self.mtimes[filename] = mtime
            elif mtime > old_time:
                logger.info('模块已更新')
                return True
        return False

    @staticmethod
    def _work_free():
        fname_list = os.listdir('.')
        if has_extension(fname_list, '.mp4', '.flv', '.3gp', '.webm', '.mkv', '.ts', '.part'):
            return False
        logger.info('进程空闲')
        return True

    async def atimer(self):
        """Check file state ervry interval. If any change is detected, exit this
        process with a special code, so that deamon will to restart a new process.
        """
        if not self._is_any_file_changed() and not self.triggered:
            return
        while True:
            await asyncio.sleep(self.interval)
            if self._work_free():
                for watched in self.watched:
                    if callable(watched):
                        if asyncio.iscoroutinefunction(watched):
                            await watched()
                        else:
                            watched()
                    else:
                        watched.stop()
                self.stop()
                # parent_path = os.path.abspath(os.path.dirname(os.path.dirname(__file__)))  # 获得所在的目录的父级目
                # path = os.path.join(parent_path, '__main__.py')
                # if sys.platform == 'win32':
                #     args = ["python", path]
                # else:
                #     args = [path, 'start']
                logger.info('重启')
                if not is_docker():
                    subprocess.Popen(sys.argv)
                return


def is_docker():
    path = '/proc/self/cgroup'
    return (
            os.path.exists('/.dockerenv') or
            os.path.isfile(path) and any('docker' in line for line in open(path))
    )
