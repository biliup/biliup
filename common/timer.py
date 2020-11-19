import asyncio
import threading


class Timer(threading.Thread):
    def __init__(self, func=None, args=(), kwargs=None, interval=15, daemon=True):
        threading.Thread.__init__(self, daemon=daemon)
        if kwargs is None:
            kwargs = {}
        self._args = args
        self._kwargs = kwargs
        self._flag = threading.Event()
        self._func = func
        self.interval = interval
        self.task = None
        self.asynchronous = False

    async def astart(self):
        self.asynchronous = True
        self.task = asyncio.create_task(self.arun())
        await self.task

    async def arun(self):
        while True:
            await self.atimer()
            await asyncio.sleep(self.interval)

    async def atimer(self):
        await self._func(*self._args, **self._kwargs)

    def timer(self):
        self._func(*self._args, **self._kwargs)

    def run(self):
        while not self._flag.is_set():
            self.timer()
            self._flag.wait(self.interval)

    def stop(self):
        if not self.asynchronous:
            return self._flag.set()
        self.task.cancel()
