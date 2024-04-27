import logging
import subprocess

logger = logging.getLogger('biliup')


class NamedLock:
    """
    简单实现的命名锁
    """
    from _thread import LockType
    _lock_dict = {}

    def __new__(cls, name) -> LockType:
        import threading
        if name not in cls._lock_dict:
            cls._lock_dict[name] = threading.Lock()
        return cls._lock_dict[name]


def silence_event_loop_closed(func):
    from functools import wraps

    @wraps(func)
    def wrapper(self, *args, **kwargs):
        try:
            return func(self, *args, **kwargs)
        except RuntimeError as e:
            if str(e) != 'Event loop is closed':
                raise

    return wrapper


def get_file_create_timestamp(file: str) -> float:
    """
    跨平台获取文件创建时间
    如无法获取则返回修改时间
    """
    import os
    import sys
    import platform
    stat_result = os.stat(file)

    if hasattr(stat_result, "st_birthtime"):
        return stat_result.st_birthtime

    if platform.system() == 'Windows' and sys.version_info < (3, 12):
        return stat_result.st_ctime

    if platform.system() == 'Linux':
        try:
            import subprocess
            time = float(subprocess.check_output(["stat", "-c", "%W", file]).decode('utf8'))
            if time > 0:
                return time
        except:
            pass

    return stat_result.st_mtime


def processor(processors, data):
    for process in processors:
        if process.get('run'):
            try:
                process_output = subprocess.check_output(
                    process['run'], shell=True,
                    input=data,
                    stderr=subprocess.STDOUT, text=True)
                logger.info(process_output.rstrip())
            except subprocess.CalledProcessError as e:
                logger.exception(e.output)
                continue
