#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ..simpleextractor import SimpleExtractor

import json
import re

class Ku6(SimpleExtractor):
    name = u"酷6 (Ku6)"

    def __init__(self):
        SimpleExtractor.__init__(self)
        self.url_pattern = 'flvURL: "([^"]+)'
        self.title_pattern = 'title = "([^"]+)'

site = Ku6()
