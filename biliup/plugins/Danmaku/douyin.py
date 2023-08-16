# 抖音的弹幕录制参考了 https://github.com/LyzenX/DouyinLiveRecorder 和 https://github.com/YunzhiYike/live-tool
# 2023.07.14：KNaiFen：这部分代码参考了https://github.com/SmallPeaches/DanmakuRender

import gzip
import time
import requests
import json
from urllib.parse import unquote
from biliup.config import config
from .douyin_util.dy_pb2 import ChatMessage, PushFrame, Response
from .. import match1
from google.protobuf import json_format


class Douyin:
    headers = {
        'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
        'Accept-Encoding': 'gzip, deflate',
        'Accept-Language': 'zh-CN,zh;q=0.8,en-US;q=0.5,en;q=0.3',
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4515.159 Safari/537.36',
        'Referer': 'https://live.douyin.com/',
        'Cookie': config.get('user', {}).get('douyin_cookie', '')
    }
    heartbeat = b':\x02hb'
    heartbeatInterval = 10

    @staticmethod
    async def get_ws_info(url):
        if "/user/" in url:
            user_page = requests.get(url, headers=Douyin.headers, timeout=5).text
            user_page_data = unquote(
                user_page.split('<script id="RENDER_DATA" type="application/json">')[1].split('</script>')[0])
            room_id = match1(user_page_data, r'"web_rid":"([^"]+)"')
        else:
            room_id = url.split('douyin.com/')[1].split('/')[0].split('?')[0]
        if room_id[0] == "+":
            room_id = room_id[1:]
        if room_id.isdigit():
            room_id = f"+{room_id}"

        response = requests.get(f'https://live.douyin.com/{room_id}', headers=Douyin.headers, timeout=5)
        if "ttwid" not in Douyin.headers['Cookie']:
            Douyin.headers['Cookie'] = f'ttwid={response.cookies.get("ttwid", "")};{Douyin.headers["Cookie"]}'
        data = json.loads(
            unquote(response.text.split('<script id="RENDER_DATA" type="application/json">')[1].split('</script>')[0]))
        real_rid = data['app']['initialState']['roomStore']['roomInfo']['roomId']
        user_unique_id = data['app']['odin']['user_unique_id']

        url = f"wss://webcast3-ws-web-lf.douyin.com/webcast/im/push/v2/?app_name=douyin_web&version_code=180800&webcast_sdk_version=1.3.0&update_version_code=1.3.0&compress=gzip&internal_ext=internal_src:dim|wss_push_room_id:{real_rid}|wss_push_did:{user_unique_id}|dim_log_id:2023011316221327ACACF0E44A2C0E8200|fetch_time:${int(time.time())}123|seq:1|wss_info:0-1673598133900-0-0|wrds_kvs:WebcastRoomRankMessage-1673597852921055645_WebcastRoomStatsMessage-1673598128993068211&cursor=u-1_h-1_t-1672732684536_r-1_d-1&host=https://live.douyin.com&aid=6383&live_id=1&did_rule=3&debug=false&endpoint=live_pc&support_wrds=1&im_path=/webcast/im/fetch/&device_platform=web&cookie_enabled=true&screen_width=1228&screen_height=691&browser_language=zh-CN&browser_platform=Win32&browser_name=Mozilla&browser_version=5.0%20Mozilla/5.0%20(Windows%20NT%2010.0;%20Win64;%20x64)%20AppleWebKit/537.36%20(KHTML,%20like%20Gecko)%20Chrome/92.0.4515.159%20Safari/537.36&browser_online=true&tz_name=Asia/Shanghai&identity=audience&room_id={real_rid}&heartbeatDuration=0&signature=00000000"
        return url, []

    @staticmethod
    def decode_msg(data):
        wss_package = PushFrame()
        wss_package.ParseFromString(data)
        logid = wss_package.logId
        decompressed = gzip.decompress(wss_package.payload)
        payload_package = Response()
        payload_package.ParseFromString(decompressed)

        ack = None
        if payload_package.needAck:
            obj = PushFrame()
            obj.payloadType = 'ack'
            obj.logId = logid
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
