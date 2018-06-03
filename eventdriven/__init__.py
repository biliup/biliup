import time
from eventdriven import eventType, event
__all__ = ['event', 'eventType', 'Putevent']


class Putevent(object):
    def __init__(self, eventmanager, _dict, queue):
        self.eventManager = eventmanager
        self.dict = _dict
        self.q = queue

    def timer(self, interval=40):
        self.eventManager.start()
        for eventtype in self.dict:
            __event = eventType.Event(eventtype)
            self.q.put(__event)
        while True:
            _event = self.q.get()
            self.eventManager.put(_event)
            # print(self.q.qsize())
            time.sleep(interval)


