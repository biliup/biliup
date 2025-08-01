from biliup.common.tars import tarscore
from .__util import auto_decode_fields

@auto_decode_fields
class HuyaGetCdnTokenInfoReq(tarscore.struct):

    __tars_class__ = "Huya.GetCdnTokenInfoReq"

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
        oos.write(tarscore.int32, 3, value.presenterUid)

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaGetCdnTokenInfoReq()
        value.url = ios.read(tarscore.string, 0, False)
        # print(("url = %s" % value.url))
        value.cdnType = ios.read(tarscore.string, 1, False)
        # print(("cdnType = %s" % value.cdnType))
        value.streamName = ios.read(tarscore.string, 2, False)
        # print(("streamName = %s" % value.streamName))
        value.presenterUid = ios.read(tarscore.int32, 3, False)
        # print(("presenterUid = %d" % value.presenterUid))
        return value

    def as_dict(self):
        return self.__dict__.copy()

@auto_decode_fields
class HuyaGetCdnTokenInfoRsp(tarscore.struct):

    __tars_class__ = "Huya.GetCdnTokenInfoRsp"

    def __init__(self):
        self.url: tarscore.string = ""
        self.cdnType: tarscore.string = ""
        self.streamName: tarscore.string = ""
        self.presenterUid: tarscore.int32 = 0
        self.antiCode: tarscore.string = ""
        self.sTime: tarscore.string = ""
        self.flvAntiCode: tarscore.string = ""
        self.hlsAntiCode: tarscore.string = ""

    @staticmethod
    def writeTo(oos: tarscore.TarsOutputStream, value):
        oos.write(tarscore.string, 0, value.url)
        oos.write(tarscore.string, 1, value.cdnType)
        oos.write(tarscore.string, 2, value.streamName)
        oos.write(tarscore.int32, 3, value.presenterUid)
        oos.write(tarscore.string, 4, value.antiCode)
        oos.write(tarscore.string, 5, value.sTime)
        oos.write(tarscore.string, 6, value.flvAntiCode)
        oos.write(tarscore.string, 7, value.hlsAntiCode)

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaGetCdnTokenInfoRsp()
        value.url = ios.read(tarscore.string, 0, False)
        # print(("url = %s" % value.url))
        value.cdnType = ios.read(tarscore.string, 1, False)
        # print(("cdnType = %s" % value.cdnType))
        value.streamName = ios.read(tarscore.string, 2, False)
        # print(("streamName = %s" % value.streamName))
        value.presenterUid = ios.read(tarscore.int32, 3, False)
        # print(("presenterUid = %d" % value.presenterUid))
        value.antiCode = ios.read(tarscore.string, 4, False)
        # print(("antiCode = %s" % value.antiCode))
        value.sTime = ios.read(tarscore.string, 5, False)
        # print(("sTime = %s" % value.sTime))
        value.flvAntiCode = ios.read(tarscore.string, 6, False)
        # print(("flvAntiCode = %s" % value.flvAntiCode))
        value.hlsAntiCode = ios.read(tarscore.string, 7, False)
        # print(("hlsAntiCode = %s" % value.hlsAntiCode))
        return value

    def as_dict(self):
        return self.__dict__.copy()
