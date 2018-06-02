import time
from eventdriven import eventType, event
__all__ = ['event', 'eventType', 'Putevent']


class Putevent(object):
    def __init__(self, eventmanager, _dict):
        self.eventManager = eventmanager
        self.dict = _dict

    def timer(self, interval=40):
        self.eventManager.start()
        while True:
            for type_ in self.dict.copy():
                _event = eventType.Event(type_)
                _event.dict_['dict'] = self.dict
                self.eventManager.put(_event)
                time.sleep(interval)


