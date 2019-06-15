#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content
from ykdl.util.match import match1, matchall
from ykdl.compact import compact_bytes

import hashlib
import json

SECKEY = "RTYDFkHAL$#%^tsf_)(*^Gd%$"

class Chushou(VideoExtractor):
    name = u'Chushou Live (触手直播)'

    def prepare(self):
        info = VideoInfo(self.name, True)
        self.vid = match1(self.url, '(\d+).htm')
        api_url = "https://chushou.tv/live-room/get-play-url.htm"
        t = get_content("https://chushou.tv/timestamp/get.htm")
        sign_context = "_t={}&protocols=1,2&roomId={}".format(t,self.vid)
        sign = hashlib.md5(compact_bytes(SECKEY+'&'+sign_context, 'utf-8')).hexdigest()

        data = json.loads(get_content(api_url+'?'+sign_context+'&_sign={}'.format(sign)))

        assert data['code'] == 0, data['message']

        info.stream_types.append("current")
        info.streams["current"] = {'container': 'flv', 'video_profile': 'current', 'src' : [data["data"][0]['shdPlayUrl']], 'size': float('inf')}

        return info

site = Chushou()
