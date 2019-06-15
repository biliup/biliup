#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1, matchall

from .bilibase import BiliBase, sign_api_url


APPKEY = 'iVGUTjsxvpLeuDCf'
SECRETKEY = 'aHRmhWMLkdeMuILqORnYZocwMBpMEOdt'
api_url = 'https://interface.bilibili.com/v2/playurl'

class BiliVideo(BiliBase):
    name = u'哔哩哔哩 (Bilibili)'

    def get_vid_title(self):
        av_id = match1(self.url, '(?:/av|aid=)(\d+)')
        page_index = '1'
        if "#page=" in self.url or "?p=" in self.url or 'index_' in self.url:
            page_index = match1(self.url, '(?:#page|\?p)=(\d+)', 'index_(\d+)\.')
        if page_index == '1':
            self.url = 'https://www.bilibili.com/av{}/'.format(av_id)
        else:
            self.url = 'https://www.bilibili.com/av{}/?p={}'.format(av_id, page_index)
        if not self.vid:
            html = get_content(self.url)
            #vid = match1(html, 'cid=(\d+)', 'cid="(\d+)', '"cid":(\d+)')
            title = match1(html, '"title":"([^"]+)', '<h1 title="([^"]+)', '<title>([^<]+)').strip()
            video_list = matchall(html, ['"cid":(\d+),"page":(\d+),"from":"[^"]+","part":"([^"]*)",'])
            for cid, page, part in video_list:
               if page == page_index:
                   vid = cid
                   if len(video_list) > 1:
                       title = u'{} - {} - {}'.format(title, page, part)
                   elif part:
                       title = u'{} - {}'.format(title, part)
                   break

        return vid, title

    def get_api_url(self, qn):
        params_str = 'appkey={}&cid={}&player=0&qn={}'.format(APPKEY, self.vid, qn)
        return sign_api_url(api_url, params_str, SECRETKEY)

    def prepare_list(self):
        av_id = match1(self.url, '(?:/av|aid=)(\d+)')
        self.url = 'https://www.bilibili.com/av{}/'.format(av_id)
        html = get_content(self.url)
        video_list = matchall(html, ['"page":(\d+),'])
        if video_list:
            return ['https://www.bilibili.com/av{}/?p={}'.format(av_id, p) for p in video_list]

site = BiliVideo()
