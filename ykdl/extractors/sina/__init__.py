#!/usr/bin/env python
# -*- coding: utf-8 -*-

import re

def get_extractor(url):
    if re.search("open.sina", url):
        from . import openc as s
    else:
        from . import video as s
    return s.site
