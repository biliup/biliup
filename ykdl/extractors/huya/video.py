#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content
from ykdl.util.match import match1
from ykdl.compact import compact_str

import json

class HuyaVideo(VideoExtractor):
    name = u"huya video (虎牙视频)"

    supported_stream_types = ['BD', 'TD', 'HD', 'SD']

    stream_2_profile = {u'原画':"BD", u'超清': 'TD', u'高清': 'HD', u'流畅': 'SD' }

    def prepare(self):
        info = VideoInfo(self.name)
        if not self.vid:
            self.vid = match1(self.url, 'play/(\d+)')
            info.title = self.name + str(self.vid)

        if not self.vid:
            html = get_content(self.url)
            self.vid = match1(html, 'data-vid="(\d+)')
            info.title = match1(html, '<title>([^<]+)').split('_')[0]

        api_url = 'http://playapi.v.duowan.com/index.php?vid={}&partner=&r=play%2Fvideo'.format(self.vid)
        data = json.loads(get_content(api_url))['result']['items']

        for i in data:
            d = i['transcode']
            s = i['task_name'][0:2]
            p = self.stream_2_profile[compact_str(s)]
            info.stream_types.append(p)
            info.streams[p] = {'container': 'mp4', 'video_profile': s, 'src': [d['urls'][0]], 'size' : int(d['size'])}

        info.stream_types = sorted(info.stream_types, key = self.supported_stream_types.index)
        return info

site = HuyaVideo()
