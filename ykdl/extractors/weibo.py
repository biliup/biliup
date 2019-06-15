#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ..simpleextractor import SimpleExtractor
from ykdl.util.html import add_header, get_content
from ykdl.util.match import match1

class Weibo(SimpleExtractor):
    name = u"微博秒拍 (Weibo)"

    def __init__(self):
        SimpleExtractor.__init__(self)
        add_header('User-Agent', 'Mozilla/5.0 (Linux; Android 4.4.2; Nexus 4 Build/KOT49H) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/34.0.1847.114 Mobile Safari/537.36')

    def get_title(self):
        if self.title_patterns:
            self.info.title = match1(self.html, *self.title_patterns)
        # JSON string escaping in safe
        exec('self.info.title = """%s"""' % self.info.title.replace('"""', ''))

    def get_url(self):
        if self.url_patterns:
            self.v_url = [match1(self.html, *self.url_patterns)]

    def l_assert(self):
        # Mobile ver.
        self.title_patterns = '"content2": "(.+?)",', '"status_title": "(.+?)",'
        self.url_patterns = '"stream_url_hd": "([^"]+)', '"stream_url": "([^"]+)'
        if 'm.weibo.cn' in self.url:
            return
        if 'weibo.com' in self.url and '/tv/v/' not in self.url and 'fid=' not in self.url:
            self.url = self.url.replace('weibo.com', 'm.weibo.cn', 1)
            return

        self.url_patterns = r'(?:data-url|controls src)\s*=\s*[\"\']([^\"\']+)',
        self.title_patterns = '<title>([^<]+)</',
        self.url = self.url.replace('%3A', ':')
        fid = match1(self.url, r'\?fid=(\d{4}:\w+)')
        if fid is not None:
            self.url = 'http://p.weibo.com/show/channerWbH5/{}'.format(fid)
        elif '/p/230444' in self.url:
            fid = match1(url, r'/p/230444(\w+)')
            self.url = 'http://p.weibo.com/show/channerWbH5/1034:{}'.format(fid)
        else:
            html = get_content(self.url)
            url = match1(html, '"page_url": "([^"]+)')
            assert url, 'No url match'
            self.url = url
            self.l_assert()

site = Weibo()
