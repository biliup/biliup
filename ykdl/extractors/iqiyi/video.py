#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import matchall, match1
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.compact import urlencode, compact_bytes

from .util import get_macid, md5, md5x, cmd5x

import json
import time

def gettmts(tvid, vid):
    tm = int(time.time() * 1000)
    key = 'd5fb4bd9d50c4be6948c97edd7254b0e'
    host = 'https://cache.m.iqiyi.com'
    params = {
        'src': '76f90cbd92f94a2e925d83e8ccd22cb7',
        'sc': md5(str(tm) + key + vid),
        't': tm
    }
    src = '/tmts/{}/{}/?{}'.format(tvid, vid, urlencode(params))
    req_url = '{}{}'.format(host, src)
    html = get_content(req_url)
    return json.loads(html)

def getdash(tvid, vid, bid=500):
    tm = int(time.time() * 1000)
    host = 'https://cache.video.iqiyi.com'
    params = {
        'tvid': tvid,
        'bid': bid,
        'vid': vid,
        'src': '01010031010000000000',
        'vt': 0,
        'rs': 1,
        'uid': '',
        'ori': 'pcw',
        'ps': 0,
        'tm': tm,
        'qd_v': 1,
        'k_uid': get_macid(),
        'pt': 0,
        'd': 0,
        's': '',
        'lid': '',
        'cf': '',
        'ct': '',
        'authKey': cmd5x('0{}{}'.format(tm, tvid)),
        'k_tag': 1,
        'ost': 0,
        'ppt': 0,
        'locale': 'zh_cn',
        'pck': '',
        'k_err_retries': 0,
        'ut': 0
    }
    src = '/dash?{}'.format(urlencode(params))
    vf = cmd5x(src)
    req_url = '{}{}&vf={}'.format(host, src, vf)
    html = get_content(req_url)
    return json.loads(html)

def getvps(tvid, vid):
    tm = int(time.time() * 1000)
    host = 'http://cache.video.qiyi.com'
    params = {
        'tvid': tvid,
        'vid': vid,
        'v': 0,
        'qypid': '{}_12'.format(tvid),
        'src': '01012001010000000000',
        't': tm,
        'k_tag': 1,
        'k_uid': get_macid(),
        'rs': 1,
    }
    src = '/vps?{}'.format(urlencode(params))
    vf = md5x(src)
    req_url = '{}{}&vf={}'.format(host, src, vf)
    html = get_content(req_url)
    return json.loads(html)

