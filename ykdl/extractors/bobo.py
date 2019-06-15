#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.match import match1
from ykdl.util.html import default_proxy_handler, get_content
from ykdl.compact import build_opener, HTTPCookieProcessor, install_opener

class BoBo(VideoExtractor):
    name = u"bobo娱乐 美女直播"

    def prepare(self):
        handlers = [HTTPCookieProcessor()]
        if default_proxy_handler:
            handlers += default_proxy_handler
        install_opener(build_opener(*handlers))

        info = VideoInfo(self.name, True)
        html = get_content(self.url)
        self.vid = match1(html, '"userNum":(\d+)')
        live_id = match1(html, '"liveId":\s*(\d+)')
        assert live_id, u"主播正在休息"
        info.stream_types.append('current')
        info.streams['current'] = {'container': 'mp4', 'src': ['http://extapi.live.netease.com/redirect/video/{}'.format(self.vid)], 'size' : float('inf')}
        info.artist = match1(html, '"nick":"([^"]+)')
        info.title = match1(html, '<title>([^<]+)').split('-')[0]
        return info

site = BoBo()
