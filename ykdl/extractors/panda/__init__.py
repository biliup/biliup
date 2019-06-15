#!/usr/bin/env python
# -*- coding: utf-8 -*-

import re


def get_extractor(url):
    if re.search('xingyan.panda', url):
        from .import xingyan as s
    else:
        from .import panda as s
    return s.site
