#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.compact import compact_bytes

import uuid
import base64
import json
import time
import random

SID = uuid.uuid4().hex.upper()

class Huajiao(VideoExtractor):
    name = u'huajiao (花椒直播)'

    def prepare(self):
        info = VideoInfo(self.name, True)
        html = get_content(self.url)
        t_a = match1(html, '"keywords" content="([^"]+)')
        info.title = t_a.split(',')[0]
        info.artist = t_a.split(',')[1]

        replay_url = match1(html, '"m3u8":"([^"]+)')
        if replay_url:
            replay_url = replay_url.replace('\/','/')
            info.live = False
            info.stream_types.append('current')
            info.streams['current'] = {'container': 'm3u8', 'video_profile': 'current', 'src' : [replay_url], 'size': float('inf')}
            return info

        self.vid = match1(html, '"sn":"([^"]+)')
        channel = match1(html, '"channel":"([^"]+)')
        api_url = 'http://g2.live.360.cn/liveplay?stype=flv&channel={}&bid=huajiao&sn={}&sid={}&_rate=xd&ts={}&r={}&_ostype=flash&_delay=0&_sign=null&_ver=13'.format(channel, self.vid, SID, time.time(),random.random())
        encoded_json = get_content(api_url)
        decoded_json = base64.decodestring(compact_bytes(encoded_json[0:3]+ encoded_json[6:], 'utf-8')).decode('utf-8')
        video_data = json.loads(decoded_json)
        live_url = video_data['main']
        info.live = True
        info.stream_types.append('current')
        info.streams['current'] = {'container': 'flv', 'video_profile': 'current', 'src' : [live_url], 'size': float('inf')}
        return info

site = Huajiao()
