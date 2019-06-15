#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo

import json
from random import randint
import datetime

class Laifeng(VideoExtractor):
    name = u'laifeng (来疯直播)'

    def prepare(self):
        assert self.url, "please provide valid url"
        info = VideoInfo(self.name, True)
        html = get_content(self.url)
        Alias = match1(html, 'initAlias:\'([^\']+)' ,'"ln":\s*"([^"]+)"')
        Token = match1(html, 'initToken: \'([^\']+)', '"tk":\s*"([^"]+)"')
        info.artist = match1(html, 'anchorName:\s*\'([^\']+)', '"anchorName":\s*"([^"]+)"')
        info.title = info.artist + u'的直播房间'
        t = datetime.datetime.utcnow().isoformat().split('.')[0] + 'Z'
        api_url = "http://lapi.lcloud.laifeng.com/Play?AppId=101&StreamName={}&Action=Schedule&Token={}&Version=2.0&CallerVersion=3.3&Caller=flash&Format=HttpFlv&Timestamp={}&Format=HttpFlv&rd={}".format(Alias, Token, t, randint(10000, 99999) )
        data1 = json.loads(get_content(api_url))
        assert data1['Code'] == 'Success', data1['Message']
        stream_url = data1['HttpFlv'][0]['Url']

        info.stream_types.append('current')
        info.streams['current'] = {'container': 'flv', 'video_profile': 'current', 'src' : [stream_url], 'size': float('inf')}
        return info

site = Laifeng()
