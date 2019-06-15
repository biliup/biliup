#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content
from ykdl.util.match import match1
import json
import time

class Panda(VideoExtractor):
    name = u'熊猫TV (Panda)'

    live_base = "http://pl{}.live.panda.tv/live_panda/{}.flv?sign={}&ts={}&rid={}"
    api_url = "http://www.panda.tv/api_room_v2?roomid={}&__plat=pc_web&_={}"

    def prepare(self):
        info = VideoInfo(self.name, True)
        if not self.vid:
            self.vid = match1(self.url, 'panda.tv/(\w+)')

        content = get_content(self.api_url.format(self.vid, int(time.time())))
        stream_data = json.loads(content)
        errno = stream_data['errno']
        errmsg = stream_data['errmsg']
        assert not errno, "Errno: {}, Errmsg: {}".format(errno, errmsg)
        assert stream_data['data']['videoinfo']['status'] == '2', u"error: (⊙o⊙)主播暂时不在家，看看其他精彩直播吧！"
        room_key = stream_data['data']['videoinfo']['room_key']
        plflag = stream_data['data']['videoinfo']['plflag'].split('_')[1]
        info.title = stream_data['data']['roominfo']['name']
        info.artist = stream_data['data']['hostinfo']['name']
        data2 = json.loads(stream_data['data']["videoinfo"]["plflag_list"])
        rid = data2["auth"]["rid"]
        sign = data2["auth"]["sign"]
        ts = data2["auth"]["time"]
        info.stream_types.append('current')
        info.streams['current'] = {'container': 'flv', 'video_profile': 'current', 'src' : [self.live_base.format(plflag, room_key, sign, ts, rid)], 'size': float('inf')}
        return info

site = Panda()
