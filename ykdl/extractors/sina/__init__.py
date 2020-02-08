#!/usr/bin/env python
# -*- coding: utf-8 -*-

import re

def get_extractor(url):
    if 'open.sina' in url:
        from . import openc as s
    else:
        from . import video as s

    return s.site, url
