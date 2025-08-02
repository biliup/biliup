import struct

from biliup.common.tars.__tup import TarsUniPacket
from biliup.common.tars.__packet import RequestPacket
from biliup.common.tars.__tars import (TarsInputStream, TarsOutputStream)
from biliup.common.tars.__util import util

from .packet.__util import STANDARD_CHARSET


class Wup(TarsUniPacket):
    def __init__(self):
        super().__init__()
        self.__mapa = util.mapclass(util.string, util.bytes)
        self.__new_buffer = self.__mapa()
        self.__code = RequestPacket()

    @property
    def version(self):
        return self.__code.iVersion

    @version.setter
    def version(self, value):
        self.__code.iVersion = value

    @property
    def servant(self):
        return self.__code.sServantName

    @servant.setter
    def servant(self, value):
        self.__code.sServantName = value

    @property
    def func(self):
        return self.__code.sFuncName

    @func.setter
    def func(self, value):
        self.__code.sFuncName = value

    @property
    def requestid(self):
        return self.__code.iRequestId

    @requestid.setter
    def requestid(self, value):
        self.__code.iRequestId = value

    def put(self, vtype, name, value):
        oos = TarsOutputStream()
        oos.write(vtype, 0, value)
        self.__new_buffer[name] = oos.getBuffer()

    def get(self, vtype, name):
        if isinstance(name, str):
            name = name.encode(STANDARD_CHARSET)

        if (name in self.__new_buffer) == False:
            raise Exception("UniAttribute not found key:%s" % name)

        t = self.__new_buffer[name]
        o = TarsInputStream(t)
        return o.read(vtype, 0, True)

    def encode_v3(self):
        oos = TarsOutputStream()
        oos.write(self.__mapa, 0, self.__new_buffer)

        self.__code.iVersion = 3 # Force to use TarsV3
        self.__code.sBuffer = oos.getBuffer()

        sos = TarsOutputStream()
        RequestPacket.writeTo(sos, self.__code)

        return struct.pack('!i', 4 + len(sos.getBuffer())) + sos.getBuffer()

    def decode_v3(self, buf):
        ois = TarsInputStream(buf[4:])
        self.__code = RequestPacket.readFrom(ois)

        sis = TarsInputStream(self.__code.sBuffer)
        self.__new_buffer = sis.read(self.__mapa, 0, True)

    def clear(self):
        self.__code.__init__()