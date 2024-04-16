# 抖音的弹幕录制参考了 https://github.com/LyzenX/DouyinLiveRecorder 和 https://github.com/YunzhiYike/live-tool
# 2023.07.14：KNaiFen：这部分代码参考了https://github.com/SmallPeaches/DanmakuRender

import gzip

import aiohttp
import json
from urllib.parse import unquote
from biliup.config import config
from .douyin_util.dy_pb2 import ChatMessage, PushFrame, Response
from .. import match1, random_user_agent
from google.protobuf import json_format


class Douyin:
    headers = {
        'user-agent': random_user_agent(),
        'Referer': 'https://live.douyin.com/',
        'Cookie': config.get('user', {}).get('douyin_cookie', '')
    }
    heartbeat = b':\x02hb'
    heartbeatInterval = 10

    @staticmethod
    async def get_ws_info(url, context):
        async with aiohttp.ClientSession() as session:
            from biliup.plugins.douyin import DouyinUtils
            if "/user/" in url:
                async with session.get(url, headers=Douyin.headers, timeout=5) as resp:
                    user_page = await resp.text()
                    user_page_data = unquote(
                        user_page.split('<script id="RENDER_DATA" type="application/json">')[1].split('</script>')[0])
                    room_id = match1(user_page_data, r'"web_rid":"([^"]+)"')
            else:
                room_id = url.split('douyin.com/')[1].split('/')[0].split('?')[0]

            if room_id[0] == "+":
                room_id = room_id[1:]

            if "ttwid" not in Douyin.headers['Cookie']:
                Douyin.headers['Cookie'] = f'ttwid={DouyinUtils.get_ttwid()};{Douyin.headers["Cookie"]}'

            async with session.get(
                    DouyinUtils.build_request_url(f"https://live.douyin.com/webcast/room/web/enter/?web_rid={room_id}"),
                    headers=Douyin.headers, timeout=5) as resp:
                room_info = json.loads(await resp.text())['data']['data'][0]
                url = DouyinUtils.build_request_url(
                    f"wss://webcast3-ws-web-lf.douyin.com/webcast/im/push/v2/?room_id={room_info['id_str']}&compress=gzip&signature=00000000")
                return url, []

    @staticmethod
    def decode_msg(data):
        wss_package = PushFrame()
        wss_package.ParseFromString(data)
        log_id = wss_package.logId
        decompressed = gzip.decompress(wss_package.payload)
        payload_package = Response()
        payload_package.ParseFromString(decompressed)

        ack = None
        if payload_package.needAck:
            obj = PushFrame()
            obj.payloadType = 'ack'
            obj.logId = log_id
            obj.payloadType = payload_package.internalExt
            ack = obj.SerializeToString()

        msgs = []
        for msg in payload_package.messagesList:
            if msg.method == 'WebcastChatMessage':
                chat_message = ChatMessage()
                chat_message.ParseFromString(msg.payload)
                data = json_format.MessageToDict(chat_message, preserving_proto_field_name=True)
                # name = data['user']['nickName']
                content = data['content']
                # msg_dict = {"time": now, "name": name, "content": content, "msg_type": "danmaku", "color": "ffffff"}
                msg_dict = {"content": content, "msg_type": "danmaku"}
                msgs.append(msg_dict)

        return msgs, ack
