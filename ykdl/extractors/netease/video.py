#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.compact import compact_unquote

class NeteaseVideo(VideoExtractor):
    name = u"网易视频 (163)"
    sopported_stream_types = ['shd', 'hd', 'flv']
    stream_2_profile = {'shd': u'超清', 'hd': u'高清', 'flv': u'标清'}
    stream_2_id = {'shd': 'TD', 'hd': 'HD', 'flv': 'SD'}

    def prepare(self):
        info = VideoInfo(self.name)
        if not self.vid:
            html = get_content(self.url)
            topiccid = match1(html, 'topicid : \"([^\"]+)', 'topicid=([^&]+)')
            vid = match1(html, 'vid : \"([^\"]+)', 'vid=([^&]+)')
            self.vid = (topiccid, vid)
        topiccid, _vid = self.vid
        code = _vid[-2:]
        video_xml = get_content('http://xml.ws.126.net/video/{}/{}/{}_{}.xml'.format(code[0], code[1], topiccid, _vid))
        info.title = compact_unquote(match1(video_xml, '<title>([^<]+)'))

        for tp in self.sopported_stream_types:
            searchcode = '<{}Url><flv>([^<]+)'.format(tp)
            url = match1(video_xml, searchcode)
            if url:
                info.stream_types.append(self.stream_2_id[tp])
                info.streams[self.stream_2_id[tp]] = {'container': 'flv', 'video_profile': self.stream_2_profile[tp], 'src' : [url], 'size': 0}
        return info

site = NeteaseVideo()
