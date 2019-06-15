#!/usr/bin/env python
# -*- coding: utf-8 -*-
import re

def get_extractor(url):
    if re.search("v.douyu", url) or re.search("vmobile.douyu", url):
        from . import video as s
        return s.site
    else:
        from . import live as s
        return s.site
