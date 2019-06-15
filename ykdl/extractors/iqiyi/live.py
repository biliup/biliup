#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content, add_header
from ykdl.util.match import match1
from ykdl.compact import urlencode

from .util import get_random_str, get_macid, cmd5x

import json
import time


def getlive(vid):
    tm = time.time()
    host = 'https://live.video.iqiyi.com'
    params = {
        'lp': vid,
        'src': '01010031010000000000',
        'uid': '',
        'rateVers': 'PC_QIYI_3',
        'k_uid': get_macid(24),
        'qdx': 'n',
        'qdv': 3,
        'qd_v': 1,
        'dfp': get_random_str(66),
        'v': 1,
        'k_err_retries': 0,
        'tm': int(tm + 1),
    }
    src = '/live?{}'.format(urlencode(params))
    vf = cmd5x(src)
    req_url = '{}{}&vf={}'.format(host, src, vf)
    st = int(tm * 1000)
    et = int((tm + 1296000) * 1000)
    c_dfp = '__dfp={}@{}@{}'.format(params['dfp'], et, st)
    add_header('Cookie', c_dfp)
    html = get_content(req_url)
    return json.loads(html)

class IqiyiLive(VideoExtractor):
    name = u"爱奇艺直播 (IqiyiLive)"

    ids = ['4k','BD', 'TD', 'HD', 'SD', 'LD']
    type_2_id = {
        #'': '4k',
        'RESOLUTION_1080P': 'BD',
        'RESOLUTION_720P': 'TD',
        'HIGH_DEFINITION': 'HD',
        'SMOOTH': 'SD',
        #'': 'LD'
    }

    def prepare(self):
        info = VideoInfo(self.name, True)
        html = get_content(self.url)
        self.vid = match1(html, '"qipuId":(\d+),')
        title = match1(html, '"roomTitle":"([^"]+)",')
        artist = match1(html, '"anchorNickname":"([^"]+)",')
        info.title = u'{} - {}'.format(title, artist)
        info.artist = artist

        data = getlive(self.vid)
        self.logger.debug('data:\n' + str(data))
        assert data['code'] == 'A00000', data.get('msg', 'can\'t play this live video!!')
        data = data['data']

        for stream in data['streams']:
            # TODO: parse more format types.
            # Streams which use formatType 'TS' are slow,
            # and rolling playback use formatType 'HLFLV' with scheme 'hcdnlive://'.
            # Its host and path encoded as like:
            #   'AMAAAAD3PV2R2QI7MXRQ4L2BD5Y...'
            # the real url is:
            #   'https://hlslive.video.iqiyi.com/live/{hl_slid}.flv?{params}'
            # Request it, the response is a json data which contains CDN informations.
            if stream['formatType'] == 'TS':
                m3u8 = stream['url']
                # miswrote 'streamType' to 'steamType'
                stream_type = stream['steamType']
                stream_profile = stream['screenSize']
                stream_id = self.type_2_id[stream_type]
                info.stream_types.append(stream_id)
                info.streams[stream_id] = {
                    'video_profile': stream_profile,
                    'container': 'm3u8',
                    'src' : [m3u8],
                    'size': float('inf')
                }

        assert info.stream_types, 'can\'t play this live video!!'
        if len(info.stream_types) == 1:
            info.streams['current'] = info.streams.pop(info.stream_types[0])
            info.stream_types[0] = 'current'
        else:
            info.stream_types = sorted(info.stream_types, key=self.ids.index)

        return info

site = IqiyiLive()
