#!/usr/bin/env python
# -*- coding: utf-8 -*-

def get_extractor(url):
    if "com/v" in url:
        from . import video as s
    else:
        from . import live as s
    return s.site

