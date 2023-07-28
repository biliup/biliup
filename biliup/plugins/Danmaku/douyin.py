# 抖音的弹幕录制参考了 https://github.com/LyzenX/DouyinLiveRecorder 和 https://github.com/YunzhiYike/live-tool
# 2023.07.14：KNaiFen：这部分代码参考了https://github.com/SmallPeaches/DanmakuRender

import gzip
import time
import re
from urllib.parse import unquote

import requests
import json

from google.protobuf import json_format

from .douyin_util.dy_pb2 import PushFrame, Response, ChatMessage  # 反序列化后导入


class Douyin:
    headers = {
        'authority': 'live.douyin.com',
        'user-agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.5112.81 Safari/537.36 Edg/104.0.1293.54',
    }
    heartbeat = b':\x02hb'
    heartbeatInterval = 10

    @staticmethod
    async def get_ws_info(url):
        web_rid = url.split('/')[-1]
        if not Douyin.headers.get('cookie'):
            response = requests.get(f'https://live.douyin.com/{web_rid}', headers=Douyin.headers, timeout=5)
            Douyin.headers['cookie'] = '__ac_nonce=' + response.cookies.get('__ac_nonce')
            response = requests.get(f'https://live.douyin.com/{web_rid}', headers=Douyin.headers, timeout=5)
            Douyin.headers['cookie'] += '; ttwid=' + response.cookies.get('ttwid')
        else:
            response = requests.get(f'https://live.douyin.com/{web_rid}', headers=Douyin.headers, timeout=5)

        render_data = re.findall(r"<script id=\"RENDER_DATA\" type=\"application/json\">.*?</script>", response.text)[0]
        render_data = unquote(render_data)
        render_data = re.sub(r'(<script.*?>|</script>)', '', render_data)
        data = json.loads(render_data)
        real_rid = data['app']['initialState']['roomStore']['roomInfo']['roomId']
        user_unique_id = data['app']['odin']['user_unique_id']

        url = f"wss://webcast3-ws-web-lf.douyin.com/webcast/im/push/v2/?app_name=douyin_web&version_code=180800&webcast_sdk_version=1.3.0&update_version_code=1.3.0&compress=gzip&internal_ext=internal_src:dim|wss_push_room_id:{real_rid}|wss_push_did:{user_unique_id}|dim_log_id:2023011316221327ACACF0E44A2C0E8200|fetch_time:${int(time.time())}123|seq:1|wss_info:0-1673598133900-0-0|wrds_kvs:WebcastRoomRankMessage-1673597852921055645_WebcastRoomStatsMessage-1673598128993068211&cursor=u-1_h-1_t-1672732684536_r-1_d-1&host=https://live.douyin.com&aid=6383&live_id=1&did_rule=3&debug=false&endpoint=live_pc&support_wrds=1&im_path=/webcast/im/fetch/&device_platform=web&cookie_enabled=true&screen_width=1228&screen_height=691&browser_language=zh-CN&browser_platform=Mozilla&browser_name=Mozilla&browser_version=5.0%20(Windows%20NT%2010.0;%20Win64;%20x64)%20AppleWebKit/537.36%20(KHTML,%20like%20Gecko)%20Chrome/100.0.4896.75%20Safari/537.36&browser_online=true&tz_name=Asia/Shanghai&identity=audience&room_id={real_rid}&heartbeatDuration=0&signature=00000000"
        return url, []

    @staticmethod
    def decode_msg(data):
        wssPackage = PushFrame()
        wssPackage.ParseFromString(data)
        logid = wssPackage.logId
        decompressed = gzip.decompress(wssPackage.payload)
        payloadPackage = Response()
        payloadPackage.ParseFromString(decompressed)

        ack = None
        if payloadPackage.needAck:
            obj = PushFrame()
            obj.payloadType = 'ack'
            obj.logId = logid
            obj.payloadType = payloadPackage.internalExt
            ack = obj.SerializeToString()

        msgs = []
        for msg in payloadPackage.messagesList:
            if msg.method == 'WebcastChatMessage':
                chatMessage = ChatMessage()
                chatMessage.ParseFromString(msg.payload)
                data = json_format.MessageToDict(chatMessage, preserving_proto_field_name=True)
                # name = data['user']['nickName']
                content = data['content']
                # msg_dict = {"time": now, "name": name, "content": content, "msg_type": "danmaku", "color": "ffffff"}
                msg_dict = {"content": content, "msg_type": "danmaku"}
                msgs.append(msg_dict)

        return msgs, ack
