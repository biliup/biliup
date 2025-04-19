from biliup.common.tars import tarscore
from .__util import auto_decode_fields
from .UserId import HuyaUserId
from .BeginLiveNotice import HuyaBeginLiveNotice
from .StreamSettingNotice import HuyaStreamSettingNotice


class HuyaGetLivingInfoReq(tarscore.struct):

    __tars_class__ = "Huya.GetLivingInfoReq"

    def __init__(self):
        self.tId: HuyaUserId = HuyaUserId()
        self.lTopSid: tarscore.int64 = 0
        self.lSubSid: tarscore.int64 = 0
        self.lPresenterUid: tarscore.int64 = 0

    @staticmethod
    def writeTo(oos: tarscore.TarsOutputStream, value):
        oos.write(HuyaUserId, 0, value.tId)
        oos.write(tarscore.int64, 1, value.lTopSid)
        oos.write(tarscore.int64, 2, value.lSubSid)
        oos.write(tarscore.int64, 3, value.lPresenterUid)

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaGetLivingInfoReq()
        value.tId = ios.read(HuyaUserId, 0, False)
        value.lTopSid = ios.read(tarscore.int64, 1, False)
        value.lSubSid = ios.read(tarscore.int64, 2, False)
        value.lPresenterUid = ios.read(tarscore.int64, 3, False)
        return value



class HuyaGetLivingInfoRsp(tarscore.struct):

    __tars_class__ = "Huya.GetLivingInfoRsp"

    def __init__(self):
        self.bIsLiving: tarscore.int32 = 0
        self.tNotice: HuyaBeginLiveNotice = HuyaBeginLiveNotice()
        self.tStreamSettingNotice: HuyaStreamSettingNotice = HuyaStreamSettingNotice()
        self.bIsSelfLiving: tarscore.int32 = 0

    @staticmethod
    def writeTo(oos: tarscore.TarsOutputStream, value):
        oos.write(
            coder=tarscore.int32,
            tag=0,
            value=value.bIsLiving
        )
        oos.write(
            coder=HuyaBeginLiveNotice,
            tag=1,
            value=value.tNotice
        )
        oos.write(
            coder=HuyaStreamSettingNotice,
            tag=2,
            value=value.tStreamSettingNotice
        )
        oos.write(
            coder=tarscore.int32,
            tag=3,
            value=value.bIsSelfLiving
        )

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaGetLivingInfoRsp()
        value.bIsLiving = ios.read(tarscore.int32, 0, False)
        value.tNotice = ios.read(HuyaBeginLiveNotice, 1, False)
        value.tStreamSettingNotice = ios.read(HuyaStreamSettingNotice, 2, False)
        value.bIsSelfLiving = ios.read(tarscore.int32, 3, False)
        return value
