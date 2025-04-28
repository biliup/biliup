from biliup.common.tars import tarscore
from .StreamInfo import HuyaStreamInfo
from .MultiStreamInfo import HuyaMultiStreamInfo


class HuyaBeginLiveNotice(tarscore.struct):

    __tars_class__ = "Huya.BeginLiveNotice"

    def __init__(self):
        self.lPresenterUid: tarscore.int64 = 0
        self.iGameId: tarscore.int32 = 0
        self.sGameName: tarscore.string = ""
        self.iRandomRange: tarscore.int32 = 0
        self.iStreamType: tarscore.int32 = 0
        self.vStreamInfo: tarscore.vector = tarscore.vector(HuyaStreamInfo) # FIXME
        self.vCdnList: tarscore.vector = tarscore.vector(tarscore.string) # FIXME
        self.lLiveId: tarscore.int64 = 0
        self.iPCDefaultBitRate: tarscore.int32 = 0
        self.iWebDefaultBitRate: tarscore.int32 = 0
        self.iMobileDefaultBitRate: tarscore.int32 = 0
        self.lMultiStreamFlag: tarscore.int64 = 0
        self.sNick: tarscore.string = ""
        self.lYYId: tarscore.int64 = 0
        self.lAttendeeCount: tarscore.int64 = 0
        self.iCodecType: tarscore.int32 = 0
        self.iScreenType: tarscore.int32 = 0
        self.vMultiStreamInfo: tarscore.vector = tarscore.vector(HuyaMultiStreamInfo) # FIXME
        self.sLiveDesc: tarscore.string = ""
        self.lLiveCompatibleFlag: tarscore.int64 = 0
        self.sAvatarUrl: tarscore.string = ""
        self.iSourceType: tarscore.int32 = 0
        self.sSubchannelName: tarscore.string = ""
        self.sVideoCaptureUrl: tarscore.string = ""
        self.iStartTime: tarscore.int32 = 0
        self.lChannelId: tarscore.int64 = 0
        self.lSubChannelId: tarscore.int64 = 0
        self.sLocation: tarscore.string = ""

    @staticmethod
    def writeTo(oos: tarscore.TarsOutputStream, value):
        oos.write(tarscore.int64, 0, value.lPresenterUid)
        oos.write(tarscore.int32, 1, value.iGameId)
        oos.write(tarscore.string, 2, value.sGameName)
        oos.write(tarscore.int32, 3, value.iRandomRange)
        oos.write(tarscore.int32, 4, value.iStreamType)
        oos.write(tarscore.vector, 5, value.vStreamInfo)
        oos.write(tarscore.vector, 6, value.vCdnList)
        oos.write(tarscore.int64, 7, value.lLiveId)
        oos.write(tarscore.int32, 8, value.iPCDefaultBitRate)
        oos.write(tarscore.int32, 9, value.iWebDefaultBitRate)
        oos.write(tarscore.int32, 10, value.iMobileDefaultBitRate)
        oos.write(tarscore.int64, 11, value.lMultiStreamFlag)
        oos.write(tarscore.string, 12, value.sNick)
        oos.write(tarscore.int64, 13, value.lYYId)
        oos.write(tarscore.int64, 14, value.lAttendeeCount)
        oos.write(tarscore.int32, 15, value.iCodecType)
        oos.write(tarscore.int32, 16, value.iScreenType)
        oos.write(tarscore.vector, 17, value.vMultiStreamInfo)
        oos.write(tarscore.string, 18, value.sLiveDesc)
        oos.write(tarscore.int64, 19, value.lLiveCompatibleFlag)
        oos.write(tarscore.string, 20, value.sAvatarUrl)
        oos.write(tarscore.int32, 21, value.iSourceType)
        oos.write(tarscore.string, 22, value.sSubchannelName)
        oos.write(tarscore.string, 23, value.sVideoCaptureUrl)
        oos.write(tarscore.int32, 24, value.iStartTime)
        oos.write(tarscore.int64, 25, value.lChannelId)
        oos.write(tarscore.int64, 26, value.lSubChannelId)
        oos.write(tarscore.string, 27, value.sLocation)

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaBeginLiveNotice()
        value.lPresenterUid = ios.read(tarscore.int64, 0, False)
        value.iGameId = ios.read(tarscore.int32, 1, False)
        value.sGameName = ios.read(tarscore.string, 2, False)
        value.iRandomRange = ios.read(tarscore.int32, 3, False)
        value.iStreamType = ios.read(tarscore.int32, 4, False)
        value.vStreamInfo = ios.read(tarscore.vector, 5, False)
        value.vCdnList = ios.read(tarscore.vector, 6, False)
        value.lLiveId = ios.read(tarscore.int64, 7, False)
        value.iPCDefaultBitRate = ios.read(tarscore.int32, 8, False)
        value.iWebDefaultBitRate = ios.read(tarscore.int32, 9, False)
        value.iMobileDefaultBitRate = ios.read(tarscore.int32, 10, False)
        value.lMultiStreamFlag = ios.read(tarscore.int64, 11, False)
        value.sNick = ios.read(tarscore.string, 12, False)
        value.lYYId = ios.read(tarscore.int64, 13, False)
        value.lAttendeeCount = ios.read(tarscore.int64, 14, False)
        value.iCodecType = ios.read(tarscore.int32, 15, False)
        value.iScreenType = ios.read(tarscore.int32, 16, False)
        value.vMultiStreamInfo = ios.read(tarscore.vector, 17, False)
        value.sLiveDesc = ios.read(tarscore.string, 18, False)
        value.lLiveCompatibleFlag = ios.read(tarscore.int64, 19, False)
        value.sAvatarUrl = ios.read(tarscore.string, 20, False)
        value.iSourceType = ios.read(tarscore.int32, 21, False)
        value.sSubchannelName = ios.read(tarscore.string, 22, False)
        value.sVideoCaptureUrl = ios.read(tarscore.string, 23, False)
        value.iStartTime = ios.read(tarscore.int32, 24, False)
        value.lChannelId = ios.read(tarscore.int64, 25, False)
        value.lSubChannelId = ios.read(tarscore.int64, 26, False)
        value.sLocation = ios.read(tarscore.string, 27, False)
        return value

    def as_dict(self):
        return {
            "lPresenterUid": self.lPresenterUid,
            "iGameId": self.iGameId,
            "sGameName": self.sGameName,
            "iRandomRange": self.iRandomRange,
            "iStreamType": self.iStreamType,
            "vStreamInfo": self.vStreamInfo,
            "vCdnList": self.vCdnList,
            "lLiveId": self.lLiveId,
            "iPCDefaultBitRate": self.iPCDefaultBitRate,
            "iWebDefaultBitRate": self.iWebDefaultBitRate,
            "iMobileDefaultBitRate": self.iMobileDefaultBitRate,
            "lMultiStreamFlag": self.lMultiStreamFlag,
            "sNick": self.sNick,
            "lYYId": self.lYYId,
            "lAttendeeCount": self.lAttendeeCount,
            "iCodecType": self.iCodecType,
            "iScreenType": self.iScreenType,
            "vMultiStreamInfo": self.vMultiStreamInfo,
            "sLiveDesc": self.sLiveDesc,
            "lLiveCompatibleFlag": self.lLiveCompatibleFlag,
            "sAvatarUrl": self.sAvatarUrl,
            "iSourceType": self.iSourceType,
            "sSubchannelName": self.sSubchannelName,
            "sVideoCaptureUrl": self.sVideoCaptureUrl,
            "iStartTime": self.iStartTime,
            "lChannelId": self.lChannelId,
            "lSubChannelId": self.lSubChannelId,
            "sLocation": self.sLocation
        }