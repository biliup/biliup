#!/usr/bin/env python
# -*- coding: utf-8 -*-

import re

def get_extractor(url):
    if 'video' in url:
        from . import video as s
    elif 'gongkaike' in url:
        from . import gongkaike as s
    else:
        from . import news as s

    return s.site, url
