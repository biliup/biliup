#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1, matchall
from ykdl.compact import urlencode

from .bilibase import BiliBase, sign_api_url

import json


APPKEY = 'iVGUTjsxvpLeuDCf'
SECRETKEY = 'aHRmhWMLkdeMuILqORnYZocwMBpMEOdt'
api_url = 'https://interface.bilibili.com/v2/playurl'

class BiliVideo(BiliBase):
    name = u'哔哩哔哩 (Bilibili)'

    def get_page_info(self):
        page_index = match1(self.url, '\?p=(\d+)', 'index_(\d+)\.') or '1'
        html = get_content(self.url)
        date = json.loads(match1(html, '__INITIAL_STATE__=({.+?});'))['videoData']
        title = date['title']
        artist = date['owner']['name']
        pages = date['pages']
        for page in pages:
           index = str(page['page'])
           subtitle = page['part']
           if index == page_index:
               vid = page['cid']
               if len(pages) > 1:
                   title = u'{} - {} - {}'.format(title, index, subtitle)
               elif subtitle and subtitle != title:
                   title = u'{} - {}'.format(title, subtitle)
               break

        return vid, title, artist

    def get_api_url(self, qn):
        params_str = urlencode([
            ('appkey', APPKEY),
            ('cid', self.vid),
            ('platform', 'html5'),
            ('player', 0),
            ('qn', qn)
        ])
        return sign_api_url(api_url, params_str, SECRETKEY)

    def prepare_list(self):
        av_id = match1(self.url, '/av(\d+)')
        html = get_content(self.url)
        video_list = matchall(html, ['"page":(\d+),'])
        if video_list:
            return ['https://www.bilibili.com/av{}/?p={}'.format(av_id, p) for p in video_list]

site = BiliVideo()
