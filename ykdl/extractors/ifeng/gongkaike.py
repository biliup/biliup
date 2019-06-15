#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from xml.dom.minidom import parseString
from ykdl.util.html import get_content
from ykdl.util.match import match1, matchall

class IfengOpenC(VideoExtractor):
    name = u'凤凰公开课 (ifeng open course)'

    def prepare(self):
        info = VideoInfo(self.name)
        if not self.vid:
            self.vid= match1(self.url, '#([a-zA-Z0-9\-]+)', '/([a-zA-Z0-9\-]+).shtml')
        if not self.vid:
            html = get_content(self.url)
            self.vid = match1(html, '"vid": "([^"]+)', 'vid: "([^"]+)')

        xml = get_content('http://vxml.ifengimg.com/video_info_new/{}/{}/{}.xml'.format(self.vid[-2], self.vid[-2:], self.vid))

        info.title = match1(xml, 'SE_Title="([^"]+)')
        urls = matchall(xml, ['playurl="([^"]+)'])
        urls = ['http://ips.ifeng.com/' + u[7:] for u in urls ]
        info.stream_types.append('current')
        info.streams['current'] = {'container': 'mp4', 'video_profile': 'current', 'src' : urls, 'size': 0}

        return info

site = IfengOpenC()
