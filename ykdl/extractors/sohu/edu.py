#!/usr/bin/env python
# -*- coding: utf-8 -*-

from .sohubase import SohuBase

class EduSohu(SohuBase):
    name = u'搜狐课堂 (Sohu Edu)'

    apiurl = 'http://my.tv.sohu.com/play/videonew.do?vid=%s&referer=http://edu.tv.sohu.com/'

site = EduSohu()
