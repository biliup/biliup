#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import add_default_handler, install_default_handlers, get_content, add_header
from ykdl.util.match import match1, matchall
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.compact import HTTPCookieProcessor

import json
import sys
import base64
import uuid
import time

ua = 'AppleCoreMedia/1.0.0.16E227 (iPhone; U; CPU OS 12_2 like Mac OS X; zh_cn)'

py3 = sys.version_info[0] == 3
if py3:
    maketrans = bytes.maketrans
    bytearray2str = bytearray.decode
else:
    from string import maketrans 
    bytearray2str = str

encode_translation = maketrans(b'+/=', b'_~-')
decode_translation = maketrans(b'_~-', b'+/=')

def encode_tk2(s):
    if not isinstance(s, bytes):
        s = s.encode()
    s = bytearray(base64.b64encode(s).translate(encode_translation))
    s.reverse()
    return bytearray2str(s)

def decode_tk2(s):
    if not isinstance(s, bytes):
        s = s.encode()
    s = bytearray(s)
    s.reverse()
    s = base64.b64decode(s.translate(decode_translation))
    if not isinstance(s, str):
        s = s.decode()
    return s

def generate_tk2(did):
    s = 'did={}|pno=1030|ver=0.3.0301|clit={}'.format(did, int(time.time()))
    return encode_tk2(s)

class Hunantv(VideoExtractor):
    name = u"芒果TV (HunanTV)"

    supported_stream_profile = [ u'蓝光', u'超清', u'高清', u'标清' ]
    supported_stream_types = [ 'BD', 'TD', 'HD', 'SD' ]
    profile_2_types = { u'蓝光': 'BD', u'超清': 'TD', u'高清': 'HD', u'标清': 'SD' }
    
    def prepare(self):
        add_default_handler(HTTPCookieProcessor)
        install_default_handlers()
        add_header('Referer', self.url)
        add_header('User-Agent', ua)

        info = VideoInfo(self.name)
        if self.url and not self.vid:
            self.vid = match1(self.url, 'https?://www.mgtv.com/b/\d+/(\d+).html')
            if self.vid is None:
                html = get_content(self.url)
                self.vid = match1(html, 'vid=(\d+)', 'vid=\"(\d+)', 'vid: (\d+)')

        did = str(uuid.uuid4())
        tk2 = generate_tk2(did)

        api_info_url = 'https://pcweb.api.mgtv.com/player/video?tk2={}&video_id={}&type=pch5'.format(tk2, self.vid)
        meta = json.loads(get_content(api_info_url))

        assert meta['code'] == 200, '[failed] code: {}, msg: {}'.format(meta['code'], meta['msg'])
        assert meta['data'], '[Failed] Video info not found.'

        pm2 = meta['data']['atc']['pm2']
        info.title = meta['data']['info']['title']

        api_source_url = 'https://pcweb.api.mgtv.com/player/getSource?pm2={}&tk2={}&video_id={}&type=pch5'.format(pm2, tk2, self.vid)
        meta = json.loads(get_content(api_source_url))

        assert meta['code'] == 200, '[failed] code: {}, msg: {}'.format(meta['code'], meta['msg'])
        assert meta['data'], '[Failed] Video source not found.'

        data = meta['data']
        domain = data['stream_domain'][0]
        for lstream in data['stream']:
            lurl = lstream['url']
            if lurl:
                lurl = '{}{}&did={}'.format(domain, lurl, did)
                url = json.loads(get_content(lurl))['info']
                video_profile = lstream['name']
                stream = self.profile_2_types[video_profile]
                info.streams[stream] = {
                    'container': 'm3u8',
                    'video_profile': video_profile,
                    'src' : [url]
                }
                info.stream_types.append(stream)
        info.stream_types= sorted(info.stream_types, key = self.supported_stream_types.index)
        info.extra['referer'] = self.url
        info.extra['ua'] = ua
        return info

    def prepare_list(self):

        html = get_content(self.url, headers={})

        return matchall(html, ['"a-pic-play" href="([^"]+)"'])

site = Hunantv()
