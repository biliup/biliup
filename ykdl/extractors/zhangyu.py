#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.simpleextractor import SimpleExtractor
from ykdl.util.html import get_content, add_header
from ykdl.util.match import match1, matchall

class ZYLive(SimpleExtractor):
    name = u"ZhangYu Live (章鱼直播)"

    def __init__(self):
        SimpleExtractor.__init__(self)
        add_header('User-Agent', 'Mozilla/5.0 (iPhone; CPU iPhone OS 5_0 like Mac OS X) AppleWebKit/534.46 (KHTML, like Gecko) Version/5.1 Mobile/9A334 Safari/7534.48.3')
        self.live = True
        self.title_pattern = '<title>([^<]+)'
        self.url_pattern = '<video _src=\'([^\']+)'
        self.artist_pattern = 'videoTitle = \"([^\"]+)'

site = ZYLive()
