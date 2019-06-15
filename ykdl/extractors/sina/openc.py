#!/usr/bin/env python
# -*- coding: utf-8 -*-

import hashlib
import random
import time

from ykdl.util.html import get_content
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.match import match1, matchall

def get_k(vid, rand):
    t = str(int('{0:b}'.format(int(time.time()))[:-6], 2))
    return hashlib.md5((vid + 'Z6prk18aWxP278cVAH' + t + rand).encode('utf-8')).hexdigest()[:16] + t
 
def video_info_xml(vid):
    rand = "0.{0}{1}".format(random.randint(10000, 10000000), random.randint(10000, 10000000))
    url = 'http://ask.ivideo.sina.com.cn/v_play.php?vid={0}&ran={1}&p=i&k={2}'.format(vid, rand, get_k(vid, rand))
    xml = get_content(url)
    return xml

class OpenC(VideoExtractor):
    name = u'Sina openCourse (新浪公开课)'

    def prepare(self):
        info = VideoInfo(self.name)
        if self.url:
            html = get_content(self.url)
            self.vid = match1(html, 'playVideo\(\"(\d+)')

        self.logger.debug("VID: {}".format(self.vid))

        xml = video_info_xml(self.vid)

        info.title = match1(xml, '<vname><!\[CDATA\[([^\]]+)')
        urls = matchall(xml, ['<url><!\[CDATA\[([^\]]+)'])
        sizes = matchall(xml, ['<filesize>([^<]+)'])
        size = 0
        for s in sizes:
            size += int(s)

        info.stream_types.append('current')
        info.streams['current'] = {'container': 'hlv', 'video_profile': 'current', 'src': urls, 'size' : size}
        return info

site = OpenC()

