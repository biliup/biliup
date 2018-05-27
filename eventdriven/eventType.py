from Engine import *


class Event:
    """事件对象"""

    def __init__(self, type_=None):
        """Constructor"""
        self.type_ = type_  # 事件类型
        self.dict_ = {}  # 字典用于保存具体的事件数据


class Batch(object):
    def __init__(self, eventManager, dic, Queue):
        self.__eventManager = eventManager
        self.dict = dic
        self.queue = Queue

    def get_downloader(self, dl_dict, key, queue):
        obj = []
        for dl in dl_dict:
            downloader = getattr(download, dl)(self.dict, key, queue)
            obj.append(downloader)
        return obj

    @staticmethod
    def get_handler(obj):
        handler = []
        for obj in obj:
            dl = obj.run
            handler.append(dl)
        return handler

    def addhandler(self, event_, handler):
        # 注册事件
        for dl in handler:
            self.__eventManager.register(event_.type_, dl)

    def register(self):
        for key in self.dict.copy():
            event_ = Event(type_=key)

            dl = self.get_downloader(self.dict[key], key, self.queue)
            handler = self.get_handler(dl)
            self.addhandler(event_, handler)
            
            uploader = upload.Upload(self.dict, key)
            self.__eventManager.register(event_.type_, uploader.start)



