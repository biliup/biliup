#!/usr/bin/env python
# -*- coding: utf-8 -*-

import re

def get_extractor(url):
    if 'cc.163' in url:
        from . import live as s
    elif 'open.163' in url:
        from . import openc as s
    elif 'music.163' in url:
        from . import music as s
        return s.get_extractor(url)
    elif '3g.163' in url:
        from . import m3g as s
    else:
        from . import video as s

    return s.site, url
