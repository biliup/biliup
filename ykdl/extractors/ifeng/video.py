#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content

import json


class IfengVideo(VideoExtractor):
    name = u'凤凰视频 (ifeng video)'

    def prepare(self):
        info = VideoInfo(self.name)
        self.vid = self.url[-13: -6]
        info.title = self.name + '-' + self.vid
        api_url = 'http://tv.ifeng.com/html5/{}/video.json'.format(self.vid)
        data = json.loads(get_content(api_url)[12:])
        if 'bqSrc' in data:
            info.stream_types.append('SD')
            info.streams['SD'] = {'container': 'mp4', 'video_profile': u'标清', 'src' : [data['bqSrc']], 'size': 0}
        if 'gqSrc' in data:
            info.stream_types.append('HD')
            info.streams['HD'] = {'container': 'mp4', 'video_profile': u'高清', 'src' : [data['gqSrc']], 'size': 0}
        return info
site = IfengVideo()
