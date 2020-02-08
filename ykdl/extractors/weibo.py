#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ..simpleextractor import SimpleExtractor
from ykdl.util.html import add_header, get_content, get_location
from ykdl.util.match import match1
from ykdl.compact import compact_unquote, urlsplit, parse_qs


class Weibo(SimpleExtractor):
    name = u"微博秒拍 (Weibo)"

    def __init__(self):
        SimpleExtractor.__init__(self)
        add_header('User-Agent', 'Baiduspider')

    def get_title(self):
        if self.title_patterns:
            self.info.title = match1(self.html, *self.title_patterns)
        # JSON string escaping in safe
        exec('self.info.title = """%s"""' % self.info.title.replace('"""', ''))

    def get_url(self):
        if self.url_patterns:
            v_url = match1(self.html, *self.url_patterns)
            if v_url.startswith('http%3A'):
                v_url = compact_unquote(v_url)
            self.v_url = [v_url]

    def l_assert(self):
        if self.url.startswith('http://'):
            self.url = self.url.replace('http://', 'https://', 1)
        self.url = get_location(self.url)

        if 'passport.weibo.com' in self.url:
            query = urlsplit(self.url).query
            self.url = parse_qs(query)['url'][0]
            return self.l_assert()

        # Mobile ver.
        if 'm.weibo.cn' in self.url:
            self.title_patterns = '"content2": "(.+?)",', '"status_title": "(.+?)",'
            self.url_patterns = '"stream_url_hd": "([^"]+)', '"stream_url": "([^"]+)'
            return

        if '/tv/v/' in self.url or 'fid=' not in self.url:
            self.title_patterns = 'class="info_txt \w+">([^<]+)</', 'class="WB_text \w+"[^>]+>\s*(?:<a[^<]+</a>)?\s*([^<]+)'
            self.url_patterns = 'video-sources\s*=\s*".+?(?:&\d+=http.+?)*&\d+=(http.+?[^=])(?:&\d+=)*&qType=\w+"',
            return

        self.title_patterns = '<title>([^<]+)</',
        self.url_patterns = r'(?:data-url|controls src)\s*=\s*[\"\']([^\"\']+)',
        html = get_content(self.url)
        url = match1(html, '"page_url": "([^"]+)')
        assert url, 'No url match'
        self.url = url
        self.l_assert()

site = Weibo()
