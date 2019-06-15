#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content, add_header
from ykdl.util.match import match1
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo

import json

class DouyutvVideo(VideoExtractor):
    name = u'斗鱼视频 (DouyuTV)'

    def prepare(self):
        info = VideoInfo(self.name)
        add_header('X-Requested-With', 'XMLHttpRequest')

        if self.url:
            self.vid = match1(self.url, 'show/(.*)')
        html = get_content(self.url)
        info.title = match1(html, u'title>(.+?)_斗鱼视频 - 最6的弹幕视频网站<')

        json_request_url = "https://vmobile.douyu.com/video/getInfo?vid={}".format(self.vid)
        html_content = get_content(json_request_url)
        self.logger.debug('video_data: ' + html_content)

        video_data = json.loads(html_content)
        assert video_data['error'] == 0, video_data

        real_url = video_data['data']['video_url']
        info.stream_types.append('current')
        info.streams['current'] = {'container': 'm3u8', 'video_profile': 'current', 'src' : [real_url], 'size': 0}

        return info

site = DouyutvVideo()
