# encoding: UTF-8
# 系统模块
import functools
import inspect
import logging
from collections.abc import Generator
from dataclasses import dataclass, field
from queue import Queue
from threading import *

logger = logging.getLogger('biliup')


class EventManager(Thread):
    def __init__(self, context=None, pool=None):
        """初始化事件管理器"""
        super().__init__(name='Synchronous', daemon=True)
        if pool is None:
            pool = {}
        if context is None:
            context = {}
        self.context = context
        # 事件对象列表
        self.__eventQueue = Queue()
        # 事件管理器开关
        self.__active = True
        # 事件处理线程池
        self._pool = pool
        # 阻塞函数列表
        self.__block = []

        # 这里的__handlers是一个字典，用来保存对应的事件的响应函数
        # 其中每个键对应的值是一个列表，列表中保存了对该事件监听的响应函数，一对多
        self.__handlers = {}

        self.__method = {}

    def run(self):
        while self.__active:
            event = self.__eventQueue.get()
            if event is not None:
                self.__event_process(event)

    def __event_process(self, event):
        """处理事件"""
        # 检查是否存在对该事件进行监听的处理函数
        if not self.__active or event.type_ not in self.__handlers:
            return
        # 若存在，则按顺序将事件传递给处理函数执行
        for handler in self.__handlers[event.type_]:
            if handler.__qualname__ in self.__block:
                self._pool.get(handler.pool).submit(handler, event)
            else:
                handler(event)

    def stop(self):
        """停止"""
        # 将事件管理器设为停止
        self.__active = False
        self.__eventQueue.put(None)
        for pool in self._pool.values():
            pool.shutdown()

    def add_event_listener(self, type_, handler):
        """绑定事件和监听器处理函数"""
        # 尝试获取该事件类型对应的处理函数列表，若无则创建
        try:
            handlerlist = self.__handlers[type_]
        except KeyError:
            handlerlist = []

        self.__handlers[type_] = handlerlist
        # 若要注册的处理器不在该事件的处理器列表中，则注册该事件
        if handler not in handlerlist:
            @functools.wraps(handler)
            def try_handler(event):
                try:
                    handler(event)
                except:
                    logger.exception('try_handler')

            handlerlist.append(try_handler)

    def remove_event_listener(self, type_, handler):
        """移除监听器的处理函数"""
        try:
            handler_list = self.__handlers[type_]
            for method in handler_list:
                # 如果该函数存在于列表中，则移除
                if handler.__qualname__ == method.__qualname__:
                    handler_list.remove(method)

                # 如果函数列表为空，则从引擎中移除该事件类型
            if not handler_list:
                del self.__handlers[type_]

        except KeyError:
            pass

    def send_event(self, event):
        """发送事件，向事件队列中存入事件"""
        self.__eventQueue.put(event)

    def register(self, type_, block=False):
        classname = inspect.getouterframes(inspect.currentframe())[1][3]

        def callback(result):
            if not result:
                pass
            elif isinstance(result, (tuple, Generator)):
                for event in result:
                    self.send_event(event)
            else:
                self.send_event(result)

        def appendblock(fc, blk):
            if blk:
                self.__block.append(fc.__qualname__)

        if classname == '<module>':
            def decorator(func):
                appendblock(func, block)

                @functools.wraps(func)
                def wrapper(event):
                    _event = func(*event.args)
                    callback(_event)
                    return _event

                wrapper.pool = block
                self.add_event_listener(type_, wrapper)
                return wrapper
        else:
            def decorator(func):
                appendblock(func, block)

                self.__method.setdefault(type_, [])
                self.__method[type_].append(func.__name__)

                @functools.wraps(func)
                def wrapper(this, event):
                    _event = func(this, *event.args)
                    callback(_event)
                    return _event

                wrapper.pool = block
                return wrapper
        return decorator

    def server(self):
        def decorator(cls):
            sig = inspect.signature(cls)
            kwargs = {}
            for k in sig.parameters:
                kwargs[k] = self.context[k]
            instance = cls(**kwargs)
            self.context[cls.__name__] = instance
            for type_ in self.__method:
                for handler in self.__method[type_]:
                    self.add_event_listener(type_, getattr(instance, handler))
            self.__method.clear()
            return cls

        return decorator


@dataclass
class Event:
    """事件对象"""
    type_: str  # 事件类型
    args: tuple = ()
    dict: dict = field(default_factory=dict)  # type: ignore # 字典用于保存具体的事件数据
