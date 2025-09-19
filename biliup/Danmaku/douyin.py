# 抖音的弹幕录制参考了 https://github.com/LyzenX/DouyinLiveRecorder 和 https://github.com/YunzhiYike/live-tool
# 2023.07.14：KNaiFen：这部分代码参考了https://github.com/SmallPeaches/DanmakuRender
# 2024.06.22: 添加来自 https://github.com/hua0512/stream-rec 修改后的 webmssdk.js，以计算 signature

import gzip
import logging

import aiohttp
from google.protobuf import json_format

from .douyin_util.dy_pb2 import ChatMessage, PushFrame, Response

logger = logging.getLogger('biliup')

class Douyin:
    from biliup.config import config

    headers = {
        # 'user-agent': random_user_agent(),
        'Referer': 'https://live.douyin.com/',
        'Cookie': config.get('user', {}).get('douyin_cookie', '')
    }
    heartbeat = b':\x02hb'
    heartbeatInterval = 10

    @staticmethod
    async def get_ws_info(url, context):
        async with aiohttp.ClientSession() as session:
            from biliup.plugins.douyin import DouyinUtils
            from .douyin_util import DouyinDanmakuUtils
            Douyin.headers['user-agent'] = DouyinUtils.DOUYIN_USER_AGENT

            if "ttwid" not in Douyin.headers['Cookie']:
                Douyin.headers['Cookie'] = f'ttwid={DouyinUtils.get_ttwid()};{Douyin.headers["Cookie"]}'

            USER_UNIQUE_ID = DouyinDanmakuUtils.get_user_unique_id()
            VERSION_CODE = 180800 # https://lf-cdn-tos.bytescm.com/obj/static/webcast/douyin_live/7697.782665f8.js -> a.ry
            WEBCAST_SDK_VERSION = "1.0.14-beta.0" # https://lf-cdn-tos.bytescm.com/obj/static/webcast/douyin_live/7697.782665f8.js -> ee.VERSION
            # logger.info(f"user_unique_id: {USER_UNIQUE_ID}")
            sig_params = {
                "live_id": "1",
                "aid": "6383",
                "version_code": VERSION_CODE,
                "webcast_sdk_version": WEBCAST_SDK_VERSION,
                "room_id": context['room_id'],
                "sub_room_id": "",
                "sub_channel_id": "",
                "did_rule": "3",
                "user_unique_id": USER_UNIQUE_ID,
                "device_platform": "web",
                "device_type": "",
                "ac": "",
                "identity": "audience"
            }
            signature = DouyinDanmakuUtils.get_signature(DouyinDanmakuUtils.get_x_ms_stub(sig_params))
            # logger.info(f"signature: {signature}")
            webcast5_params = {
                "room_id": context['room_id'],
                "compress": 'gzip',
                # "app_name": "douyin_web",
                "version_code": VERSION_CODE,
                "webcast_sdk_version": WEBCAST_SDK_VERSION,
                # "update_version_code": "1.0.14-beta.0",
                # "cookie_enabled": "true",
                # "screen_width": "1920",
                # "screen_height": "1080",
                # "browser_online": "true",
                # "tz_name": "Asia/Shanghai",
                # "cursor": "t-1718899404570_r-1_d-1_u-1_h-7382616636258522175",
                # "internal_ext": "internal_src:dim|wss_push_room_id:7382580251462732598|wss_push_did:7344670681018189347|first_req_ms:1718899404493|fetch_time:1718899404570|seq:1|wss_info:0-1718899404570-0-0|wrds_v:7382616716703957597",
                # "host": "https://live.douyin.com",
                "live_id": "1",
                "did_rule": "3",
                # "endpoint": "live_pc",
                # "support_wrds": "1",
                "user_unique_id": USER_UNIQUE_ID,
                # "im_path": "/webcast/im/fetch/",
                "identity": "audience",
                # "need_persist_msg_count": "15",
                # "insert_task_id": "",
                # "live_reason": "",
                # "heartbeatDuration": "0",
                "signature": signature,
            }
            wss_url = f"wss://webcast5-ws-web-lf.douyin.com/webcast/im/push/v2/?{'&'.join([f'{k}={v}' for k, v in webcast5_params.items()])}"
            url = DouyinUtils.build_request_url(wss_url)
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
                # print(content)
                # msg_dict = {"time": now, "name": name, "content": content, "msg_type": "danmaku", "color": "ffffff"}
                msg_dict = {"content": content, "msg_type": "danmaku"}
                msgs.append(msg_dict)

        return msgs, ack
