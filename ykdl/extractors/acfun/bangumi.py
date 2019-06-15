#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1, matchall
from ykdl.compact import urlencode

import json
import time

from .acbase import AcBase


class AcBan(AcBase):

    name = u'ACfun 弹幕视频网 (番剧)'

    def list_only(self):
        return '/bangumi/aa' in self.url

    def get_page_info(self, html):
        artist = None
        bgmInfo = json.loads(match1(html, u'var bgmInfo = ({.+?})</script>'))
        videoInfo = bgmInfo['video']['videos'][0]
        title = u'{} - {} {}'.format(
                bgmInfo['album']['title'],
                videoInfo['episodeName'],
                videoInfo['newTitle']
        ).rstrip()
        sourceVid = videoInfo['videoId']

        return title, artist, sourceVid

    def get_path_list(self):
        html = get_content(self.url)
        albumId = match1(self.url, '/a[ab](\d+)')
        groupId = match1(html, '"groups":[{[^}]*?"id":(\d+)')
        contentsCount = int(match1(html, '"contentsCount":(\d+)'))
        params = {
            'albumId': albumId,
            'groupId': groupId,
            'num': 1,
            'size': max(contentsCount, 20),
            '_': int(time.time() * 1000),
        }
        data = json.loads(get_content('https://www.acfun.cn/album/abm/bangumis/video?' + urlencode(params)))
        videos = []
        for c in data['data']['content']:
            vid = c['videos'][0]['id']
            v = '/bangumi/ab{}_{}_{}'.format(albumId, groupId, vid)
            videos.append(v)

        return videos

site = AcBan()
