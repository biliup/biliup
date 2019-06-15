#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content, add_header
from ykdl.util.match import match1, matchall
from ykdl.util.jsengine import JSEngine, javascript_is_supported
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.compact import urlencode

import time
import json
import uuid
import random
import string


douyu_match_pattern = [ 'class="hroom_id" value="([^"]+)',
                        'data-room_id="([^"]+)'
                      ]

def get_random_name(l):
    return random.choice(string.ascii_lowercase) + \
           ''.join(random.sample(string.ascii_letters + string.digits, l - 1))

class Douyutv(VideoExtractor):
    name = u'斗鱼直播 (DouyuTV)'

    stream_ids = ['BD10M', 'BD8M', 'BD4M', 'BD', 'TD', 'HD', 'SD']
    profile_2_id = {
        u'蓝光10M': 'BD10M',
        u'蓝光8M': 'BD8M',
        u'蓝光4M': 'BD4M',
        u'蓝光': 'BD',
        u'超清': 'TD',
        u'高清': 'HD',
        u'流畅': 'SD'
     }

    def prepare(self):
        assert javascript_is_supported, "No JS Interpreter found, can't parse douyu live!"

        info = VideoInfo(self.name, True)
        add_header("Referer", 'https://www.douyu.com')

        html = get_content(self.url)
        self.vid = match1(html, '\$ROOM\.room_id\s*=\s*(\d+)',
                                'room_id\s*=\s*(\d+)',
                                '"room_id.?":(\d+)',
                                'data-onlineid=(\d+)')
        title = match1(html, 'Title-headlineH2">([^<]+)<')
        artist = match1(html, 'Title-anchorName" title="([^"]+)"')

        if not title or not artist:
            html = get_content('https://open.douyucdn.cn/api/RoomApi/room/' + self.vid)
            room_data = json.loads(html)
            if room_data['error'] == 0:
                room_data = room_data['data']
                title = room_data['room_name']
                artist = room_data['owner_name']

        info.title = u'{} - {}'.format(title, artist)
        info.artist = artist

        html_h5enc = get_content('https://www.douyu.com/swf_api/homeH5Enc?rids=' + self.vid)
        data = json.loads(html_h5enc)
        assert data['error'] == 0, data['msg']
        js_enc = data['data']['room' + self.vid]

        try:
            # try load local .js file first
            # from https://cdnjs.com/libraries/crypto-js
            from pkgutil import get_data
            js_md5 = get_data(__name__, 'crypto-js-md5.min.js')
            if not isinstance(js_md5, str):
                js_md5 = js_md5.decode()
        except IOError:
            js_md5 = get_content('https://cdnjs.cloudflare.com/ajax/libs/crypto-js/3.1.9-1/crypto-js.min.js')

        names_dict = {
            'debugMessages': get_random_name(8),
            'decryptedCodes': get_random_name(8),
            'resoult': get_random_name(8),
            '_ub98484234': get_random_name(8),
            'workflow': match1(js_enc, 'function ub98484234\(.+?\Weval\((\w+)\);'),
        }
        js_dom = '''
        {debugMessages} = {{{decryptedCodes}: []}};
        if (!this.window) {{window = {{}};}}
        if (!this.document) {{document = {{}};}}
        '''.format(**names_dict)
        js_patch = '''
        {debugMessages}.{decryptedCodes}.push({workflow});
        var patchCode = function(workflow) {{
            var testVari = /(\w+)=(\w+)\([\w\+]+\);.*?(\w+)="\w+";/.exec(workflow);
            if (testVari && testVari[1] == testVari[2]) {{
                {workflow} += testVari[1] + "[" + testVari[3] + "] = function() {{return true;}};";
            }}
        }};
        patchCode({workflow});
        var subWorkflow = /(?:\w+=)?eval\((\w+)\)/.exec({workflow});
        if (subWorkflow) {{
            var subPatch = (
                "{debugMessages}.{decryptedCodes}.push('sub workflow: ' + subWorkflow);" +
                "patchCode(subWorkflow);"
            ).replace(/subWorkflow/g, subWorkflow[1]) + subWorkflow[0];
            {workflow} = {workflow}.replace(subWorkflow[0], subPatch);
        }}
        eval({workflow});
        '''.format(**names_dict)
        js_debug = '''
        var {_ub98484234} = ub98484234;
        ub98484234 = function(p1, p2, p3) {{
            try {{
                var resoult = {_ub98484234}(p1, p2, p3);
                {debugMessages}.{resoult} = resoult;
            }} catch(e) {{
                {debugMessages}.{resoult} = e.message;
            }}
            return {debugMessages};
        }};
        '''.format(**names_dict)
        js_enc = js_enc.replace('eval({workflow});'.format(**names_dict), js_patch)

        js_ctx = JSEngine()
        js_ctx.eval(js_md5)
        js_ctx.eval(js_dom)
        js_ctx.eval(js_enc)
        js_ctx.eval(js_debug)
        did = uuid.uuid4().hex
        tt = str(int(time.time()))
        ub98484234 = js_ctx.call('ub98484234', self.vid, did, tt)
        self.logger.debug('ub98484234: %s', ub98484234)
        ub98484234 = ub98484234[names_dict['resoult']]
        params = {
            'v': match1(ub98484234, 'v=(\d+)'),
            'did': did,
            'tt': tt,
            'sign': match1(ub98484234, 'sign=(\w{32})'),
            'cdn': '',
            'iar': 0,
            'ive': 0
        }

        def get_live_info(rate=0):
            params['rate'] = rate
            data = urlencode(params)
            if not isinstance(data, bytes):
                data = data.encode()
            html_content = get_content('https://www.douyu.com/lapi/live/getH5Play/{}'.format(self.vid), data=data)
            self.logger.debug(html_content)

            live_data = json.loads(html_content)
            if live_data['error']:
                return live_data['msg']

            live_data = live_data["data"]
            real_url = '{}/{}'.format(live_data['rtmp_url'], live_data['rtmp_live'])
            rate_2_profile = dict((rate['rate'], rate['name']) for rate in live_data['multirates'])
            video_profile = rate_2_profile[live_data['rate']]
            stream = self.profile_2_id[video_profile]
            if stream in info.streams:
                return
            info.stream_types.append(stream)
            info.streams[stream] = {
                'container': 'flv',
                'video_profile': video_profile,
                'src' : [real_url],
                'size': float('inf')
            }

            error_msges = []
            if rate == 0:
                rate_2_profile.pop(0, None)
                rate_2_profile.pop(live_data['rate'], None)
                for rate in rate_2_profile:
                    error_msg = get_live_info(rate)
                    if error_msg:
                        error_msges.append(error_msg)
            if error_msges:
                return ', '.join(error_msges)

        error_msg = get_live_info()
        assert len(info.stream_types), error_msg
        info.stream_types = sorted(info.stream_types, key=self.stream_ids.index)
        return info

    def prepare_list(self):

        html = get_content(self.url)
        return matchall(html, douyu_match_pattern)

site = Douyutv()
