#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1, matchall
from ykdl.compact import urlencode

import json
import time

from .acbase import AcBase


class AcBan(AcBase):

    name = u'AcFun 弹幕视频网 (番剧)'

    def get_page_info(self, html):
        artist = None
        bgmInfo = json.loads(match1(html, u'(?:pageInfo|bangumiData) = ({.+?});'))
        videoInfo = bgmInfo.get('currentVideoInfo')
        assert videoInfo, bgmInfo.get('playErrorMessage') or "can't play this video!!"

        title = u'{} - {}'.format(
                bgmInfo['bangumiTitle'],
                bgmInfo['episodeName'],
        )
        sourceVid = videoInfo['id']
        m3u8Info = videoInfo.get('playInfos')
        if m3u8Info:
            m3u8Info = m3u8Info[0]
        else:
            m3u8Info = videoInfo.get('ksPlayJson')

        return title, artist, sourceVid, m3u8Info

    def get_path_list(self):
        html = get_content(self.url)
        videos = matchall(html, ['href=[\'"](/bangumi/aa\d+_\d+_\d+)[\'"] data-title=[\'"]'])
        return videos

site = AcBan()
