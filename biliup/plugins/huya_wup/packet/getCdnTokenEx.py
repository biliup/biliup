from biliup.common.tars import tarscore
from .__util import auto_decode_fields
from ..wup_struct.UserId import HuyaUserId

@auto_decode_fields
class HuyaGetCdnTokenExReq(tarscore.struct):

    __tars_class__ = "HUYA.GetCdnTokenExReq"

    def __init__(self):
        self.sFlvUrl: tarscore.string = ""
        self.sStreamName: tarscore.string = ""
        self.iLoopTime: tarscore.int32 = 0
        self.tId: HuyaUserId = HuyaUserId()
        self.iAppId: tarscore.int32 = 66

    @staticmethod
    def writeTo(oos: tarscore.TarsOutputStream, value):
        oos.write(tarscore.string, 0, value.sFlvUrl)
        oos.write(tarscore.string, 1, value.sStreamName)
        oos.write(tarscore.int32, 2, value.iLoopTime)
        oos.write(HuyaUserId, 3, value.tId)
        oos.write(tarscore.int32, 4, value.iAppId)

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaGetCdnTokenExReq()
        value.sFlvUrl = ios.read(tarscore.string, 0, False)
        value.sStreamName = ios.read(tarscore.string, 1, False)
        value.iLoopTime = ios.read(tarscore.int32, 2, False)
        value.tId = ios.read(HuyaUserId, 3, False)
        value.iAppId = ios.read(tarscore.int32, 4, False)
        return value

    def as_dict(self):
        return self.__dict__.copy()

@auto_decode_fields
class HuyaGetCdnTokenExRsp(tarscore.struct):

    __tars_class__ = "HUYA.GetCdnTokenExRsp"

    def __init__(self):
        self.sFlvToken: tarscore.string = ""
        self.iExpireTime: tarscore.int64 = 0

    @staticmethod
    def writeTo(oos: tarscore.TarsOutputStream, value):
        oos.write(tarscore.string, 0, value.sFlvToken)
        oos.write(tarscore.int64, 1, value.iExpireTime)

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaGetCdnTokenExRsp()
        value.sFlvToken = ios.read(tarscore.string, 0, False)
        value.iExpireTime = ios.read(tarscore.int64, 1, False)
        return value

    def as_dict(self):
        return self.__dict__.copy()
