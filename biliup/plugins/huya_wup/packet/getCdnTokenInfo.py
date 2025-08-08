from biliup.common.tars import tarscore
from .__util import auto_decode_fields

@auto_decode_fields
class HuyaGetCdnTokenReq(tarscore.struct):

    __tars_class__ = "HUYA.GetCdnTokenReq"

    def __init__(self):
        self.url: tarscore.string = ""
        self.cdnType: tarscore.string = ""
        self.streamName: tarscore.string = ""
        self.presenterUid: tarscore.int32 = 0

    @staticmethod
    def writeTo(oos: tarscore.TarsOutputStream, value):
        oos.write(tarscore.string, 0, value.url)
        oos.write(tarscore.string, 1, value.cdnType)
        oos.write(tarscore.string, 2, value.streamName)
        oos.write(tarscore.int64, 3, value.presenterUid)

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaGetCdnTokenReq()
        value.url = ios.read(tarscore.string, 0, False)
        value.cdnType = ios.read(tarscore.string, 1, False)
        value.streamName = ios.read(tarscore.string, 2, False)
        value.presenterUid = ios.read(tarscore.int64, 3, False)
        return value

    def as_dict(self):
        return self.__dict__.copy()

@auto_decode_fields
class HuyaGetCdnTokenRsp(tarscore.struct):

    __tars_class__ = "HUYA.GetCdnTokenRsp"

    def __init__(self):
        self.url: tarscore.string = ""
        self.cdnType: tarscore.string = ""
        self.streamName: tarscore.string = ""
        self.presenterUid: tarscore.int64 = 0
        self.antiCode: tarscore.string = ""
        self.sTime: tarscore.string = ""
        self.flvAntiCode: tarscore.string = ""
        self.hlsAntiCode: tarscore.string = ""

    @staticmethod
    def writeTo(oos: tarscore.TarsOutputStream, value):
        oos.write(tarscore.string, 0, value.url)
        oos.write(tarscore.string, 1, value.cdnType)
        oos.write(tarscore.string, 2, value.streamName)
        oos.write(tarscore.int64, 3, value.presenterUid)
        oos.write(tarscore.string, 4, value.antiCode)
        oos.write(tarscore.string, 5, value.sTime)
        oos.write(tarscore.string, 6, value.flvAntiCode)
        oos.write(tarscore.string, 7, value.hlsAntiCode)

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaGetCdnTokenRsp()
        value.url = ios.read(tarscore.string, 0, False)
        value.cdnType = ios.read(tarscore.string, 1, False)
        value.streamName = ios.read(tarscore.string, 2, False)
        value.presenterUid = ios.read(tarscore.int64, 3, False)
        value.antiCode = ios.read(tarscore.string, 4, False)
        value.sTime = ios.read(tarscore.string, 5, False)
        value.flvAntiCode = ios.read(tarscore.string, 6, False)
        value.hlsAntiCode = ios.read(tarscore.string, 7, False)
        return value

    def as_dict(self):
        return self.__dict__.copy()
