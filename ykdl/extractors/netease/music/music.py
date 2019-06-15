#!/usr/bin/env python
# -*- coding: utf-8 -*-

import json

from .musicbase import NeteaseMusicBase
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content, add_header
from ykdl.util.match import match1


class NeteaseMusic(NeteaseMusicBase):
    name = u"Netease Music (网易云音乐)"
    api_url = "http://music.163.com/api/song/detail/?id={}&ids=[{}]&csrf_token="

    def get_music(self, data):
       return data['songs'][0]

    def prepare_list(self):
        add_header("Referer", "http://music.163.com/")
        vid =  match1(self.url, '\?id=(.*)')
        if "album" in self.url:
           api_url = "http://music.163.com/api/album/{}?id={}&csrf_token=".format(vid, vid)
           listdata = json.loads(get_content(api_url))
           playlist = listdata['album']['songs']
        elif "playlist" in self.url:
           api_url = "http://music.163.com/api/playlist/detail?id={}&csrf_token=".format(vid)
           listdata = json.loads(get_content(api_url))
           playlist = listdata['result']['tracks']
        elif "toplist" in self.url:
           api_url = "http://music.163.com/api/playlist/detail?id={}&csrf_token=".format(vid)
           listdata = json.loads(get_content(api_url))
           playlist = listdata['result']['tracks']
        elif "artist" in self.url:
           api_url = "http://music.163.com/api/artist/{}?id={}&csrf_token=".format(vid, vid)
           listdata = json.loads(get_content(api_url))
           playlist = listdata['hotSongs']

        return [p['id'] for p in playlist]

site = NeteaseMusic()
