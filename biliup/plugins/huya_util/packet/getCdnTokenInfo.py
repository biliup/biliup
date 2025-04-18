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
        oos.write(
            coder=tarscore.string,
            tag=0,
            value=value.url
        )
        oos.write(
            coder=tarscore.string,
            tag=1,
            value=value.cdnType
        )
        oos.write(
            coder=tarscore.string,
            tag=2,
            value=value.streamName
        )
        oos.write(
            coder=tarscore.int32,
            tag=3,
            value=value.presenterUid
        )

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaGetCdnTokenInfoReq()
        value.url = ios.read(
            coder=tarscore.string,
            tag=0,
            require=False
        )
        # print(("url = %s" % value.url))
        value.cdnType = ios.read(
            coder=tarscore.string,
            tag=1,
            require=False
        )
        # print(("cdnType = %s" % value.cdnType))
        value.streamName = ios.read(
            coder=tarscore.string,
            tag=2,
            require=False
        )
        # print(("streamName = %s" % value.streamName))
        value.presenterUid = ios.read(
            coder=tarscore.int32,
            tag=3,
            require=False
        )
        # print(("presenterUid = %d" % value.presenterUid))
        return value

    def as_dict(self):
        return {
            "url": self.url,
            "cdnType": self.cdnType,
            "streamName": self.streamName,
            "presenterUid": self.presenterUid
        }

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
        oos.write(
            coder=tarscore.string,
            tag=0,
            value=value.url
        )
        oos.write(
            coder=tarscore.string,
            tag=1,
            value=value.cdnType
        )
        oos.write(
            coder=tarscore.string,
            tag=2,
            value=value.streamName
        )
        oos.write(
            coder=tarscore.int32,
            tag=3,
            value=value.presenterUid
        )
        oos.write(
            coder=tarscore.string,
            tag=4,
            value=value.antiCode
        )
        oos.write(
            coder=tarscore.string,
            tag=5,
            value=value.sTime
        )
        oos.write(
            coder=tarscore.string,
            tag=6,
            value=value.flvAntiCode
        )
        oos.write(
            coder=tarscore.string,
            tag=7,
            value=value.hlsAntiCode
        )

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaGetCdnTokenInfoRsp()
        value.url = ios.read(
            coder=tarscore.string,
            tag=0,
            require=False
        )
        # print(("url = %s" % value.url))
        value.cdnType = ios.read(
            coder=tarscore.string,
            tag=1,
            require=False
        )
        # print(("cdnType = %s" % value.cdnType))
        value.streamName = ios.read(
            coder=tarscore.string,
            tag=2,
            require=False
        )
        # print(("streamName = %s" % value.streamName))
        value.presenterUid = ios.read(
            coder=tarscore.int32,
            tag=3,
            require=False
        )
        # print(("presenterUid = %d" % value.presenterUid))
        value.antiCode = ios.read(
            coder=tarscore.string,
            tag=4,
            require=False
        )
        # print(("antiCode = %s" % value.antiCode))
        value.sTime = ios.read(
            coder=tarscore.string,
            tag=5,
            require=False
        )
        # print(("sTime = %s" % value.sTime))
        value.flvAntiCode = ios.read(
            coder=tarscore.string,
            tag=6,
            require=False
        )
        # print(("flvAntiCode = %s" % value.flvAntiCode))
        value.hlsAntiCode = ios.read(
            coder=tarscore.string,
            tag=7,
            require=False
        )
        # print(("hlsAntiCode = %s" % value.hlsAntiCode))
        return value

    def as_dict(self):
        return {
            "url": self.url,
            "cdnType": self.cdnType,
            "streamName": self.streamName,
            "presenterUid": self.presenterUid,
            "antiCode": self.antiCode,
            "sTime": self.sTime,
            "flvAntiCode": self.flvAntiCode,
            "hlsAntiCode": self.hlsAntiCode,
        }
