#!/usr/bin/env python
# -*- coding: utf-8 -*-
import re

def get_extractor(url):
    if re.search('live.iqiyi', url):
        from . import live as s
    else:
        from . import video as s
    return s.site
