#!/usr/bin/env python
# -*- coding: utf-8 -*-

def get_extractor(url):
    if '/bangumi/' in url:
        from . import bangumi as s
    else:
        from . import video as s
    return s.site
