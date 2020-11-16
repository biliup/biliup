import threading


class Timer(threading.Thread):
    def __init__(self, func=None, args=(), kwargs=None, interval=15):
        threading.Thread.__init__(self, daemon=True)
        if kwargs is None:
            kwargs = {}
        self._args = args
        self._kwargs = kwargs
        self._flag = threading.Event()
        self._func = func
        self.interval = interval

    def timer(self):
        self._func(*self._args, **self._kwargs)

    def run(self):
        while not self._flag.is_set():
            self.timer()
            self._flag.wait(self.interval)

    def stop(self):
        self._flag.set()
