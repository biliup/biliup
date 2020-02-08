#!/usr/bin/env python
# -*- coding: utf-8 -*-

import re

def get_extractor(url):
    if '/program' in url:
        from . import program as s
    elif '/dj' in url:
        from . import program as s
    elif '/mv' in url:
        from . import mv as s
    else:
        from . import music as s

    return s.site, url
