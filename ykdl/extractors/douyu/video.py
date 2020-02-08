#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content, add_header
from ykdl.util.match import match1
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.compact import urlencode

from .util import get_h5enc, ub98484234

import json

class DouyutvVideo(VideoExtractor):
    name = u'斗鱼视频 (DouyuTV)'

    stream_ids = ['BD', 'TD', 'HD', 'SD', 'LD']
    profile_2_id = {
        u'high': 'TD',
        u'normal': 'HD',
    }

    def prepare(self):
        info = VideoInfo(self.name)
        pid = match1(self.url, 'show/(.*)')
        if 'vmobile' in self.url:
            self.url = 'https://v.douyu.com/show/' + pid

        html = get_content(self.url)
        info.title = match1(html, u'title>(.+?)_斗鱼视频 - 最6的弹幕视频网站<')
        self.vid = match1(html, '"point_id":\s?(\d+)')

        js_enc = get_h5enc(html, self.vid)
        params = {'vid': pid}
        ub98484234(js_enc, self, params)

        add_header("Referer", self.url)
        add_header('Cookie', 'dy_did={}'.format(params['did']))

        data = urlencode(params)
        if not isinstance(data, bytes):
            data = data.encode()
        html_content = get_content('https://v.douyu.com/api/stream/getStreamUrl', data=data)
        self.logger.debug('video_data: ' + html_content)

        video_data = json.loads(html_content)
        assert video_data['error'] == 0, video_data

        for video_profile, stream_date in video_data['data']['thumb_video'].items():
            if not stream_date:
                continue
            stream = self.profile_2_id[video_profile]
            info.stream_types.append(stream)
            info.streams[stream] = {
                'container': 'm3u8',
                'video_profile': video_profile,
                'src' : [stream_date['url']],
                'size': 0
            }

        info.stream_types = sorted(info.stream_types, key=self.stream_ids.index)
        return info

site = DouyutvVideo()
