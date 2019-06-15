#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo

import json

class HuajiaoVideo(VideoExtractor):
    name = u'huajiao video (花椒小视频)'

    def prepare(self):
        info = VideoInfo(self.name, True)
        html = get_content(self.url)

        self.vid = match1(self.url, 'vid=(\d+)')

        video_data = json.loads("[" + match1(html, '_DATA.list = \[([^\[]+)];') + "]")

        if self.vid:
            for data in video_data:
                if data['vid']  == self.vid:
                    break
        else:
            data = video_data[0]
        assert 'video_url' in data, "No video found!!"
        info.artist = data['user_name']
        info.title = data['video_name']
        info.stream_types.append('current')
        info.streams['current'] = {'container': 'mp4', 'src': [data['video_url']], 'size' : 0}
        return info

site = HuajiaoVideo()
