#!/usr/bin/env python
# -*- coding: utf-8 -*-

import json
import random
import base64, time
import sys
import hashlib

from ykdl.util.html import get_content, url_info
from ykdl.util.match import match1, matchall
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.compact import compact_tempfile

def calcTimeKey(t):
    ror = lambda val, r_bits, : ((val & (2**32-1)) >> r_bits%32) |  (val << (32-(r_bits%32)) & (2**32-1))
    magic = 185025305
    return ror(t, magic % 17) ^ magic

def decode(data):
    version = data[0:5]
    if version.lower() == b'vc_01':
        #get real m3u8
        loc2 = bytearray(data[5:])
        length = len(loc2)
        loc4 = [0]*(2*length)
        for i in range(length):
            loc4[2*i] = loc2[i] >> 4
            loc4[2*i+1]= loc2[i] & 15;
        loc6 = loc4[len(loc4)-11:]+loc4[:len(loc4)-11]
        loc7 = bytearray(length)
        for i in range(length):
            loc7[i] = (loc6[2 * i] << 4) +loc6[2*i+1]
        return loc7
    else:
        # directly return
        return data

class Letv(VideoExtractor):
    name = u"乐视 (Letv)"

    supported_stream_types = [ '1080p', '1300', '1000', '720p', '350' ]

    stream_2_profile = {'1080p': u'1080p' , '1300': u'超清', '1000': u'高清' , '720p': u'标清', '350': u'流畅' }

    stream_2_id = {'1080p': 'BD' , '1300': 'TD', '1000': 'HD' , '720p': 'SD', '350': 'LD' }

    __STREAM_TEMP__ = []


    def prepare(self):
        info = VideoInfo(self.name)
        stream_temp = {'1080p': None , '1300': None, '1000':None , '720p': None, '350': None }
        self.__STREAM_TEMP__.append(stream_temp)
        if not self.vid:
            self.vid = match1(self.url, 'vplay/(\d+).html', '#record/(\d+)')

        #normal process
        url = 'http://player-pc.le.com/mms/out/video/playJson?id={}&platid=1&splatid=105&format=1&tkey={}&domain=www.le.com&region=cn&source=1000&accessyx=1'.format(self.vid, calcTimeKey(int(time.time())))
        r = get_content(url)
        data=json.loads(r)
        data = data['msgs']

        info.title = data['playurl']['title']
        available_stream_id = sorted(list(data["playurl"]["dispatch"].keys()), key = self.supported_stream_types.index)
        for stream in available_stream_id:
            s_url =data["playurl"]["domain"][0]+data["playurl"]["dispatch"][stream][0]
            uuid = hashlib.sha1(s_url.encode('utf8')).hexdigest() + '_0'
            s_url = s_url.replace('tss=0', 'tss=ios')
            s_url+="&m3v=1&termid=1&format=1&hwtype=un&ostype=MacOS10.12.4&p1=1&p2=10&p3=-&expect=3&tn={}&vid={}&uuid={}&sign=letv".format(random.random(), self.vid, uuid)
            r2=get_content(s_url)
            data2=json.loads(r2)

            # hold on ! more things to do
            # to decode m3u8 (encoded)
            suffix = '&r=' + str(int(time.time() * 1000)) + '&appid=500'
            m3u8 = get_content(data2["location"]+suffix, charset = 'ignore')
            m3u8_list = decode(m3u8)
            stream_id = self.stream_2_id[stream]
            info.streams[stream_id] = {'container': 'm3u8', 'video_profile': self.stream_2_profile[stream], 'size' : 0}
            stream_temp[stream] = compact_tempfile(mode='w+b', suffix='.m3u8')
            stream_temp[stream].write(m3u8_list)
            info.streams[stream_id]['src'] = [stream_temp[stream].name]
            stream_temp[stream].flush()
            info.stream_types.append(stream_id)
        return info

    def prepare_list(self):

        html = get_content(self.url)

        return matchall(html, ['vid="(\d+)"'])

site = Letv()
