#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.simpleextractor import SimpleExtractor
from ykdl.util.match import match1

import re
import json

class Iqilu(SimpleExtractor):
    name = u"齐鲁网 (iqilu)"

    def __init__(self):
        SimpleExtractor.__init__(self)
        self.title_pattern = '<meta name="description" content="(.*?)\"\W'
        self.url_pattern = 'id=\'playerId\' url=\'(.*)\''

    def l_assert(self):
        assert re.match(r'http://v.iqilu.com/\w+', self.url)


site = Iqilu()
