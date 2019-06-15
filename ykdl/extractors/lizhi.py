#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content
from ykdl.util.match import match1

import json

class Lizhi(VideoExtractor):
    name = u"Lizhi FM (荔枝电台)"
    audio_content = None
    def prepare(self):
        # url like http://www.lizhi.fm/#/549759/18864883431656710
        self.vid = match1(self.url, '/(\d+/\d+)')
        api_url = 'http://www.lizhi.fm/api/audio/'+ self.vid
        self.audio_content = json.loads(get_content(api_url))["audio"]

    def extract(self):
        self.stream_types = []
        self.title = self.audio_content["name"]
        res_url = self.audio_content["url"]
        self.stream_types.append('current')
        self.streams['current'] = {'container': 'mp3', 'video_profile': 'current', 'src' : [res_url], 'size': 0}

    def download_playlist(self, url, param):
        # like this http://www.lizhi.fm/#/31365/
        #api desc: s->start l->length band->some radio
        #http://www.lizhi.fm/api/radio_audios?s=0&l=100&band=31365
        self.param = param
        band_id = match1(url, '/(\d+)')
        #try to get a considerable large l to reduce html parsing task.
        api_url = 'http://www.lizhi.fm/api/radio_audios?s=0&l=65535&band='+band_id
        content_json = json.loads(get_content(api_url))
        for sound in content_json:
            self.audio_content = sound
            self.download_normal()

site = Lizhi()
