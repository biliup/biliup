import json
import re
from struct import pack

import aiohttp

from biliup.plugins import match1


class Douyu:
    wss_url = 'wss://danmuproxy.douyu.com:8506/'
    heartbeat = b"\x14\x00\x00\x00\x14\x00\x00\x00\xb1\x02\x00\x00\x74\x79\x70\x65\x40\x3d\x6d\x72\x6b\x6c\x2f\x00"
    heartbeatInterval = 30

    @staticmethod
    async def get_ws_info(url):
        async with aiohttp.ClientSession() as session:
            if 'm.douyu.com' in url:
                room_no = url.split('m.douyu.com/')[1].split('/')[0].split('?')[0]
                async with session.get(f'https://www.douyu.com/{room_no}', timeout=5) as resp:
                    room_page = await resp.text()
            else:
                async with session.get(url, timeout=5) as resp:
                    room_page = await resp.text()
        room_id = match1(room_page, r'\$ROOM\.room_id\s*=\s*(\d+)')
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
        msgs = []
        for msg in re.findall(b'(type@=.*?)\x00', data):
            msga = {}
            try:
                msg = msg.replace(b'@=', b'":"').replace(b'/', b'","')
                msg = msg.replace(b'@A', b'@').replace(b'@S', b'/')
                msg = json.loads((b'{"' + msg[:-2] + b'}').decode('utf8', 'ignore'))
                msga['name'] = msg.get('nn', '')
                msga['content'] = msg.get('txt', '')
                msga['msg_type'] = {'dgb': 'gift', 'chatmsg': 'danmaku',
                                   'uenter': 'enter'}.get(msg['type'], 'other')
                msga['col'] = msg.get('col', '0')
                msgs.append(msga)
            except Exception as Error:
                # print(f"decode_msg：捕获到异常：{Error}")
                pass
        return msgs
