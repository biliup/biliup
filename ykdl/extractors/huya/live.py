#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.compact import unescape
from ykdl.util.html import get_content, add_header
from ykdl.util.match import match1, matchall

import json
import random

class HuyaLive(VideoExtractor):
    name = u"Huya Live (虎牙直播)"

    def prepare(self):
        info = VideoInfo(self.name, True)

        html  = get_content(self.url)

        json_stream = match1(html, '"stream": ({.+?})\s*};')
        assert json_stream, "live video is offline"
        data = json.loads(json_stream)
        assert data['status'] == 200, data['msg']

        room_info = data['data'][0]['gameLiveInfo']
        info.title = u'{}「{} - {}」'.format(room_info['roomName'], room_info['nick'], room_info['introduction'])
        info.artist = room_info['nick']

        stream_info = random.choice(data['data'][0]['gameStreamInfoList'])
        sHlsUrl = stream_info['sHlsUrl']
        sStreamName = stream_info['sStreamName']
        sHlsUrlSuffix = stream_info['sHlsUrlSuffix']
        sHlsAntiCode = stream_info['sHlsAntiCode']
        hls_url = u'{}/{}.{}?{}'.format(sHlsUrl, sStreamName, sHlsUrlSuffix, sHlsAntiCode)

        info.stream_types.append("current")
        info.streams["current"] = {
            'container': 'm3u8',
            'video_profile': 'current',
            'src': [unescape(hls_url)],
            'size' : float('inf')
        }
        return info

site = HuyaLive()
