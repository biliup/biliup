#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1
from ykdl.embedextractor import EmbedExtractor
from ykdl.videoinfo import VideoInfo

class Dilidili(EmbedExtractor):
    name = u'嘀哩嘀哩（dilidili）'

    def build_videoinfo(self, title, ext, *urls):
        info = VideoInfo(self.name)
        info.title = title
        channel = 1
        for url in urls:
            t = 'Channel ' + str(channel)
            info.stream_types.append(t)
            info.streams[t] = {
                'container': ext,
                'video_profile': t,
                'src': [url],
                'size' : 0
            }
            channel += 1
        self.video_info['info'] = info
    
    def prepare(self):
        html = get_content(self.url)
        title = match1(html, u'<title>(.+?)丨嘀哩嘀哩</title>')
        source_url = match1(html, r'var sourceUrl\s?=\s?"(.+?)"', r'var Url\s?=\s?"(.+?)"')
        
        # First type, bangumi
        # http://www.dilidili.wang/watch3/72766/ or
        # http://www.dilidili.wang/watch3/52919/
        if source_url:
            ext = source_url.split('?')[0].split('.')[-1]
        
            # Dilidili hosts this video itself
            if ext in ('mp4', 'flv', 'f4v', 'm3u', 'm3u8'):
                self.build_videoinfo(title, ext, source_url)
        
            # It is an embedded video from other websites
            else:
                self.video_info['url'] = source_url
                self.video_info['title'] = title

        # Second type, user-uploaded videos
        # http://www.dilidili.wang/huiyuan/76983/
        else:
            player_url = match1(html, r'<iframe src="(.+?)"')
            html = get_content(player_url)
            video_url = match1(html, r'var main = "(.+?)"')
            video_url_full = '/'.join(player_url.split('/')[0:3]) + video_url
            ext = video_url.split('?')[0].split('.')[-1]
            self.build_videoinfo(title, ext, video_url_full)


site = Dilidili()
