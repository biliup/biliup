# 抖音的弹幕录制参考了 https://github.com/LyzenX/DouyinLiveRecorder 和 https://github.com/YunzhiYike/live-tool
# 2023.07.14：KNaiFen：这部分代码参考了https://github.com/SmallPeaches/DanmakuRender

from datetime import datetime
import threading
import asyncio
import gzip
import time
import re
import requests
import urllib
import json
import os
import queue

import websocket
import lxml.etree as etree
from google.protobuf import json_format

from .douyin_util.dy_pb2 import PushFrame, Response, ChatMessage  # 反序列化后导入


class Douyin:
    headers = {
        'authority': 'live.douyin.com',
        'user-agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.5112.81 Safari/537.36 Edg/104.0.1293.54',
    }
    heartbeatInterval = 30

    def __init__(self, url, filename):
        self.__ws = None
        self.__stop = False
        self.__web_rid = url.split('/')[-1]
        self.__dm_queue = queue.Queue()
        self.ttwid = None
        self.__starttime = time.time()
        self.__filename = os.path.splitext(filename)[0] + '.xml'
        self.__filename_video_suffix = filename
        self.__url = ''

        if not os.path.exists(self.__filename):
            with open(self.__filename, mode='w') as f:
                f.write("<?xml version='1.0' encoding='UTF-8'?>\n"
                        "<i xmlns:ns0='http://www.w3.org/1999/XSL/Transform'>\n"
                        "</i>"
                        )

        if not self.headers.get('cookie'):
            try:
                response = requests.get(f'https://live.douyin.com/{self.__web_rid}', headers=self.headers, timeout=5)
                self.headers.update({'cookie': '__ac_nonce=' + response.cookies.get('__ac_nonce')})

                response = requests.get(f'https://live.douyin.com/{self.__web_rid}', headers=self.headers, timeout=5)
                self.headers['cookie'] += '; ttwid=' + response.cookies.get('ttwid')
            except Exception as e:
                # logging.exception(e)
                raise Exception('获取抖音cookies错误.')

        if len(self.__web_rid) == 19:
            self.real_rid = self.__web_rid
        else:
            try:
                resp = self._get_response_douyin()
                self.real_rid = resp['app']['initialState']['roomStore']['roomInfo']['roomId']
            except:
                raise Exception('房间号错误.')

    def _get_response_douyin(self):
        text = requests.get(f'https://live.douyin.com/{self.__web_rid}', headers=self.headers, timeout=5).text
        render_data = re.findall(r"<script id=\"RENDER_DATA\" type=\"application/json\">.*?</script>", text)[0]
        data = urllib.parse.unquote(render_data)
        data = re.sub(r'(<script.*?>|</script>)', '', data)
        data = json.loads(data)

        return data

    def get_danmu_ws_url(self):
        resp = self._get_response_douyin()
        user_unique_id = resp['app']['odin']['user_unique_id']
        return f"wss://webcast3-ws-web-lf.douyin.com/webcast/im/push/v2/?app_name=douyin_web&version_code=180800&webcast_sdk_version=1.3.0&update_version_code=1.3.0&compress=gzip&internal_ext=internal_src:dim|wss_push_room_id:{self.real_rid}|wss_push_did:{user_unique_id}|dim_log_id:2023011316221327ACACF0E44A2C0E8200|fetch_time:${int(time.time())}123|seq:1|wss_info:0-1673598133900-0-0|wrds_kvs:WebcastRoomRankMessage-1673597852921055645_WebcastRoomStatsMessage-1673598128993068211&cursor=u-1_h-1_t-1672732684536_r-1_d-1&host=https://live.douyin.com&aid=6383&live_id=1&did_rule=3&debug=false&endpoint=live_pc&support_wrds=1&im_path=/webcast/im/fetch/&device_platform=web&cookie_enabled=true&screen_width=1228&screen_height=691&browser_language=zh-CN&browser_platform=Mozilla&browser_name=Mozilla&browser_version=5.0%20(Windows%20NT%2010.0;%20Win64;%20x64)%20AppleWebKit/537.36%20(KHTML,%20like%20Gecko)%20Chrome/100.0.4896.75%20Safari/537.36&browser_online=true&tz_name=Asia/Shanghai&identity=audience&room_id={self.real_rid}&heartbeatDuration=0&signature=00000000"

    async def start(self):
        self.__ws = websocket.WebSocketApp(
            url=self.get_danmu_ws_url(),
            header=self.headers,
            cookie=self.headers.get('cookie'),
            on_message=self._onMessage,
            on_error=self._onError,
            on_open=self._onOpen,
        )
        t = threading.Thread(target=self.__ws.run_forever, daemon=True)
        t.start()

        parser = etree.XMLParser(recover=True)
        tree = etree.parse(self.__filename, parser=parser)
        root = tree.getroot()

        def write_file(filename):
            with open(filename, "wb") as f:
                etree.indent(root, "\t")
                tree.write(f, encoding="UTF-8", xml_declaration=True, pretty_print=True)

        msg_i = 0
        while not self.__stop:
            try:
                m = self.__dm_queue.get_nowait()
            except Exception as Error:
                await asyncio.sleep(1)
                continue

            try:
                if m['msg_type'] == 'danmaku':
                    d = etree.SubElement(root, 'd')
                    msg_time = format(time.time() - self.__starttime, '.3f')
                    d.set('p', f"{msg_time},1,25,16777215,0,0,0,0")
                    d.text = m["content"]

                    if msg_i >= 1:
                        write_file(self.__filename)
                        msg_i = 0
                    else:
                        msg_i = msg_i + 1
            except Exception as Error:
                pass



    async def stop(self):
        self.__stop = True
        self.__ws.close()
        if not (os.path.exists(f"{self.__filename_video_suffix}.part") or
                os.path.exists(f"{self.__filename_video_suffix}")):
            os.remove(self.__filename)

    def _onOpen(self, ws):
        t = threading.Thread(target=self._heartbeat, args=(ws,), daemon=True)
        t.start()

    def _onMessage(self, ws: websocket.WebSocketApp, message: bytes):
        wssPackage = PushFrame()
        wssPackage.ParseFromString(message)
        logid = wssPackage.logId
        decompressed = gzip.decompress(wssPackage.payload)
        payloadPackage = Response()
        payloadPackage.ParseFromString(decompressed)

        # 发送ack包
        if payloadPackage.needAck:
            obj = PushFrame()
            obj.payloadType = 'ack'
            obj.logId = logid
            obj.payloadType = payloadPackage.internalExt
            data = obj.SerializeToString()
            ws.send(data, websocket.ABNF.OPCODE_BINARY)
        # 处理消息
        for msg in payloadPackage.messagesList:
            # now = datetime.now()
            if msg.method == 'WebcastChatMessage':
                chatMessage = ChatMessage()
                chatMessage.ParseFromString(msg.payload)
                data = json_format.MessageToDict(chatMessage, preserving_proto_field_name=True)
                name = data['user']['nickName']
                content = data['content']
                # msg_dict = {"time": now, "name": name, "content": content, "msg_type": "danmaku", "color": "ffffff"}
                msg_dict = {"content": content, "msg_type": "danmaku"}
                # print(msg_dict)
                self.__dm_queue.put_nowait(msg_dict)


    def _heartbeat(self, ws: websocket.WebSocketApp):
        while not self.__stop:
            obj = PushFrame()
            obj.payloadType = 'hb'
            data = obj.SerializeToString()
            ws.send(data, websocket.ABNF.OPCODE_BINARY)
            time.sleep(10)

    def _onError(self, ws, error):
        raise error

