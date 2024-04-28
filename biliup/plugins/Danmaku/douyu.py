import logging
import re
from struct import pack

import aiohttp

from biliup.plugins import match1

logger = logging.getLogger('biliup')


class Douyu:
    wss_url = 'wss://danmuproxy.douyu.com:8506/'
    heartbeat = b"\x14\x00\x00\x00\x14\x00\x00\x00\xb1\x02\x00\x00\x74\x79\x70\x65\x40\x3d\x6d\x72\x6b\x6c\x2f\x00"
    heartbeatInterval = 30
    msg_col = {'0': '16777215', '1': '16717077', '2': '2000880', '3': '8046667', '4': '16744192',
               '5': '10172916',
               '6': '16738740', '7': '16777215'}

    @staticmethod
    async def get_ws_info(url, context):
        async with aiohttp.ClientSession() as session:
            if 'm.douyu.com' in url:
                room_id = url.split('m.douyu.com/')[1].split('/')[0].split('?')[0]
            else:
                async with session.get(url, timeout=5) as resp:
                    room_page = await resp.text()
                    room_id = match1(room_page, r'\$ROOM\.room_id\s*=\s*(\d+)', r'apm_room_id\s*=\s*(\d+)')[0]
        reg_datas = []
        data = f'type@=loginreq/roomid@={room_id}/'
        s = pack('i', 9 + len(data)) * 2
        s += b'\xb1\x02\x00\x00'  # 689
        s += data.encode('ascii') + b'\x00'
        reg_datas.append(s)
        data = f'type@=joingroup/rid@={room_id}/gid@=-9999/'
        s = pack('i', 9 + len(data)) * 2
        s += b'\xb1\x02\x00\x00'  # 689
        s += data.encode('ascii') + b'\x00'
        reg_datas.append(s)
        return Douyu.wss_url, reg_datas

    @staticmethod
    def decode_msg(data):
        def stt_loads(stt_str):
            if '/' in stt_str:
                stt_items = stt_str.split('/')
                stt_list = []
                stt_dict = {}
                for stt_item in stt_items:
                    if stt_item == '':
                        continue
                    stt_item_decode = stt_loads(stt_item)
                    if type(stt_item_decode) is dict:
                        stt_dict.update(stt_item_decode)
                    else:
                        stt_list.append(stt_item_decode)
                if len(stt_list) > 0:
                    return stt_list
                else:
                    return stt_dict
            elif '@=' in stt_str:
                key, value = stt_str.split('@=')
                return {stt_loads(key): stt_loads(value)}
            else:
                return stt_str.replace("@A", "@").replace('@S', '/')

        msgs = []
        for msg in re.findall(b'(type@=.*?)\x00', data):
            try:
                msg = stt_loads(msg.decode('utf-8'))
                if type(msg) is dict:
                    msgs.append({
                        'name': msg.get('nn', ''),
                        'content': msg.get('txt', ''),
                        'msg_type': {
                            'dgb': 'gift',
                            'chatmsg': 'danmaku',
                            'uenter': 'enter'
                        }.get(msg['type'], 'other'),
                        'color': Douyu.msg_col.get(msg.get('col')),
                    })
            except:
                logger.warning(f"{Douyu.__name__}: 弹幕接收异常", exc_info=True)
        return msgs
