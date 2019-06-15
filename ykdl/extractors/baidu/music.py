#!/usr/bin/env python
# -*- coding: utf-8 -*-

from __future__ import print_function
from ykdl.util.match import match1
from ykdl.util.html import get_content
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.compact import urlencode, compact_bytes

import time
import json

class BaiduMusic(VideoExtractor):
    name = u'BaiduMusic (百度音乐)'


    def prepare(self):
        info = VideoInfo(self.name)
        if not self.vid:
            self.vid = match1(self.url, 'http://music.baidu.com/song/([\d]+)')

        param = urlencode({'songIds': self.vid})

        song_data = json.loads(get_content('http://play.baidu.com/data/music/songlink', data=compact_bytes(param, 'utf-8')))['data']['songList'][0]

        info.title = song_data['songName']
        info.artist = song_data['artistName']

        info.stream_types.append('current')
        info.streams['current'] = {'container': song_data['format'], 'video_profile': 'current', 'src' : [song_data['songLink']], 'size': song_data['size']}
        return info

    def prepare_list(self):

        album_id = match1(self.url, 'http://music.baidu.com/album/([\d]+)')
        data = json.loads(get_content('http://play.baidu.com/data/music/box/album?albumId={}&type=album&_={}'.format(album_id, time.time())))

        print('album:		%s' % data['data']['albumName'])

        return data['data']['songIdList']

site = BaiduMusic()
