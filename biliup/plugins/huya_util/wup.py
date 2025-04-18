from biliup.common.tars import tarscore
from biliup.common.tars.__tup import TarsUniPacket
from . import (
    HuyaGetCdnTokenInfoReq,
    HuyaGetCdnTokenInfoRsp,
    STANDARD_CHARSET
)

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




if __name__ == "__main__":
    import requests
    import time
    import base64
    cdn: str = "AL"
    stream_name: str = "1001276654-1001276654-4300450483178307584-2002676764-10057-A-0-1-imgplus"
    presenter_uid: int = 1001276654
    wup_url: str = "https://wup.huya.com"
    huya_headers: dict = {
        "user-agent": f"HYSDK(Windows, {int(time.time())})",
        "referer": "https://www.huya.com/",
        "origin": "https://www.huya.com",
    }
    # TupVersion3 = 3
    wup_req = Wup()
    # wup_req.version = TupVersion3
    wup_req.requestid = 1
    wup_req.servant = "liveui"
    wup_req.func = "getCdnTokenInfo"
    token_info_req = HuyaGetCdnTokenInfoReq()
    token_info_req.cdnType = cdn
    token_info_req.streamName = stream_name
    token_info_req.presenterUid = presenter_uid
    wup_req.put_by_class(
        vtype=HuyaGetCdnTokenInfoReq,
        name="tReq",
        value=token_info_req
    )
    data = wup_req.encode_v3()
    # true_data = "AAAAkBADLDxAAVYGbGl2ZXVpZg9nZXRDZG5Ub2tlbkluZm99AABkCAABBgR0UmVxHQAAVwoGABYCQUwmSDEwMDEyNzY2NTQtMTAwMTI3NjY1NC00MzAwNDUwNDgzMTc4MzA3NTg0LTIwMDI2NzY3NjQtMTAwNTctQS0wLTEtaW1ncGx1czI7rkTuC4yYDKgM"
    # true_data = base64.b64decode(true_data)
    # if data != true_data:
    #     raise Exception("data != true_data")
    # data = base64.b64encode(data)
    rsp = requests.post(wup_url, data=data, headers=huya_headers)
    rsp_bytes = rsp.content
    # true_data = "AAACPRADLDxAAVYGbGl2ZXVpZg9nZXRDZG5Ub2tlbkluZm99AAECEAgAAgYAHQAAAQwGBHRSc3AdAAEB+woGABYCQUwmUTExOTk1MjczMTc2MDgtMTE5OTUyNzMxNzYwOC01Mjg5MDAzMjIwMDAwMDQ3MTA0LTIzOTkwNTQ3NTg2NzItMTAwNTctQS0wLTEtaW1ncGx1czJJZlBoRn93c1NlY3JldD1jNzZmYzA0MGViMjMxOGQ3NGRjMjlhNTE1ZDMzNjI1MCZ3c1RpbWU9NjgwMjg4MDgmZm09UkZkeE9FSmpTak5vTmtSS2REWlVXVjhrTUY4a01WOGtNbDhrTXclM0QlM0QmY3R5cGU9aHV5YV9jb21tc2VydmVyVgg2ODAyODZkY2aGd3NTZWNyZXQ9Yzc2ZmMwNDBlYjIzMThkNzRkYzI5YTUxNWQzMzYyNTAmd3NUaW1lPTY4MDI4ODA4JmZtPVJGZHhPRUpqU2pOb05rUktkRFpVV1Y4a01GOGtNVjhrTWw4a013JTNEJTNEJmN0eXBlPWh1eWFfY29tbXNlcnZlciZmcz1nY3R2hndzU2VjcmV0PWM3NmZjMDQwZWIyMzE4ZDc0ZGMyOWE1MTVkMzM2MjUwJndzVGltZT02ODAyODgwOCZmbT1SRmR4T0VKalNqTm9Oa1JLZERaVVdWOGtNRjhrTVY4a01sOGtNdyUzRCUzRCZjdHlwZT1odXlhX2NvbW1zZXJ2ZXImZnM9Z2N0C4yYDKgM"
    # true_data = base64.b64decode(true_data)
    wup_rsp = Wup()
    wup_rsp.decode_v3(rsp_bytes)
    # HACK: InputStream 的 readFrom 没有正确解码字符串，暂时手动编码回 bytes
    token_info_rsp = wup_rsp.get_by_class(
        vtype=HuyaGetCdnTokenInfoRsp,
        name=bytes("tRsp", encoding=STANDARD_CHARSET)
    )
    print(token_info_rsp.as_dict())
