#!/usr/bin/env python
# -*- coding: utf-8 -*-

import re

def get_extractor(url):
    if re.search("/program", url):
        from . import program as s
    elif re.search("/dj", url):
        from . import program as s
    elif re.search("/mv", url):
        from . import mv as s
    else:
        from . import music as s
    return s.site
