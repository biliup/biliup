#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.compact import urlencode

import json
import time

class CNTV(VideoExtractor):
    name = u'央视网 (CNTV)'

    supported_chapters = [
        ['chapters6',   'BD', u'超高清 1080P'],
        ['chapters5',   'TD', u'超高清 720P'],
        ['chapters4',   'TD', u'超清'],
        ['chapters3',   'HD', u'高清'],
        ['chapters2',   'SD', u'标清'],
        ['lowChapters', 'LD', u'流畅']]

    def prepare(self):
        info = VideoInfo(self.name)
        self.vid = match1(self.url, 'videoCenterId=(\w+)')
        if self.url and not self.vid:
            content = get_content(self.url)
            self.vid = match1(content, 'guid = "([^"]+)', '"videoCenterId","([^"]+)')
        assert self.vid, 'cant find vid'

        params = {
            'pid': self.vid,
            'tsp': int(time.time()),
            'vn': 2054,
            'pcv': 152438790
        }
        data = get_content('http://vdn.apps.cntv.cn/api/getHttpVideoInfo.do?' + urlencode(params))
        self.logger.debug('data> ' + data)
        data = json.loads(data)

        video_data = data['video']
        info.title = u'{} - {}'.format(data['title'], data['play_channel'])

        for chapters, stream_type, profile in self.supported_chapters:
            stream_data = video_data.get(chapters)
            if stream_data:
                urls = []
                for v in stream_data:
                   urls.append(v['url'])
                info.stream_types.append(stream_type)
                info.streams[stream_type] = {
                    'container': 'mp4',
                    'video_profile': profile,
                    'src': urls, 
                    'size' : 0
                }
        return info

site = CNTV()
