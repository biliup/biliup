import threading
from _thread import LockType


# 简单实现的命名锁
class NamedLock:
    _lock_dict = {}

    def __new__(cls, name) -> LockType:
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
