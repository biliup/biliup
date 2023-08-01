import threading
from _thread import LockType


# 简单实现的命名锁
class NamedLock:
    _lock_dict = {}

    def __new__(cls, name) -> LockType:
        if name not in cls._lock_dict:
            cls._lock_dict[name] = threading.Lock()
        return cls._lock_dict[name]
