def singleton(cls):
    instance = cls()
    cls.__call__ = lambda self: instance
    return instance


# 用来包装需要多进程的函数（多进程执行避免主进程阻塞）
class Service:
    def __init__(self, pool, func, callback):
        # 事件处理进程池
        self.__pool = pool
        self.__func = func
        self.callback = callback

    def __run(self, args):
        self.__pool.apply_async(func=self.__func, args=args, callback=self.callback)
        # self.__pool.apply(func=self.__func, args=args)

    def start(self, event):
        args = event.args
        self.__run(args)

    def stop(self):
        self.__pool.close()
        self.__pool.join()
