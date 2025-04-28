from biliup.common.tars import tarscore

class HuyaMultiStreamInfo(tarscore.struct):

    __tars_class__ = "Huya.MultiStreamInfo"

    def __init__(self):
        self.sDisplayName: tarscore.string = ""
        self.iBitRate: tarscore.int32 = 0
        self.iCodecType: tarscore.int32 = 0
        self.iCompatibleFlag: tarscore.int32 = 0

    @staticmethod
    def writeTo(oos: tarscore.TarsOutputStream, value):
        oos.write(tarscore.string, 0, value.sDisplayName)
        oos.write(tarscore.int32, 1, value.iBitRate)
        oos.write(tarscore.int32, 2, value.iCodecType)
        oos.write(tarscore.int32, 3, value.iCompatibleFlag)

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaMultiStreamInfo()
        value.sDisplayName = ios.read(tarscore.string, 0, False)
        value.iBitRate = ios.read(tarscore.int32, 1, False)
        value.iCodecType = ios.read(tarscore.int32, 2, False)
        value.iCompatibleFlag = ios.read(tarscore.int32, 3, False)
        return value
