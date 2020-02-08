#!/usr/bin/env python
# -*- coding: utf-8 -*-

import re

def get_extractor(url):
    if 'live.iqiyi' in url:
        from . import live as s
    else:
        from . import video as s

    return s.site, url
