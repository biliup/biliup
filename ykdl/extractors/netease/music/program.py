#!/usr/bin/env python
# -*- coding: utf-8 -*-

import json

from .musicbase import NeteaseMusicBase
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content, add_header
from ykdl.util.match import match1

class NeteaseDj(NeteaseMusicBase):
    name = u'Netease Dj (网易电台)'
    api_url = "http://music.163.com/api/dj/program/detail/?id={}&ids=[{}]&csrf_token="

    def get_music(self, data):
       return data["program"]["mainSong"]

    def prepare_list(self):

        add_header("Referer", "http://music.163.com/")
        vid =  match1(self.url, '\?id=([^&]+)')
        if "djradio" in self.url:
           api_url = "http://music.163.com/api/dj/program/byradio/?radioId={}&ids=[{}]&csrf_token=".format(vid, vid)
           listdata = json.loads(get_content(api_url))
           playlist = listdata['programs']
        return [p['id'] for p in playlist]

site = NeteaseDj()
