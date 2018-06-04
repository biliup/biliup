from bin.Engine import upload


class Event:
    """事件对象"""

    def __init__(self, type_=None):
        """Constructor"""
        self.type_ = type_  # 事件类型
        self.dict_ = {}  # 字典用于保存具体的事件数据


class Batch(object):
    def __init__(self, event_manager, dic):
        self.__eventManager = event_manager
        self.dict = dic
        # self.queue = Queue

    @staticmethod
    def get_downloader(dl_dict, items):
        download = __import__('bin.Engine.download', fromlist=['download'])
        obj = []
        # from Engine import download
        for dl in dl_dict:
            downloader = getattr(download, dl)(items)
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

        for d in self.dict.items():
            key, value = d
            event_ = Event(type_=key)

            dl = self.get_downloader(value, d)
            handler = self.get_handler(dl)
            self.addhandler(event_, handler)

            uploader = upload.Upload(d)
            self.__eventManager.register(event_.type_, uploader.start)