class Iqiyi(VideoExtractor):
    name = u"爱奇艺 (Iqiyi)"

    ids = ['4k','BD', 'TD', 'HD', 'SD', 'LD']
    vd_2_id = dict(sum([[(vd, id) for vd in vds] for id, vds in {
        '4k': [10, 19],
        'BD': [5, 18, 600],
        'TD': [4, 17, 500],
        'HD': [2, 14, 21, 300],
        'SD': [1, 200],
        'LD': [96, 100]
    }.items()], []))
    id_2_profile = {
        '4k': '4k',
        'BD': '1080p',
        'TD': '720p',
        'HD': '540p',
        'SD': '360p',
        'LD': '210p'
    }

    def prepare(self):
        info = VideoInfo(self.name)

        if self.url and not self.vid:
            vid = matchall(self.url, ['curid=([^_]+)_([\w]+)'])
            if vid:
                self.vid = vid[0]
                info_u = 'http://pcw-api.iqiyi.com/video/video/playervideoinfo?tvid=' + self.vid[0]
                try:
                    info_json = json.loads(get_content(info_u))
                    info.title = info_json['data']['vn']
                except:
                    self.vid = None

        def get_vid():
            html = get_content(self.url)
            video_info = match1(html, ":video-info='(.+?)'")

            if video_info:
                video_info = json.loads(video_info)
                self.vid = str(video_info['tvId']), str(video_info['vid'])
                info.title = video_info['name']

            else:
                tvid = match1(html,
                              'tvId:\s*"([^"]+)',
                              'data-video-tvId="([^"]+)',
                              '''\['tvid'\]\s*=\s*"([^"]+)''',
                              '"tvId":\s*([^,]+)')
                videoid = match1(html,
                                'data-video-vid="([^"]+)',
                                'vid:\s*"([^"]+)',
                                '''\['vid'\]\s*=\s*"([^"]+)''',
                                '"vid":\s*([^,]+)')
                if not (tvid and videoid):
                    url = match1(html, '(www\.iqiyi\.com/v_\w+\.html)')
                    if url:
                        self.url = 'https://' + url
                        return get_vid()
                self.vid = (tvid, videoid)
                info.title = match1(html, '<title>([^<]+)').split('-')[0]

        if self.url and not self.vid:
            get_vid()
        tvid, vid = self.vid
        assert tvid and vid, 'can\'t play this video!!'

        def push_stream_vd(vs):
            vd = vs['vd']
            stream = self.vd_2_id[vd]
            if not stream in info.streams:
                info.stream_types.append(stream)
            elif int(vd) < 10: 
                return
            m3u8 = vs['m3utx']
            stream_profile = self.id_2_profile[stream]
            info.streams[stream] = {
                'video_profile': stream_profile,
                'container': 'm3u8',
                'src': [m3u8],
                'size': 0
            }

        def push_stream_bid(bid, container, fs_array, size):
            stream = self.vd_2_id[bid]
            if stream in info.streams:
                return
            real_urls = []
            for seg_info in fs_array:
                url = url_prefix + seg_info['l']
                json_data = json.loads(get_content(url))
                down_url = json_data['l']
                real_urls.append(down_url)
            info.stream_types.append(stream)
            stream_profile = self.id_2_profile[stream]
            info.streams[stream] = {
                'video_profile': stream_profile,
                'container': container,
                'src': real_urls,
                'size': size
            }

        try:
            # try use tmts first
            # less http requests, get results quickly
            tmts_data = gettmts(tvid, vid)
            self.logger.debug('tmts_data:\n' + str(tmts_data))
            assert tmts_data['code'] == 'A00000', 'can\'t play this video!!'
            vs_array = tmts_data['data']['vidl']
            for vs in vs_array:
                push_stream_vd(vs)
            vip_conf = tmts_data['data'].get('ctl', {}).get('configs')
            if vip_conf:
                for vds in (('10', '19'), ('18', '5')):
                    for vd in vds:
                        if vd in vip_conf:
                            tmts_data = gettmts(tvid, vip_conf[vd]['vid'])
                            if tmts_data['code'] == 'A00000':
                                push_stream_vd(tmts_data['data'])
                                break

        except:
            try:
                # use vps as preferred fallback
                vps_data = getvps(tvid, vid)
                self.logger.debug('vps_data:\n' + str(vps_data))
                assert vps_data['code'] == 'A00000', 'can\'t play this video!!'
                url_prefix = vps_data['data']['vp']['du']
                vs_array = vps_data['data']['vp']['tkl'][0]['vs']
                for vs in vs_array:
                    bid = vs['bid']
                    fs_array = vs['fs']
                    size = vs['vsize']
                    push_stream_bid(bid, 'flv', fs_array, size)

            except:
                # use dash as fallback
                for bid in (500, 300, 200, 100):
                    dash_data = getdash(tvid, vid, bid)
                    self.logger.debug('dash_data:\n' + str(dash_data))
                    assert dash_data['code'] == 'A00000', 'can\'t play this video!!'
                    url_prefix = dash_data['data']['dd']
                    streams = dash_data['data']['program']['video']
                    for stream in streams:
                        if 'fs' in stream:
                            _bid = stream['bid']
                            container = stream['ff']
                            fs_array = stream['fs']
                            size = stream['vsize']
                            break
                    push_stream_bid(_bid, container, fs_array, size)

        info.stream_types = sorted(info.stream_types, key=self.ids.index)
        return info

    def prepare_list(self):
        html = get_content(self.url)

        return matchall(html, ['data-tvid=\"([^\"]+)\" data-vid=\"([^\"]+)\"'])

site = Iqiyi()
