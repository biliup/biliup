from biliup.common.tars import tarscore

class HuyaStreamInfo(tarscore.struct):

    __tars_class__ = "Huya.StreamInfo"

    def __init__(self):
        self.sCdnType: tarscore.string = ""
        self.iIsMaster: tarscore.int32 = 0
        self.lChannelId: tarscore.int64 = 0
        self.lSubChannelId: tarscore.int64 = 0
        self.lPresenterUid: tarscore.int64 = 0
        self.sStreamName: tarscore.string = ""
        self.sFlvUrl: tarscore.string = ""
        self.sFlvUrlSuffix: tarscore.string = ""
        self.sFlvAntiCode: tarscore.string = ""
        self.sHlsUrl: tarscore.string = ""
        self.sHlsUrlSuffix: tarscore.string = ""
        self.sHlsAntiCode: tarscore.string = ""
        self.iLineIndex: tarscore.int32 = 0
        self.iIsMultiStream: tarscore.int32 = 0
        self.iPCPriorityRate: tarscore.int32 = 0
        self.iWebPriorityRate: tarscore.int32 = 0
        self.iMobilePriorityRate: tarscore.int32 = 0

    @staticmethod
    def writeTo(oos: tarscore.TarsOutputStream, value):
        oos.write(tarscore.string, 0, value.sCdnType)
        oos.write(tarscore.int32, 1, value.iIsMaster)
        oos.write(tarscore.int64, 2, value.lChannelId)
        oos.write(tarscore.int64, 3, value.lSubChannelId)
        oos.write(tarscore.int64, 4, value.lPresenterUid)
        oos.write(tarscore.string, 5, value.sStreamName)
        oos.write(tarscore.string, 6, value.sFlvUrl)
        oos.write(tarscore.string, 7, value.sFlvUrlSuffix)
        oos.write(tarscore.string, 8, value.sFlvAntiCode)
        oos.write(tarscore.string, 9, value.sHlsUrl)
        oos.write(tarscore.string, 10, value.sHlsUrlSuffix)
        oos.write(tarscore.string, 11, value.sHlsAntiCode)
        oos.write(tarscore.int32, 12, value.iLineIndex)
        oos.write(tarscore.int32, 13, value.iIsMultiStream)
        oos.write(tarscore.int32, 14, value.iPCPriorityRate)
        oos.write(tarscore.int32, 15, value.iWebPriorityRate)
        oos.write(tarscore.int32, 16, value.iMobilePriorityRate)

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaStreamInfo()
        value.sCdnType = ios.read(tarscore.string, 0, False)
        value.iIsMaster = ios.read(tarscore.int32, 1, False)
        value.lChannelId = ios.read(tarscore.int64, 2, False)
        value.lSubChannelId = ios.read(tarscore.int64, 3, False)
        value.lPresenterUid = ios.read(tarscore.int64, 4, False)
        value.sStreamName = ios.read(tarscore.string, 5, False)
        value.sFlvUrl = ios.read(tarscore.string, 6, False)
        value.sFlvUrlSuffix = ios.read(tarscore.string, 7, False)
        value.sFlvAntiCode = ios.read(tarscore.string, 8, False)
        value.sHlsUrl = ios.read(tarscore.string, 9, False)
        value.sHlsUrlSuffix = ios.read(tarscore.string, 10, False)
        value.sHlsAntiCode = ios.read(tarscore.string, 11, False)
        value.iLineIndex = ios.read(tarscore.int32, 12, False)
        value.iIsMultiStream = ios.read(tarscore.int32, 13, False)
        value.iPCPriorityRate = ios.read(tarscore.int32, 14, False)
        value.iWebPriorityRate = ios.read(tarscore.int32, 15, False)
        value.iMobilePriorityRate = ios.read(tarscore.int32, 16, False)
        return value



