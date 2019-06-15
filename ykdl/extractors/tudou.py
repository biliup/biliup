#!/usr/bin/env python
# -*- coding: utf-8 -*-

from .youku import Youku
from ykdl.util.html import get_location, add_header
from ykdl.util.match import match1

class Tudou(Youku):
    name = u"Tudou (土豆)"

    def prepare(self):
        if match1(self.url, '(new-play|video)\.tudou\.com/') is None:
            self.url = get_location(self.url)
        return Youku.prepare(self)

site = Tudou()
