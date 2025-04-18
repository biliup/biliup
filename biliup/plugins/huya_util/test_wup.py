from biliup.plugins.huya_util import Wup, DEFAULT_TICKET_NUMBER
from biliup.plugins.huya_util.packet import (
    HuyaGetCdnTokenInfoReq,
    HuyaGetCdnTokenInfoRsp
)
from biliup.plugins.huya_util.packet.__util import STANDARD_CHARSET


if __name__ == "__main__":
    import requests
    import time
    # import base64
    cdn: str = "AL"
    stream_name: str = "1320365459-1320365459-5670926465173028864-2640854374-10057-A-0-1"
    presenter_uid: int = 1320365459
    wup_url: str = "https://wup.huya.com"
    huya_headers: dict = {
        "user-agent": f"HYSDK(Windows, {int(time.time())})",
        "referer": "https://www.huya.com/",
        "origin": "https://www.huya.com",
    }
    # TupVersion3 = 3
    wup_req = Wup()
    # wup_req.version = TupVersion3
    wup_req.requestid = abs(DEFAULT_TICKET_NUMBER)
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
    rsp = requests.post(wup_url, data=data, headers=huya_headers)
    rsp_bytes = rsp.content
    wup_rsp = Wup()
    wup_rsp.decode_v3(rsp_bytes)
    # WARNING: InputStream 的 readFrom 没有正确解码字符串，暂时手动编码回 bytes
    token_info_rsp = wup_rsp.get_by_class(
        vtype=HuyaGetCdnTokenInfoRsp,
        name=bytes("tRsp", encoding=STANDARD_CHARSET)
    )
    print(token_info_rsp.as_dict())
