#!/usr/bin/env python
# -*- coding: utf-8 -*-

import json

from ykdl.util.match import match1
from ykdl.util.html import get_content, add_header
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.compact import compact_unquote


class Baomihua(VideoExtractor):

    name = u"爆米花（Baomihua)"

    def prepare(self):

        info = VideoInfo(self.name)
        if self.url:
            self.vid = match1(self.url, '_(\d+)', 'm/(\d+)', 'v/(\d+)')

        add_header('Referer', 'http://m.video.baomihua.com/')
        html = get_content('http://play.baomihua.com/getvideourl.aspx?flvid={}&datatype=json&devicetype=wap'.format(self.vid))
        data = json.loads(html)

        info.title = compact_unquote(data["title"])
        host = data['host']
        stream_name = data['stream_name']
        t = data['videofiletype']
        size = int(data['videofilesize'])

        hls = data['ishls']
        url = "http://{}/{}/{}.{}".format(host, hls, stream_name, t)
        info.stream_types.append('current')
        info.streams['current'] = {'video_profile': 'current', 'container': t, 'src': [url], 'size' : size}
        return info

site = Baomihua()
