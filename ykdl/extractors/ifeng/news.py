#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from xml.dom.minidom import parseString
from ykdl.util.html import get_content
from ykdl.util.match import match1

class Ifeng(VideoExtractor):
    name = u'凤凰新闻 (ifeng news)'

    supported_stream_types = ['1M','500k', '350k']
    types_2_id = {'1M': 'TD','500k': 'HD', '350k':'SD'}
    types_2_profile = {'1M': u'超清','500k': u'高清', '350k':u'标清'}
    ids = ['TD', 'HD', 'SD']
    def prepare(self):
        info = VideoInfo(self.name)
        if not self.vid:
            self.vid= match1(self.url, '#([a-zA-Z0-9\-]+)', '/([a-zA-Z0-9\-]+).shtml')
        if not self.vid:
            html = get_content(self.url)
            self.vid = match1(html, '"vid": "([^"]+)', 'vid: "([^"]+)')

        xml = get_content('http://vxml.ifengimg.com/video_info_new/{}/{}/{}.xml'.format(self.vid[-2], self.vid[-2:], self.vid))
        doc = parseString(xml.encode('utf-8'))
        info.title = doc.getElementsByTagName('item')[0].getAttribute("Name")
        videos = doc.getElementsByTagName('videos')
        for v in videos[0].getElementsByTagName('video'):
            if v.getAttribute("mediaType") == 'mp4':
                _t = v.getAttribute("type")
                _u = v.getAttribute("VideoPlayUrl")
                stream_id = self.types_2_id[_t]
                stream_profile = self.types_2_profile[_t]
                info.stream_types.append(stream_id)
                info.streams[stream_id] = {'container': 'mp4', 'video_profile': stream_profile, 'src' : [_u], 'size': 0}

        info.stream_types = sorted(info.stream_types, key = self.ids.index)
        return info

site = Ifeng()
