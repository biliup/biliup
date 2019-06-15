#!/usr/bin/env python
# -*- coding: utf-8 -*-

from .sohubase import SohuBase

class MySohu(SohuBase):
    name = u'搜狐自媒体 (MySohu)'

    apiurl = 'http://my.tv.sohu.com/play/videonew.do?vid=%s&referer=http://my.tv.sohu.com'

site = MySohu()
