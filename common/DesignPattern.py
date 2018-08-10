def singleton(cls):
    instance = cls()
    cls.__call__ = lambda self: instance
    return instance
