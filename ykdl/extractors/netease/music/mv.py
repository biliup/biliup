#!/usr/bin/env python
# -*- coding: utf-8 -*-

import json

from ykdl.util.html import get_content, add_header
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.match import match1

class NeteaseMv(VideoExtractor):
    name = u'Netease Mv (网易音乐Mv)'

    supported_stream_code = ['1080', '720', '480', '240']
    code_2_id = {'1080': 'BD', '720': 'TD', '480':'HD', '240':'SD'}
    code_2_profile = {'1080': '1080p', '720': u'超清', '480': u'高清', '240': u'标清'}
    def prepare(self):
        add_header("Referer", "http://music.163.com/")
        video = VideoInfo(self.name)
        if not self.vid:
            self.vid =  match1(self.url, '\?id=(.*)', 'mv/(\d+)')

        api_url = "http://music.163.com/api/mv/detail/?id={}&ids=[{}]&csrf_token=".format(self.vid, self.vid)
        mv = json.loads(get_content(api_url))['data']
        video.title = mv['name']
        video.artist = mv['artistName']
        for code in self.supported_stream_code:
            if code in mv['brs']:
                stream_id = self.code_2_id[code]
                stream_profile = self.code_2_profile[code]
                video.stream_types.append(stream_id)
                video.streams[stream_id] = {'container': 'mp4', 'video_profile': stream_profile, 'src' : [mv['brs'][code]], 'size': 0}
        return video
site = NeteaseMv()
