#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content, get_location, url_info, add_header
from ykdl.util.match import match1, matchall
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo

import json
import random

api_info1 = 'https://n.miaopai.com/api/aj_media/info.json?smid={}&appid=530&_cb={}'
api_info2 = 'http://api.miaopai.com/m/v2_channel.json?fillType=259&scid={}&vend='
api_stream = 'http://gslb.miaopai.com/stream/{}.json?vend='


def get_random_str(l):
    string = []
    chars = list('abcdefghijklnmopqrstuvwxyz0123456789')
    size = len(chars)
    for i in range(l):
        string.append(random.choice(chars))
    return ''.join(string)

class Miaopai(VideoExtractor):

    name = u'秒拍 (Miaopai)'

    def prepare(self):
        info = VideoInfo(self.name)
        html = None
        title = None

        if 'show' in self.url:
            new_url = get_location(self.url)
            if new_url != self.url:
                self.logger.debug('redirect to' + new_url)
                self.url = new_url

        if not self.vid:
            self.vid = match1(self.url, '/media/([^\./]+)')
        if not self.vid:
            html = get_content(self.url)
            self.vid = match1(html, 's[cm]id ?= ?[\'"]([^\'"]+)[\'"]')
        assert self.vid, "No VID match!"
        info.title = self.name + '_' + self.vid


        if len(self.vid) > 24:
            add_header('Referer', self.url)
            cb = '_jsonp{}'.format(get_random_str(10))
            json_html = get_content(api_info1.format(self.vid, cb))
            data = json.loads(json_html[json_html.find('{'):-2])
            assert data['code'] == 200, data['msg']

            data = data['data']
            title = data['description']
            url = data['meta_data'][0]['play_urls']['m']
            _, ext, _ = url_info(url)
        
        else:
            try:
                data = json.loads(get_content(api_info2.format(self.vid)))
                assert data['status'] == 200, data['msg']

                data = data['result']
                title = data['ext']['t']
                scid = data['scid'] or self.vid
                ext = data['stream']['and']
                base = data['stream']['base']
                vend = data['stream']['vend']
                url = '{}{}.{}?vend={}'.format(base, scid, ext, vend)
            except:
                # fallback
                data = json.loads(get_content(api_stream.format(self.vid)))
                assert data['status'] == 200, data['msg']

                data = data['result'][0]
                ext = None
                scheme = data['scheme']
                host = data['host']
                path = data['path']
                sign = data['sign']
                url = '{}{}{}{}'.format(scheme, host, path, sign)

        if not title:
            if not html:
                html = get_content(self.url)
            title = match1(html, '<meta name="description" content="([^"]+)">')
        if title:
            info.title = title

        info.stream_types.append('current')
        info.streams['current'] = {
            'container': ext or 'mp4',
            'src': [url],
            'size' : 0
        }
        return info

    def prepare_list(self):
        html = get_content(self.url)
        video_list = match1(html, 'video_list=\[([^\]]+)')
        return matchall(video_list, ['\"([^\",]+)'])

site = Miaopai()
