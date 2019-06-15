#!/usr/bin/env python
# -*- coding: utf-8 -*-

import re

from ykdl.util.html import get_location

def get_extractor(url):
    if not 'bangumi' in url:
        url = get_location(url)
    if re.search("live.bili", url):
        from . import live as s
    elif re.search("vc.bili", url):
        from . import vc as s
    elif re.search("bangumi", url):
        from . import bangumi as s
    else:
        from . import video as s
    return s.site
