#!/usr/bin/env python
# -*- coding: utf-8 -*-

import re

def get_extractor(url):
    if 'my.tv.sohu.com' in url:
        from . import my as s
        return s.site, url
    elif 'edu.tv.sohu.com' in url:
        from . import edu as s
        return s.site, url
    else:
        from . import tv as s
        return s.site, url

    raise NotImplementedError(url)
