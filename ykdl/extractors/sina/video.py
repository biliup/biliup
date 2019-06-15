#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.util.match import match1, matchall
from ykdl.util.html import get_content, get_location
from ykdl.videoinfo import VideoInfo

import json

def get_realurl(url):
    location = get_location(url)
    if location != url:
        return location
    else:
       html = get_content(url)
       return matchall(html, ['CDATA\[([^\]]+)'])[1]

class Sina(VideoExtractor):
    name = u"新浪视频 (sina)"

    def prepare(self):
        info = VideoInfo(self.name)
        self.vid = match1(self.url, 'video_id=(\d+)', '#(\d{5,})', '(\d{5,})\.swf')
        if not self.vid:
            html = get_content(self.url)
            self.vid = match1(html,
                              'video_id:\'([^\']+)',
                              'SINA_TEXT_PAGE_INFO[\s\S]+?video_id: ?(\d+)')

        assert self.vid, "can't get vid"

        api_url = 'http://s.video.sina.com.cn/video/h5play?video_id={}'.format(self.vid)
        data = json.loads(get_content(api_url))['data']
        info.title = data['title']
        for t in ['mp4', '3gp', 'flv']:
            if t in data['videos']:
                video_info = data['videos'][t]
                break

        for profile in video_info:
            if not profile in info.stream_types:
                v = video_info[profile]
                tp = v['type']
                url = v['file_api']+'?vid='+v['file_id']
                r_url = get_realurl(url)
                info.stream_types.append(profile)
                info.streams[profile] = {'container': tp, 'video_profile': profile, 'src': [r_url], 'size' : 0}
        return info

    def prepare_list(self):
        html = get_content(self.url)
        return matchall(html, ['video_id: ([^,]+)'])

site = Sina()
