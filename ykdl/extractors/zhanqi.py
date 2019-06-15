#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1
from ykdl.util.m3u8_wrap import load_m3u8_playlist
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo

class Zhanqi(VideoExtractor):
    name = u'战旗 (zhanqi)'

    live_base = "http://dlhls.cdn.zhanqi.tv/zqlive/"
    vod_base = "http://dlvod.cdn.zhanqi.tv"
    def prepare(self):
        info = VideoInfo(self.name)
        html = get_content(self.url)
        video_type = match1(html, 'VideoType":"([^"]+)"')
        if video_type == 'LIVE':
            info.live = True
        elif not video_type == 'VOD':
            raise NotImplementedError('Unknown_video_type')

        info.title = match1(html, '<title>([^<]+)').split("_")[0]
        if info.live:
            rtmp_id = match1(html, 'videoId":"([^"]+)"').replace('\\/','/')
            real_url = self.live_base+'/'+rtmp_id+'.m3u8'
            print(real_url)
            info.stream_types, info.streams = load_m3u8_playlist(real_url)
        else:
            vod_url = match1(html, 'VideoID":"([^"]+)')
            if vod_url:
                vod_m3u8 = self.vod_base + '/' + match1(html, 'VideoID":"([^"]+)').replace('\\/','/')
            else:
                vod_m3u8 = match1(html, 'PlayUrl":"([^"]+)').replace('\\/','/')
            info.stream_types.append('current')
            info.streams['current'] = {'container': 'm3u8', 'video_profile': 'current', 'src' : [vod_m3u8], 'size': 0}
        return info

site = Zhanqi()
