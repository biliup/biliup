#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.match import match1
from ykdl.util.html import get_content
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo

import time
import json

class LongzhuLive(VideoExtractor):
    name = u'Longzhu Live (龙珠直播)'

    supported_stream_types = ['SD', 'HD', 'TD', 'BD', 'Phone']
    types_2_profile = {'SD': u'标清', 'HD':u'高清', 'TD':u'超清', 'BD':u'原画', 'Phone':u'手机'}

    def prepare(self):
        info = VideoInfo(self.name, True)

        if not self.vid:
            html = get_content(self.url)
            self.vid = match1(html, 'roomid: (\d+)', '__ROOMID = \'(\d+)\';', '"RoomId":(\d+)')
            info.title = match1(html, '"title":"([^"]+)', '<title>([^>]+)<')
            info.artist = match1(html, '"Name":"([^"]+)')

        api_url = 'http://livestream.plu.cn/live/getlivePlayurl?roomId={}&{}'.format(self.vid, int(time.time()))

        data = json.loads(get_content(api_url))['playLines'][0]['urls'] #don't know index 1

        for i in data:
            if i['ext'] == 'flv':
                stream_id = self.supported_stream_types[i['rateLevel'] -1]
                info.stream_types.append(stream_id)
                info.streams[stream_id] = {'container': 'flv', 'video_profile': self.types_2_profile[stream_id], 'src' : [i['securityUrl']], 'size': 0}

        #sort stream_types
        types = self.supported_stream_types
        types.reverse()
        info.stream_types = sorted(info.stream_types, key=types.index)
        return info

site = LongzhuLive()
