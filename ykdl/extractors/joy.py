#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content, url_info
from ykdl.util.match import match1, matchall
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo

class Joy(VideoExtractor):

    name = u'激动网 (Joy)'

    def prepare(self):
        info = VideoInfo(self.name)
        if not self.vid:
            self.vid = match1(self.url, 'resourceId=([0-9]+)')
        if not self.url:
            self.url = "http://www.joy.cn/video?resourceId={}".format(self.vid)

        html= get_content(self.url)

        info.title = match1(html, '<meta content=\"([^\"]+)')

        url = matchall(html, ['<source src=\"([^\"]+)'])[3]

        _, ext, size = url_info(url)

        info.stream_types.append('current')
        info.streams['current'] = {'container': ext, 'src': [url], 'size': size }
        return info

site = Joy()
