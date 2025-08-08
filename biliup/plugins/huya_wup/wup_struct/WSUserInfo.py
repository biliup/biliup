from biliup.common.tars import tarscore

class HuyaWSUserInfo(tarscore.struct):
    __tars_class__ = "Huya.WSUserInfo"

    def __init__(self):
        self.lUid: tarscore.int64 = 0
        self.bAnonymous: tarscore.boolean = True
        self.sGuid: tarscore.string = ""
        self.sToken: tarscore.string = ""
        self.lTid: tarscore.int64 = 0
        self.lSid: tarscore.int64 = 0
        self.lGroupId: tarscore.int64 = 0
        self.lGroupType: tarscore.int64 = 0

    @staticmethod
    def writeTo(oos: tarscore.TarsOutputStream, value):
        oos.write(tarscore.int64, 0, value.lUid)
        oos.write(tarscore.boolean, 1, value.bAnonymous)
        oos.write(tarscore.string, 2, value.sGuid)
        oos.write(tarscore.string, 3, value.sToken)
        oos.write(tarscore.int64, 4, value.lTid)
        oos.write(tarscore.int64, 5, value.lSid)
        oos.write(tarscore.int64, 6, value.lGroupId)
        oos.write(tarscore.int64, 7, value.lGroupType)

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaWSUserInfo()
        value.lUid = ios.read(tarscore.int64, 0, False)
        value.bAnonymous = ios.read(tarscore.boolean, 1, False)
        value.sGuid = ios.read(tarscore.string, 2, False)
        value.sToken = ios.read(tarscore.string, 3, False)
        value.lTid = ios.read(tarscore.int64, 4, False)
        value.lSid = ios.read(tarscore.int64, 5, False)
        value.lGroupId = ios.read(tarscore.int64, 6, False)
        value.lGroupType = ios.read(tarscore.int64, 7, False)
