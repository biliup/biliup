from biliup.common.tars import tarscore
from biliup.common.tars.__tars import BinBuffer

class HuyaWebSocketCommand(tarscore.struct):
    __tars_class__ = "Huya.WebSocketCommand"

    def __init__(self):
        self.iCmdType: tarscore.int32 = 0
        self.vData: tarscore.bytes = b''

    @staticmethod
    def writeTo(oos: tarscore.TarsOutputStream, value):
        oos.write(tarscore.int32, 0, value.iCmdType)
        oos.write(tarscore.bytes, 1, value.vData)
        # oos.write(tarscore.int64, 2, value.lRequestId)
        # oos.write(tarscore.string, 3, value.traceId)

    @staticmethod
    def readFrom(ios: tarscore.TarsInputStream):
        value = HuyaWebSocketCommand()
        value.iCmdType = ios.read(tarscore.int32, 0, False)
        value.vData = ios.read(tarscore.bytes, 1, False)
        # value.lRequestId = ios.read(tarscore.int64, 2, False)
        # value.traceId = ios.read(tarscore.string, 3, False)
