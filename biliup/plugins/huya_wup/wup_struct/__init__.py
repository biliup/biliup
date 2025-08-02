from enum import IntEnum

class EWebSocketCommandType(IntEnum):
    EWSCmd_NULL = 0
    EWSCmd_RegisterReq = 1
    EWSCmd_RegisterRsp = 2
    EWSCmd_WupReq = 3
    EWSCmd_WupRsp = 4
    EWSCmdC2S_HeartBeat = 5
    EWSCmdS2C_HeartBeatAck = 6
    EWSCmdS2C_MsgPushReq = 7
    EWSCmdC2S_DeregisterReq = 8
    EWSCmdS2C_DeRegisterRsp = 9
    EWSCmdC2S_VerifyCookieReq = 10
    EWSCmdS2C_VerifyCookieRsp = 11
    EWSCmdC2S_VerifyHuyaTokenReq = 12
    EWSCmdS2C_VerifyHuyaTokenRsp = 13
