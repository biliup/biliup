#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1, matchall
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
import xml.etree.ElementTree as ET

from ykdl.compact import urlencode, compact_bytes

import random
import base64
import struct
import uuid
import json

PLAYER_PLATFORMS = [11, 2, 1]
PLAYER_VERSION = '3.2.19.333'


def qq_get_final_url(url, vid, fmt_id, filename, fvkey, platform):
    params = {
        'appver': PLAYER_VERSION,
        'otype': 'json',
        'platform': platform,
        'filename': filename,
        'vid': vid,
        'format': fmt_id,
    }

    content = get_content('http://vv.video.qq.com/getkey?' + urlencode(params))
    data = json.loads(match1(content, r'QZOutputJson=(.+);$'))

    vkey = data.get('key', fvkey)
    if vkey:
        url = '{}{}?vkey={}'.format(url, filename, vkey)
    else:
        url = None
    vip = data.get('msg') == 'not pay'

    return url, vip

class QQ(VideoExtractor):

    name = u"腾讯视频 (QQ)"
    vip = None

    stream_2_id = {
        'fhd': 'BD',
        'shd': 'TD',
        'hd': 'HD',
        'mp4':'HD',
        'flv': 'HD',
        'sd': 'SD',
        'msd':'LD'
    }
    stream_ids = ['BD', 'TD', 'HD', 'SD', 'LD']


    def get_streams_info(self, profile='shd'):
        for PLAYER_PLATFORM in PLAYER_PLATFORMS.copy():
            params = {
                'otype': 'json',
                'platform': PLAYER_PLATFORM,
                'vid': self.vid,
                'defnpayver': 1,
                'appver': PLAYER_VERSION,
                'defn': profile,
            }

            content = get_content('http://vv.video.qq.com/getinfo?' + urlencode(params))
            data = json.loads(match1(content, r'QZOutputJson=(.+);$'))
            self.logger.debug('data: ' + str(data))

            if 'msg' in data:
                assert data['msg'] not in ('vid is wrong', 'vid status wrong'), 'wrong vid'
                PLAYER_PLATFORMS.remove(PLAYER_PLATFORM)
                continue

            if PLAYER_PLATFORMS and \
                    profile == 'shd' and \
                    '"name":"shd"' not in content and \
                    '"name":"fhd"' not in content:
                for infos in self.get_streams_info('hd'):
                    yield infos
                return
            break

        assert 'msg' not in data, data['msg']
        video = data['vl']['vi'][0]
        fn = video['fn']
        title = video['ti']
        td = float(video['td'])
        fvkey = video.get('fvkey')
        # Not to be absolutely accuracy.
        #fp2p = data.get('fp2p')
        #iflag = video.get('iflag')
        #pl = video.get('pl')
        #self.limit = bool(iflag or pl)
        self.vip = video['drm']

        # Priority for range fetch.
        cdn_url_1 = cdn_url_2 = cdn_url_3 = None
        for cdn in video['ul']['ui']:
            cdn_url = cdn['url']
            if 'vip' in cdn_url:
                continue
            # 'video.dispatch.tc.qq.com' supported keep-alive link.
            if cdn_url.startswith('http://video.dispatch.tc.qq.com/'):
                cdn_url_3 = cdn_url
            # IP host.
            elif match1(cdn_url, '(^http://[0-9\.]+/)'):
                if not cdn_url_2:
                    cdn_url_2 = cdn_url
            elif not cdn_url_1:
                cdn_url_1 = cdn_url
        if self.limit:
            cdn_url = cdn_url_3 or cdn_url_1 or cdn_url_2
        else:
            cdn_url = cdn_url_1 or cdn_url_2 or cdn_url_3

        dt = cdn['dt']
        if dt == 1:
            type_name = 'flv'
        elif dt == 2:
            type_name = 'mp4'
        else:
            type_name = fn.split('.')[-1]

        _num_clips = video['cl']['fc']
        self.limit = video.get('type', 0) > 1000
        if self.limit:
            if _num_clips > 1:
                self.logger.warning('Only parsed first video part!')
            for fmt in data['fl']['fi']:
                if fmt['sl']:
                    fmt_name = fmt['name']
                    fmt_cname = fmt['cname']
                    break
            fns = fn.split('.')
            fns.insert(-1, '1')
            filename = '.'.join(fns)
            url = '{}{}?vkey={}'.format(cdn_url, filename, fvkey)
            size = video['cl']['ci'][0]['cs'] # not correct, real size is smaller.
            rate = size // float(video['cl']['ci'][0]['cd'])
            yield title, fmt_name, fmt_cname, type_name, [url], size, rate
            return

        for fmt in data['fl']['fi']:
            fmt_id = fmt['id']
            fmt_name = fmt['name']
            fmt_cname = fmt['cname']
            size = fmt['fs']
            rate = size // td

            fns = fn.split('.')
            fmt_id_num = int(fmt_id)
            fmt_id_prefix = None
            num_clips = 0

            if fmt_id_num > 100000:
                fmt_id_prefix = 'm'
            elif fmt_id_num > 10000:
                fmt_id_prefix = 'p'
                num_clips = _num_clips or 1
            if fmt_id_prefix:
                fmt_id_name = fmt_id_prefix + str(fmt_id_num % 10000)
                if fns[1][0] in ('p', 'm') and not fns[1].startswith('mp'):
                    fns[1] = fmt_id_name
                else:
                    fns.insert(1, fmt_id_name)
            elif fns[1][0] in ('p', 'm') and not fns[1].startswith('mp'):
                del fns[1]

            urls =[]

            if num_clips == 0:
                filename = '.'.join(fns)
                url, vip = qq_get_final_url(cdn_url, self.vid, fmt_id, filename, fvkey, PLAYER_PLATFORM)
                if vip:
                    self.vip = vip
                elif url:
                    urls.append(url)
            else:
                fns.insert(-1, '1')
                for idx in range(1, num_clips+1):
                    fns[-2] = str(idx)
                    filename = '.'.join(fns)
                    url, vip = qq_get_final_url(cdn_url, self.vid, fmt_id, filename, fvkey, PLAYER_PLATFORM)
                    if vip:
                        self.vip = vip
                        break
                    elif url:
                        urls.append(url)

            yield title, fmt_name, fmt_cname, type_name, urls, size, rate

    def prepare(self):
        info = VideoInfo(self.name)
        if not self.vid:
            self.vid = match1(self.url,
                              'vid=(\w+)',
                              '/(\w+)\.html',
                              '/(\w+)$')

        if self.vid and match1(self.url, '(^https?://film\.qq\.com)'):
            self.url = 'https://v.qq.com/x/cover/%s.html' % self.vid

        if not self.vid or len(self.vid) != 11:
            html = get_content(self.url)
            self.vid = match1(html,
                              '&vid=(\w+)',
                              'vid:\s*[\"\'](\w+)',
                              'vid\s*=\s*[\"\']\s*(\w+)',
                              '"vid":"(\w+)"')

            if not self.vid and '<body class="page_404">' in html:
                self.logger.warning('This video has been deleted!')
                return info

        video_rate = {}
        for _ in range(2):
            try:
                for title, fmt_name, stream_profile, type_name, urls, size, rate in self.get_streams_info():
                    stream_id = self.stream_2_id[fmt_name]
                    if urls and stream_id not in info.stream_types:
                        info.stream_types.append(stream_id)
                        info.streams[stream_id] = {
                            'container': type_name,
                            'video_profile': stream_profile,
                            'src' : urls,
                            'size': size
                        }
                        video_rate[stream_id] = rate
                break
            except AssertionError as e:
                if 'wrong vid' in str(e):
                    html = get_content(self.url)
                    self.vid = match1(html,
                                      '&vid=(\w+)',
                                      'vid:\s*[\"\'](\w+)',
                                      'vid\s*=\s*[\"\']\s*(\w+)',
                                      '"vid":"(\w+)"')
                    continue
                raise e

        if self.vip:
            self.logger.warning('This is a VIP video!')
            #self.limit = False

        assert len(info.stream_types), "can't play this video!!"
        info.stream_types = sorted(info.stream_types, key = self.stream_ids.index)
        info.title = title

        if self.limit:
            # Downloading some videos is very slow, use multithreading range fetch to speed up.
            # Only for video players now.
            info.extra['rangefetch'] = {
                'first_size': 1024 * 16,
                'max_size': 1024 * 32,
                'threads': 10,
                'video_rate': video_rate
            }
            self.logger.warning('This is a restricted video!')

        info.extra['referer'] = 'https://v.qq.com/'
        return info

    def prepare_list(self):
        html = get_content(self.url)
        vids = [a.strip('"') for a in match1(html, '\"vid\":\[([^\]]+)').split(',')]
        return vids

site = QQ()
