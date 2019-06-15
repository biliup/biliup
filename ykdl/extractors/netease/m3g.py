#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.simpleextractor import SimpleExtractor
from ykdl.util.match import matchall
from ykdl.util.html import get_content

class m3g(SimpleExtractor):
    name = u"网易手机网 (163 3g)"

    def __init__(self):
        SimpleExtractor.__init__(self)
        self.url_patterns = ['"contentUrl":"([^"]+)"', '<video\s+data-src="([^"]+)"']
        self.title_pattern = 'class="title">(.+?)</'

    def get_url(self):
        if self.url_patterns:
            v_url = []
            for url in matchall(self.html, self.url_patterns):
                if url[:2] == '//':
                    url = 'http:' + url
                if url not in v_url:
                    v_url.append(url)
            self.v_url = v_url

site = m3g()
