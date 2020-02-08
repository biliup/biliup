#!/usr/bin/env python
# -*- coding: utf-8 -*-

import re

def get_extractor(url):
    if 'live.qq' in url:
        from . import live as s
    elif 'egame.qq' in url:
        from . import egame as s
    else:
        from . import video as s

    return s.site, url
