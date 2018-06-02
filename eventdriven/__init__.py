import time
from eventdriven import *
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
                self.eventManager.put(_event)
                time.sleep(interval)

def func(event):
    print('running'+event.type_)
    time.sleep(10)
    print('done'+event.type_)
def fun(event):
    print('running'+event.type_)
    time.sleep(10)
    print('done'+event.type_)
if __name__ == '__main__':
    manager = event.EventEngine()

    e = eventType.Event(type_='1')
    e1 = eventType.Event(type_='2')
    e2 = eventType.Event(type_='3')
    e3 = eventType.Event(type_='4')

    manager.register(e.type_, func)
    manager.register(e.type_, fun)
    manager.register(e1.type_, func)
    manager.register(e2.type_, func)
    manager.register(e3.type_, func)
    manager.start()
    print('go')
    while True:
        for i in event.d:
            _event = eventType.Event(i)
            manager.put(_event)
            # print('put'+i)
            print(event.d)
            time.sleep(1)