#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content, add_header
from ykdl.util.match import match1
from ykdl.util import log

import json

class YinYueTai(VideoExtractor):
    name = u'YinYueTai (音乐台)'
    ids = ['BD', 'TD', 'HD', 'SD' ]
    types_2_id = {'sh': 'BD', 'he': 'TD', 'hd':'HD', 'hc' :'SD' }
    types_2_profile = {'sh': u'原画', 'he': u'超清', 'hd': u'高清', 'hc' : u'标清' }

    def prepare(self):
        info = VideoInfo(self.name)
        if not self.vid:
            self.vid = match1(self.url, 'http://\w+.yinyuetai.com/video/(\d+)', 'http://\w+.yinyuetai.com/video/h5/(\d+)')

        data = json.loads(get_content('http://ext.yinyuetai.com/main/get-h-mv-info?json=true&videoId={}'.format(self.vid)))

        assert not data['error'], 'some error happens'

        video_data = data['videoInfo']['coreVideoInfo']

        info.title = video_data['videoName']
        info.artist = video_data['artistNames']
        for s in video_data['videoUrlModels']:
            stream_id = self.types_2_id[s['qualityLevel']]
            stream_profile = self.types_2_profile[s['qualityLevel']]
            info.stream_types.append(stream_id)
            info.streams[stream_id] = {'container': 'flv', 'video_profile': stream_profile, 'src' : [s['videoUrl']], 'size': s['fileSize']}

        info.stream_types = sorted(info.stream_types, key = self.ids.index)
        return info

    def prepare_list(self):

        playlist_id = match1(self.url, 'http://\w+.yinyuetai.com/playlist/(\d+)')

        playlist_data = json.loads(get_content('http://m.yinyuetai.com/mv/get-simple-playlist-info?playlistId={}'.format(playlist_id)))

        videos = playlist_data['playlistInfo']['videos']
        # TODO
        # I should directly use playlist data instead to request by vid... to be update
        return [v['playListDetail']['videoId'] for v in videos]

site = YinYueTai()
