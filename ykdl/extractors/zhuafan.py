#!/usr/bin/env python
# -*- coding: utf-8 -*-

import base64
import json

from ykdl.videoinfo import VideoInfo
from ykdl.extractor import VideoExtractor
from ykdl.util.match import match1
from ykdl.util.html import get_content


def decodeencoded(encodestr):
    b = bytearray(base64.b64decode(encodestr))
    t7 = bytearray()
    if len(b) > 12:
        if b[0] == 255 and b[1] == 255 and b[2] == 255 and b[3] == 254:
            t2 = b[4]
            t3 = b[5]
            t4 = b[6]
            t5 = b[7]
            t6 = (b[t4 + 8] & 255 ^ t2) << 24 | \
                 (b[t4 + 9] & 255 ^ t3) << 16 | \
                 (b[t4 + 10] & 255 ^ t2) << 8 | \
                  b[t4 + 11] & 255 ^ t3
            if t6 == len(b) - 12 - t4 - t5:
                t8 = t4 + 12
                t9 = t6 + 1
                t7 = bytearray(t9 - 1)
                while t9 >= 0:
                    if (t9 & 1) == 0:
                        t10 = t2
                    else:
                        t10 = t3
                    try:
                        t7[t9] = (b[t8 + t9] ^ t10)
                    except Exception as e:
                        pass
                    t9 -= 1
    retstr = t7.decode()
    return retstr

class JustFunLive(VideoExtractor):
    name = u"抓饭直播 (JustFun Live)"

    def prepare(self):
        info = VideoInfo(self.name, True)

        if self.url and not self.vid:
            html = get_content(self.url)

            title = match1(html, '<div class=\"play-title-inner\">([^<]+)</div>')
            info.artist = artist =match1(html, 'data-director=\"([^\"]+)\"')
            info.title = u'{} - {}'.format(title, artist)

            PL = match1(html, 'var PL = {([\s\S]+?)}')
            data = dict((k.strip(), json.loads(v)) for k, v in 
                        (kv.split(':') for kv in PL.split(','))
                        if k.strip())
            assert data['close'] == 'false', data['closeReason']
            self.logger.debug('Encoded playInfo: %s', data['playInfo'])
            playInfo = json.loads(decodeencoded(data['playInfo']))
            self.logger.debug('Decoded playInfo: %r', playInfo)

            # using only origin, as I have noticed - all links are same
            info.stream_types.append('current')
            info.streams['current'] = {
                'container': 'flv',
                'video_profile': 'current',
                'src': [playInfo['origin']],
                'size': float('inf')
            }

        return info

site = JustFunLive()
