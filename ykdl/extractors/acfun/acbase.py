#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content, add_header
from ykdl.embedextractor import EmbedExtractor
from ykdl.videoinfo import VideoInfo

import json

class AcBase(EmbedExtractor):

    def build_videoinfo(self, title, artist, size, urls):
        info = VideoInfo(self.name)
        info.title = title
        info.artist = artist
        info.stream_types.append('current')
        info.streams['current'] = {
            'container': 'm3u8',
            'src': urls,
            'size' : size
        }
        self.video_info['info'] = info

    def prepare(self):
        html = get_content(self.url)
        title, artist, sourceVid = self.get_page_info(html)

        add_header('Referer', 'https://www.acfun.cn/')
        try:
            data = json.loads(get_content('https://www.acfun.cn/video/getVideo.aspx?id={}'.format(sourceVid)))

            sourceType = data['sourceType']
            sourceId = data['sourceId']
            if sourceType == 'zhuzhan':
                sourceType = 'acfun.zhuzhan'
                encode = data['encode']
                sourceId = (sourceId, encode)
            elif sourceType == 'letv':
                #workaround for letv, because it is letvcloud
                sourceType = 'le.letvcloud'
                sourceId = (sourceId, '2d8c027396')
            elif sourceType == 'qq':
                sourceType = 'qq.video'

            self.video_info = {
                'site': sourceType,
                'vid': sourceId,
                'title': title,
                'artist': artist
            }
        except IOError:
            # TODO: get more qualities
            data = json.loads(get_content('https://www.acfun.cn/rest/pc-direct/play/playInfo/m3u8Auto?videoId={}'.format(sourceVid)))
            stream = data['playInfo']['streams'][0]
            size = stream['size']
            urls = stream['playUrls']
            self.build_videoinfo(title, artist, size, urls)

    def prepare_playlist(self):
        for p in self.get_path_list():
            next_url = 'https://www.acfun.cn' + p
            video_info = self.new_video_info()
            video_info['url'] = next_url
            self.video_info_list.append(video_info)

