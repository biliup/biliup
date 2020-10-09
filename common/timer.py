import threading


class Timer(threading.Thread):
    def __init__(self, func, args=(), kwargs=None, interval=15):
        threading.Thread.__init__(self)
        if kwargs is None:
            kwargs = {}
        self._args = args
        self._kwargs = kwargs
        self._flag = threading.Event()
        self._func = func
        self.interval = interval

    def __timer(self):
        while not self._flag.is_set():
            self._func(*self._args, **self._kwargs)
            self._flag.wait(self.interval)

    def run(self):
        self.__timer()

    def stop(self):
        self._flag.set()
