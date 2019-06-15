#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo

import json

class CNTV(VideoExtractor):
    name = u'央视网 (cctv)'

    supported_stream_types = ['TD', 'HD', 'SD', 'LD']
    type_2_cpt = {'TD': 'chapters4', 'HD': 'chapters3', 'SD':'chapters2', 'LD':'lowChapters' }

    def prepare(self):
        info = VideoInfo(self.name)
        if self.url and not self.vid:
            content = get_content(self.url)
            self.vid = match1(content, 'guid = "([^"]+)', '"videoCenterId","([^"]+)')
        assert self.vid, 'cant find vid'

        html = get_content('http://vdn.apps.cntv.cn/api/getHttpVideoInfo.do?pid={}'.format(self.vid))
        data = json.loads(html)

        video_data = data['video']
        info.title = data['title']

        for t in self.supported_stream_types:
            if self.type_2_cpt[t] in video_data:
                urls = []
                for v in video_data[self.type_2_cpt[t]]:
                   urls.append(v['url'])
                info.stream_types.append(t)
                info.streams[t] = {'container': 'mp4', 'video_profile': t, 'src': urls, 'size' : 0}
        return info

site = CNTV()
