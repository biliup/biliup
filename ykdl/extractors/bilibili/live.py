#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content, get_location
from ykdl.util.match import match1, matchall

import json
import random

api_url = 'https://api.live.bilibili.com/room/v1/Room/playUrl'
api1_url = 'https://api.live.bilibili.com/room/v1/Room/room_init'
api2_url = 'https://api.live.bilibili.com/room/v1/Room/get_info'

class BiliLive(VideoExtractor):
    name = u"Bilibili live (哔哩哔哩 直播)"

    supported_stream_profile = [u'原画', u'超清', u'高清', u'流畅']
    profile_2_type = {u'原画': 'BD', u'超清': 'TD', u'高清': 'HD', u'流畅' :'SD'}

    def prepare(self):
        info = VideoInfo(self.name, True)
        ID = match1(self.url, "/(\d+)")
        api1_data = json.loads(get_content("{}?id={}".format(api1_url, ID)))
        self.vid = api1_data["data"]["room_id"]
        api2_data = json.loads(get_content("{}?room_id={}&from=room".format(api2_url, self.vid)))
        info.title = api2_data["data"]["title"]
        assert api2_data["data"]["live_status"] == 1, u"主播正在觅食......"

        def get_live_info(q=0):
            data = json.loads(get_content("{}?player=1&cid={}&quality={}&otype=json".format(api_url, self.vid, q)))

            assert data["code"] == 0, data["msg"]

            data = data["data"]
            urls = [random.choice(data["durl"])["url"]]
            qlt = data['current_quality']
            aqlts = [int(x) for x in data['accept_quality']]
            size = float('inf')
            ext = 'flv'
            prf = self.supported_stream_profile[4 - qlt]
            st = self.profile_2_type[prf]
            if urls and st not in info.streams:
                info.stream_types.append(st)
                info.streams[st] = {'container': ext, 'video_profile': prf, 'src' : urls, 'size': size}

            if q == 0:
                aqlts.remove(qlt)
                for aqlt in aqlts:
                    get_live_info(aqlt)

        get_live_info()
        return info

site = BiliLive()
