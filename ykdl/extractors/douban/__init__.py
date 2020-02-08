#!/usr/bin/env python
# -*- coding: utf-8 -*-

import re

def get_extractor(url):
    if 'music.douban' in url:
        from . import music as s
        return s.site, url

    raise NotImplementedError(url)
