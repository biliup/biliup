#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content, add_header, fake_headers, get_location
from ykdl.util.match import match1, matchall
from ykdl.compact import compact_bytes

import hashlib
from xml.dom.minidom import parseString


def sign_api_url(api_url, params_str, skey):
    chksum = hashlib.md5(compact_bytes(params_str + skey, 'utf8')).hexdigest()
    return '{}?{}&sign={}'.format(api_url, params_str, chksum)

def parse_cid_playurl(xml):
    urls = []
    size = 0
    doc = parseString(xml.encode('utf-8'))
    fmt = doc.getElementsByTagName('format')[0].firstChild.nodeValue
    qlt = doc.getElementsByTagName('quality')[0].firstChild.nodeValue
    aqlts = doc.getElementsByTagName('accept_quality')[0].firstChild.nodeValue.split(',')
    for durl in doc.getElementsByTagName('durl'):
        urls.append('https' + durl.getElementsByTagName('url')[0].firstChild.nodeValue[4:])
        size += int(durl.getElementsByTagName('size')[0].firstChild.nodeValue)
    return urls, size, fmt, qlt, aqlts

class BiliBase(VideoExtractor):
    format_2_type_profile = {
        'hdflv2': ('BD', u'高清 1080P+'), #112
        'flv':    ('BD', u'高清 1080P'),  #80
        'flv720': ('TD', u'高清 720P'),   #64
        'hdmp4':  ('TD', u'高清 720P'),   #48
        'flv480': ('HD', u'清晰 480P'),   #32
        'mp4':    ('SD', u'流畅 360P'),   #16
        'flv360': ('SD', u'流畅 360P'),   #15
        }

    sorted_format = ['BD', 'TD', 'HD', 'SD']

    def prepare(self):
        info = VideoInfo(self.name)
        add_header("Referer", "https://www.bilibili.com/")
        info.extra["referer"] = "https://www.bilibili.com/"
        info.extra["ua"] = fake_headers['User-Agent']

        self.vid, info.title = self.get_vid_title()

        assert self.vid, "can't play this video: {}".format(self.url)

        def get_video_info(qn=0):
            # need login with "qn=112"
            if int(qn) > 80:
                return

            api_url = self.get_api_url(qn)
            html = get_content(api_url)
            self.logger.debug('HTML> ' + html)
            code = match1(html, '<code>([^<])')
            if code:
                return

            urls, size, fmt, qlt, aqlts = parse_cid_playurl(html)
            if 'mp4' in fmt:
                ext = 'mp4'
            elif 'flv' in fmt:
                ext = 'flv'
            st, prf = self.format_2_type_profile[fmt]
            if urls and st not in info.streams:
                info.stream_types.append(st)
                info.streams[st] = {'container': ext, 'video_profile': prf, 'src' : urls, 'size': size}

            if qn == 0:
                aqlts.remove(qlt)
                for aqlt in aqlts:
                    get_video_info(aqlt)

        get_video_info()

        assert len(info.stream_types), "can't play this video!!"
        info.stream_types = sorted(info.stream_types, key = self.sorted_format.index) 
        return info
