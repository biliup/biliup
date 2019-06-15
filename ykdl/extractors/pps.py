#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content, add_header
from ykdl.util.match import match1
from ykdl.compact import urlencode

from .iqiyi.util import get_macid

import json
import time
import random
import hashlib


def gsign(params):
    s = []
    for key in sorted(params.keys()):
        s.append('{}:{}'.format(key, params[key]))
    s.append('w!ytDgy#lEXWoJmN4HPf')
    s = ''.join(s)
    return hashlib.sha1(s.encode('utf8')).hexdigest()

def getlive(uid, rate='source'):
    tm = int(time.time())
    api = 'https://m-glider-xiu.pps.tv/v2/stream/get.json'
    params = {
        'type_id': 1,
        'vid': 1,
        'anchor_id': uid,
        'app_key': 'show_web_h5',
        'version': '1.0.0',
        'platform': '1_10_101',
        'time': tm,
        'netstat': 'wifi',
        'device_id': get_macid(),
        'bit_rate_type': rate,
        'protocol': 5,
    }
    params['sign'] = gsign(params)
    data = urlencode(params)
    if not isinstance(data, bytes):
        data = data.encode()
    html = get_content(api, data=data)
    return json.loads(html)

class PPS(VideoExtractor):
    name = u"奇秀（Qixiu)"

    ids = ['TD', 'HD', 'SD']
    rate_2_id = {
        'source': 'TD',
        'high': 'HD',
        'smooth': 'SD'
    }
    rate_2_profile = {
        'source': u'超清',
        'high': u'高清',
        'smooth': u'标清'
    }

    def prepare(self):
        info = VideoInfo(self.name, True)
        html = get_content(self.url)
        self.vid = match1(html, '"user_id":"([^"]+)",')
        title = json.loads(match1(html, '"room_name":("[^"]*"),'))
        artist = json.loads(match1(html, '"nick_name":("[^"]+"),'))
        info.title = u'{} - {}'.format(title, artist)
        info.artist = artist

        def get_live_info(rate='source'):
            data = getlive(self.vid, rate)
            self.logger.debug('data:\n' + str(data))
            if data['code'] != 'A00000':
                return data.get('msg')

            data = data['data']
            url = data.get('https_flv') or data.get('flv') or data.get('rtmp')
            if url:
                url = url.replace('rtmp://', 'http://')
                ran = random.randrange(1e4)
                if '?' in url:
                    url = '{}&ran={}'.format(url, ran)
                else:
                    url = '{}?ran={}'.format(url, ran)
                stream_profile = self.rate_2_profile[rate]
                stream_id = self.rate_2_id[rate]
                info.stream_types.append(stream_id)
                info.streams[stream_id] = {
                    'video_profile': stream_profile,
                    'container': 'flv',
                    'src' : [url],
                    'size': float('inf')
                }

            error_msges = []
            if rate == 'source':
                rate_list = data['rate_list']
                if 'source' in rate_list:
                    rate_list.remove('source')
                    for rate in rate_list:
                        error_msg = get_live_info(rate)
                        if error_msg:
                            error_msges.append(error_msg)
            if error_msges:
                return ', '.join(error_msges)

        error_msg = get_live_info()
        if error_msg:
            self.logger.debug('error_msg:\n' + error_msg)
        assert len(info.stream_types), error_msg or 'can\'t play this live video!!'
        info.stream_types = sorted(info.stream_types, key=self.ids.index)

        return info

site = PPS()
