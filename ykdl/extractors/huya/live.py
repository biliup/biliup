#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content, add_header
from ykdl.util.match import match1, matchall

import json
import random

class HuyaLive(VideoExtractor):
    name = u"Huya Live (虎牙直播)"

    def prepare(self):
        info = VideoInfo(self.name)

        html  = get_content(self.url)

        json_script = match1(html, '"stream": ({.+?})\s*};')
        assert json_script, "live video is offline"
        data = json.loads(json_script)
        assert data['status'] == 200, data['msg']

        room_info = data['data'][0]['gameLiveInfo']
        info.title = room_info['roomName']

        stream_info = random.choice(data['data'][0]['gameStreamInfoList'])
        sFlvUrl = stream_info['sFlvUrl']
        sStreamName = stream_info['sStreamName']
        sFlvUrlSuffix = stream_info['sFlvUrlSuffix']
        sFlvAntiCode = stream_info['sFlvAntiCode']
        flv_url = '{}/{}.{}?{}'.format(sFlvUrl, sStreamName, sFlvUrlSuffix, sFlvAntiCode)

        info.stream_types.append("current")
        info.streams["current"] = {'container': 'mp4', 'video_profile': "current", 'src': [flv_url], 'size' : float('inf')}
        return info

site = HuyaLive()
