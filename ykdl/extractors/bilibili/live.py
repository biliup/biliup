#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.compact import urlencode
from ykdl.util.html import get_content, get_location
from ykdl.util.match import match1, matchall

import json
import random

api_url = 'https://api.live.bilibili.com/room/v1/Room/playUrl?'
api1_url = 'https://api.live.bilibili.com/room/v1/Room/room_init?id={}'
api2_url = 'https://api.live.bilibili.com/room/v1/Room/get_info?room_id={}'
api3_url = 'https://api.live.bilibili.com/live_user/v1/UserInfo/get_anchor_in_room?roomid={}'

class BiliLive(VideoExtractor):
    name = u'Bilibili live (哔哩哔哩 直播)'

    profile_type = [
        (u'原画', 'BD'),
        (u'超清', 'TD'),
        (u'高清', 'HD'),
        (u'流畅', 'SD')
    ]
    supported_stream_profile = [prf for prf, _ in profile_type]
    profile_2_type = dict(profile_type)

    sorted_format = [fmt for _, fmt in profile_type]

    def prepare(self):
        info = VideoInfo(self.name, True)

        ID = match1(self.url, '/(\d+)')
        api1_data = json.loads(get_content(api1_url.format(ID)))
        if api1_data['code'] == 0:
            self.vid = api1_data['data']['room_id']
        else:
            self.logger.debug('Get room ID from API failed: %s', api1_data['msg'])
            self.vid = ID

        api2_data = json.loads(get_content(api2_url.format(self.vid)))
        assert api2_data['code'] == 0, api2_data['msg']
        api2_data = api2_data['data']
        assert api2_data['live_status'] == 1, u'主播正在觅食......'
        info.title = title = api2_data['title']

        api3_data = json.loads(get_content(api3_url.format(self.vid)))
        if api3_data['code'] == 0:
            info.artist = artist = api3_data['data']['info']['uname']
            info.title = '{} - {}'.format(title, artist)

        def get_live_info(q=0):
            params = {
                'player': 1,
                'cid': self.vid,
                'platform': 'html5',
                'quality': q,
                'otype': 'json'
            }
            data = json.loads(get_content(api_url + urlencode(params)))

            assert data['code'] == 0, data['message']

            data = data['data']
            urls = [random.choice(data['durl'])['url']]
            qlt = data['current_quality']
            aqlts = [int(x) for x in data['accept_quality']]
            size = float('inf')
            ext = 'flv'
            prf = self.supported_stream_profile[4 - qlt]
            st = self.profile_2_type[prf]
            if urls and st not in info.streams:
                info.stream_types.append(st)
                info.streams[st] = {
                    'container': ext,
                    'video_profile': prf,
                    'src' : urls,
                    'size': size
                }

            if q == 0:
                aqlts.remove(qlt)
                for aqlt in aqlts:
                    get_live_info(aqlt)

        get_live_info()
        info.stream_types = sorted(info.stream_types, key=self.sorted_format.index)
        return info

site = BiliLive()
