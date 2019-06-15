#!/usr/bin/env python
# -*- coding: utf-8 -*-


import json

from ykdl.compact import urlopen
from ykdl.simpleextractor import SimpleExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content
from ykdl.util.match import match1


class PandaXingyan(SimpleExtractor):
    name = u'熊猫星颜 (Panda Xingyan)'

    def prepare(self):
        self.live = True
        info = VideoInfo(self.name, True)

        page = get_content(self.url)
        page_meta = match1(page, r'window\.HOSTINFO=(.+?});</script>')
        page_meta = json.loads(page_meta)

        info.title = page_meta['roominfo']['name']
        info.artist = page_meta['hostinfo']['nickName']
        info.stream_types.append('current')

        stream_url = page_meta['videoinfo']['streamurl']
        assert int(page_meta['roominfo']['playstatus']), 'live show is offline'

        info.streams['current'] = dict(container='flv', video_profile='current', src=[stream_url], size=float('inf'))

        return info

site = PandaXingyan()


