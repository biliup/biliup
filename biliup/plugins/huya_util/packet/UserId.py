from biliup.common.tars import tarscore

class HuyaUserId(tarscore.struct):

    __tars_class__ = "Huya.UserId"

    def __init__(self):
        self.lUid: tarscore.int64 = 0
        self.sGuid: tarscore.string = ""
        self.sToken: tarscore.string = ""
        self.sHuYaUA: tarscore.string = ""
        self.sCookie: tarscore.string = ""

    @staticmethod
    def writeTo(oos: tarscore.TarsOutputStream, value):
        oos.write(
            coder=tarscore.int64,
            tag=0,
            value=value.lUid
        )
        oos.write(
            coder=tarscore.string,
            tag=1,
            value=value.sGuid
        )
        oos.write(
            coder=tarscore.string,
            tag=2,
            value=value.sToken
        )
        oos.write(
            coder=tarscore.string,
            tag=3,
            value=value.sHuYaUA
        )
        oos.write(
            coder=tarscore.string,
            tag=4,
            value=value.sCookie
        )

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaUserId()
        value.lUid = ios.read(tarscore.int64, 0, False)
        # print(("lUid = %d" % value.lUid))
        value.sGuid = ios.read(tarscore.string, 1, False)
        # print(("sGuid = %s" % value.sGuid))
        value.sToken = ios.read(tarscore.string, 2, False)
        # print(("sToken = %s" % value.sToken))
        value.sHuYaUA = ios.read(tarscore.string, 3, False)
        # print(("sHuYaUA = %s" % value.sHuYaUA))
        value.sCookie = ios.read(tarscore.string, 4, False)
        # print(("sCookie = %s" % value.sCookie))
        return value