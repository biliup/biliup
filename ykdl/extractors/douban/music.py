#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.match import match1
from ykdl.util.html import get_content
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.compact import urlencode, compact_bytes

import json

class DoubanMusic(VideoExtractor):
    name = u'Douban Music (豆瓣音乐)'

    song_info = {}

    def prepare(self):
        info = VideoInfo(self.name)
        if not self.vid:
            self.vid = match1(self.url, 'sid=(\d+)')

        params = {
            "source" : "",
            "sids" : self.vid,
            "ck" : ""
        }
        form = urlencode(params)
        data = json.loads(get_content('https://music.douban.com/j/artist/playlist', data = compact_bytes(form, 'utf-8')))
        self.song_info = data['songs'][0]
        self.extract_song(info)
        return info

    def extract_song(self, info):
        song = self.song_info
        info.title = song['title']
        info.artist = song['artist_name']
        info.stream_types.append('current')
        info.streams['current'] = {'container': 'mp3', 'video_profile': 'current', 'src' : [song['url']], 'size': 0}

    def parser_list(self, url):

        sids = match1(url, 'sid=([0-9,]+)')

        params = {
            "source" : "",
            "sids" : sids,
            "ck" : ""
        }
        form = urlencode(params)
        data = json.loads(get_content('https://music.douban.com/j/artist/playlist', data = compact_bytes(form, 'utf-8')))

        info_list = []
        for s in data['songs']:
            info = VideoInfo(self.name)
            self.song_info = s
            self.extract_song(info)
            info_list.append(info)
        return info_list


site = DoubanMusic()
