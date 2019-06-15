#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.simpleextractor import SimpleExtractor
from ykdl.util.match import match1
from ykdl.util.html import get_content

class OpenC(SimpleExtractor):
    name = u"网易公开课 (163 openCourse)"

    def __init__(self):
        SimpleExtractor.__init__(self)
        self.url_pattern = 'appsrc : .(.+?).,\n'
        self.title_pattern = 'title : .(.+?).,\n'

site = OpenC()
