import logging
from bin.eventdriven.eventType import Event
import threading
logger = logging.getLogger('log01')
__all__ = ['event', 'eventType', 'Timer']


def event_queue(_dict, queue):
    for eventtype in _dict:
        __event = Event(eventtype)
        queue.put(__event)


def put_event(event_manager, q):
    _event = q.get()
    event_manager.put(_event)


class Timer(object):
    def __init__(self, func, args=(), kwargs=None, interval=40):
        if kwargs is None:
            kwargs = {}
        self._args = args
        self._kwargs = kwargs
        self.active = True
        self.__flag = threading.Event()
        self._func = func
        self.interval = interval

    def __timer(self):
        while self.active:
            self._func(*self._args, **self._kwargs)
            self.__flag.wait(self.interval)

    def start(self):
        self.__timer()

    def stop(self):
        self.active = False
        self.__flag.set()
