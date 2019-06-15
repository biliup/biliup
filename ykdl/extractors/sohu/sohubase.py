#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.compact import urlparse, urlencode

import json
import time
from random import random

'''
Changelog:
    1. http://tv.sohu.com/upload/swf/20150604/Main.swf
        new api
'''


class SohuBase(VideoExtractor):

    supported_stream_types = [
        #'h2654kVid',
        #'h2654mVid',
        #'h265oriVid',
        #'h265superVid',
        #'h265highVid',
        #'h265norVid',
        'h2644kVid',
        'oriVid',
        'superVid',
        'highVid',
        'norVid'
        ]
    types_2_id = {
        'h2654kVid': '4k',
        'h2654mVid': '4k',
        'h2644kVid': '4k',
        'h265oriVid': 'BD',
        'h265superVid': 'TD',
        'h265highVid': 'HD',
        'h265norVid': 'SD',
        'oriVid': 'BD',
        'superVid': 'TD',
        'highVid': 'HD',
        'norVid': 'SD'
        }
    id_2_profile = { '4k': u'4K', 'BD': u'原画', 'TD': u'超清', 'HD': u'高清', 'SD': u'标清' }

    def parser_info(self, video, info, stream, lvid, uid):
        if not 'allot' in info or lvid != info['id']:
            return
        stream_id = self.types_2_id[stream]
        stream_profile = self.id_2_profile[stream_id]
        host = info['allot']
        prot = info['prot']
        tvid = info['tvid']
        data = info['data']
        size = sum(map(int,data['clipsBytes']))
        urls = []
        assert len(data['clipsURL']) == len(data['clipsBytes']) == len(data['su'])
        for new, clip, ck, in zip(data['su'], data['clipsURL'], data['ck']):
            params = {
                'vid': self.vid,
                'tvid': tvid,
                'file': urlparse(clip).path,
                'new': new,
                'key': ck,
                'uid': uid,
                't': random(),
                'prod': 'h5',
                'prot': prot,
                'pt': 1,
                'rb': 1,
            }
            if urlparse(new).netloc == '':
                cdnurl = 'https://'+host+'/cdnList?' + urlencode(params)
                url = json.loads(get_content(cdnurl))['url']
            else:
                url = new
            urls.append(url)
        video.streams[stream_id] = {'container': 'mp4', 'video_profile': stream_profile, 'src': urls, 'size' : size}
        video.stream_types.append(stream_id)

    def prepare(self):
        if self.url and not self.vid:
            self.vid = match1(self.url, '\Wvid=(\d+)', '\Wid=(\d+)', 'share_play.html#(\d+)_')
            if not self.vid:
                html = get_content(self.url)
                self.vid = match1(html, '/(\d+)/v\.swf', 'vid="(\d+)"', '\&id=(\d+)')
        self.logger.debug("VID> {}".format(self.vid))

        info = json.loads(get_content(self.apiurl % self.vid))
        self.logger.debug("info> {}".format(info))
        if info['status'] == 6:
            self.name = u'搜狐自媒体 (MySohu)'
            self.apiurl = 'http://my.tv.sohu.com/play/videonew.do?vid=%s&referer=http://my.tv.sohu.com'
            info = json.loads(get_content(self.apiurl % self.vid))
            self.logger.debug("info> {}".format(info))

        video = VideoInfo(self.name)
        # this is needless now, uid well be registered in the the following code
        #video.extra["header"] = "Range: "
        if info['status'] == 1:
            now = time.time()
            uid = int(now * 1000)
            params = {
                'vid': self.vid,
                'url': self.url,
                'refer': self.url,
                't': int(now),
                'uid': uid,
                #'nid': nid,
                #'pid': pid,
                #'screen': '1366x768',
                #'channeled': channeled,
                #'MTV_SRC': MTV_SRC,
                #'position': 'page_adbanner',
                #'op': 'click',
                #'details': '{}',
                #'os': 'linux',
                #'platform': 'linux',
                #'passport': '',
            }
            get_content('http://z.m.tv.sohu.com/h5_cc.gif?' + urlencode(params))

            data = info['data']
            video.title = data['tvName']
            for stream in self.supported_stream_types:
                lvid = data.get(stream)
                if lvid == 0 or not lvid:
                    continue
                if lvid != self.vid:
                    _info = json.loads(get_content(self.apiurl % lvid))
                    self.logger.debug("info> {}".format(_info))
                else:
                    _info = info

                self.parser_info(video, _info, stream, lvid, uid)
        return video
