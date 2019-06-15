#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content
from ykdl.util.match import match1

import json

class BiliVC(VideoExtractor):
    name = u'哔哩哔哩 小视频 (Bili VC)'

    def prepare(self):
        info = VideoInfo(self.name)

        self.vid = match1(self.url, 'video/(\d+)')

        api_url = 'https://api.vc.bilibili.com/clip/v1/video/detail?video_id={}'.format(self.vid)

        video_data = json.loads(get_content(api_url))

        info.title = video_data['data']['item']['description']
        info.artist = video_data['data']['user']['name']

        info.stream_types.append('current')
        info.streams['current'] = {'container': 'mp4', 'src' : [video_data['data']['item']['video_playurl']], 'size': int(video_data['data']['item']['video_size'])}

        return info

site = BiliVC()

