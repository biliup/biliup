#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content
import json

class Yizhibo(VideoExtractor):
    name = u'Yizhibo (一直播)'

    def prepare(self):
        info = VideoInfo(self.name)
        info.live = True
        self.vid = self.url[self.url.rfind('/')+1:].split(".")[0]
        json_request_url = 'http://www.yizhibo.com/live/h5api/get_basic_live_info?scid={}'.format(self.vid)
        content = json.loads(get_content(json_request_url))
        assert content['result'] == 1, "Error : {}".format(content['result'])
        info.title = content['data']['live_title']
        info.artist = content['data']['nickname']
        info.streams['current'] = {'container': 'm3u8', 'video_profile': 'current', 'src' : [content['data']['play_url']], 'size': float('inf')}
        info.stream_types.append('current')
        return info

site = Yizhibo()
