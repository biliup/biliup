from biliup.common.tars import tarscore
from biliup.common.tars.__tup import TarsUniPacket

class Wup(TarsUniPacket):
    def __init__(self):
        super().__init__()

    @classmethod
    def writeTo(cls, oos: tarscore.TarsOutputStream):
        return cls.__code.writeTo(oos)

    @classmethod
    def readFrom(cls, ios: tarscore.TarsInputStream):
        return cls.__code.readFrom(ios)

    def encode(self):
        return super().encode()

    def encode_v3(self):
        return super().encode_v3()

    def decode(self, buf):
        super().decode(buf)

    def decode_v3(self, buf):
        super().decode_v3(buf)

    def get(self, vtype, name):
        return super().get(vtype, name)

    def get_by_class(self, vtype, name):
        return super().get_by_class(vtype, name)

    def put(self, vtype, name, value):
        super().put(vtype, name, value)

    def put_by_class(self, vtype, name, value):
        super().put_by_class(vtype, name, value)
