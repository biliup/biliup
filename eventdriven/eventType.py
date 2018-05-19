from Engine import *


class Event:
    """事件对象"""

    def __init__(self, type_=None):
        """Constructor"""
        self.type_ = type_  # 事件类型
        self.dict_ = {}  # 字典用于保存具体的事件数据


class RegisterEvent(object):
    def __init__(self, eventManager, dic):
        self.__eventManager = eventManager
        self.dict = dic
        self.obj = []

    def get_dl_obj(self, ddict, key, queue):
        for dl in ddict:
            obj = getattr(download, dl)(self.dict, key, queue)
            self.obj.append(obj)

    def addhandler(self, event_):
        # 注册事件
        for obj in self.obj:
            handler = obj.run
            self.__eventManager.register(event_.type_, handler)
        self.obj.clear()

    def creator(self, queue):
        for key in self.dict.copy():
            event_ = Event(type_=key)
            self.get_dl_obj(self.dict[key], key, queue)
            self.addhandler(event_)
            self.__eventManager.register(event_.type_, upload.Upload(self.dict, key).supplemental_upload)



