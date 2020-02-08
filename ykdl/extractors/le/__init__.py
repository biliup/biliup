#!/usr/bin/env python
# -*- coding: utf-8 -*-

import re

def get_extractor(url):
    if 'lunbo' in url:
        from . import lunbo as s
    elif re.search("(live[\./]|/izt/)", url):
        from . import live as s
    elif 'bcloud' in url:
        from . import letvcloud as s
    else:
        from . import le as s

    return s.site, url
