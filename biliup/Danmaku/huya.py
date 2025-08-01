import aiohttp
from biliup.plugins import random_user_agent

from biliup.common.tars import tarscore
from biliup.plugins import match1
from biliup.plugins.huya_wup.wup_struct import EWebSocketCommandType
from biliup.plugins.huya_wup.wup_struct.WebSocketCommand import HuyaWebSocketCommand
from biliup.plugins.huya_wup.wup_struct.WSUserInfo import HuyaWSUserInfo


class Huya:
    wss_url = 'wss://cdnws.api.huya.com/'
    heartbeat = b'\x00\x03\x1d\x00\x00\x69\x00\x00\x00\x69\x10\x03\x2c\x3c\x4c\x56\x08\x6f\x6e\x6c\x69\x6e\x65\x75' \
                b'\x69\x66\x0f\x4f\x6e\x55\x73\x65\x72\x48\x65\x61\x72\x74\x42\x65\x61\x74\x7d\x00\x00\x3c\x08\x00' \
                b'\x01\x06\x04\x74\x52\x65\x71\x1d\x00\x00\x2f\x0a\x0a\x0c\x16\x00\x26\x00\x36\x07\x61\x64\x72\x5f' \
                b'\x77\x61\x70\x46\x00\x0b\x12\x03\xae\xf0\x0f\x22\x03\xae\xf0\x0f\x3c\x42\x6d\x52\x02\x60\x5c\x60' \
                b'\x01\x7c\x82\x00\x0b\xb0\x1f\x9c\xac\x0b\x8c\x98\x0c\xa8\x0c '
    heartbeatInterval = 60
    # 等待统一ua后修改
    headers = {
        'user-agent': random_user_agent(),
    }

    @staticmethod
    async def get_ws_info(url, context):
        reg_datas = []
        room_id = url.split('huya.com/')[1].split('/')[0].split('?')[0]
        async with aiohttp.ClientSession() as session:
            async with session.get(f'https://www.huya.com/{room_id}', headers=Huya.headers, timeout=5) as resp:
                room_page = await resp.text()
                uid = int(match1(room_page, r"uid\":\"?(\d+)\"?"))
                # tid = match1(room_page, r"lChannelId\":\"?(\d+)\"?")
                # sid = match1(room_page, r"lSubChannelId\":\"?(\d+)\"?")

        import base64
        ws_user_info = HuyaWSUserInfo()
        ws_user_info.iUid = uid
        ws_user_info.bAnonymous = False
        ws_user_info.lGroupId = uid
        ws_user_info.lGroupType = 3
        oos = tarscore.TarsOutputStream()
        ws_user_info.writeTo(oos, ws_user_info)

        # b64data = base64.b64encode(oos.getBuffer())
        # print(b64data)

        ws_cmd = HuyaWebSocketCommand()
        ws_cmd.iCmdType = EWebSocketCommandType.EWSCmd_RegisterReq
        ws_cmd.vData = oos.getBuffer()
        oos = tarscore.TarsOutputStream()
        ws_cmd.writeTo(oos, ws_cmd)

        # oos = tarscore.TarsOutputStream()
        # oos.write(tarscore.int64, 0, uid)
        # oos.write(tarscore.boolean, 1, False)  # Anonymous
        # oos.write(tarscore.string, 2, "")  # sGuid
        # oos.write(tarscore.string, 3, "")
        # oos.write(tarscore.int64, 4, 0)  # tid
        # oos.write(tarscore.int64, 5, 0)  # sid
        # oos.write(tarscore.int64, 6, uid)
        # oos.write(tarscore.int64, 7, 3)

        # b64data = base64.b64encode(oos.getBuffer())
        # print(b64data)

        # wscmd = tarscore.TarsOutputStream()
        # wscmd.write(tarscore.int32, 0, 1)
        # wscmd.write(tarscore.bytes, 1, oos.getBuffer())

        # b64data = base64.b64encode(oos.getBuffer())
        # print(b64data)

        reg_datas.append(oos.getBuffer())

        return Huya.wss_url, reg_datas

    @staticmethod
    def decode_msg(data):
        class User(tarscore.struct):
            @staticmethod
            def readFrom(ios):
                return ios.read(tarscore.string, 2, False).decode("utf8")

        class DColor(tarscore.struct):
            @staticmethod
            def readFrom(ios):
                return ios.read(tarscore.int32, 0, False)

        name = ""
        content = ""
        color = 16777215
        msgs = []
        ios = tarscore.TarsInputStream(data)
        if ios.read(tarscore.int32, 0, False) == 7:
            ios = tarscore.TarsInputStream(ios.read(tarscore.bytes, 1, False))
            if ios.read(tarscore.int64, 1, False) == 1400:
                ios = tarscore.TarsInputStream(ios.read(tarscore.bytes, 2, False))
                name = ios.read(User, 0, False)  # username
                content = ios.read(tarscore.string, 3, False).decode("utf8")  # content
                color = ios.read(DColor, 6, False)  # danmaku color
                if color == -1:
                    color = 16777215
        if name != "":
            msg = {"name": name, "color": f"{color}", "content": content, "msg_type": "danmaku"}
            # else:
            #     msg = {"name": "", "content": "", "msg_type": "other"}
            msgs.append(msg)
        return msgs
