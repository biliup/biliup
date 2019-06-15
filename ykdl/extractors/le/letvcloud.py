#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1, matchall
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo

import json
import base64, hashlib, time

class Letvcloud(VideoExtractor):
    name = u"乐视云 (Letvcloud)"

    supported_stream_types = ['yuanhua', 'super', 'high', 'low']
    types_2_format = {'yuanhua' : 'BD', 'super' : 'TD', 'high' : 'HD', 'low' : 'SD'}
    types_2_profile = {'yuanhua' : u'原画', 'super' : u'超清', 'high' : u'高清', 'low' : u'标清'}

    def letvcloud_download_by_vu(self):
        info = VideoInfo(self.name)
        #ran = float('0.' + str(random.randint(0, 9999999999999999))) # For ver 2.1
        #str2Hash = 'cfflashformatjsonran{ran}uu{uu}ver2.2vu{vu}bie^#@(%27eib58'.format(vu = vu, uu = uu, ran = ran)  #Magic!/ In ver 2.1
        vu, uu = self.vid
        argumet_dict ={'cf' : 'flash', 'format': 'json', 'ran': str(int(time.time())), 'uu': str(uu),'ver': '2.2', 'vu': str(vu), }
        sign_key = '2f9d6924b33a165a6d8b5d3d42f4f987'  #ALL YOUR BASE ARE BELONG TO US
        str2Hash = ''.join([i + argumet_dict[i] for i in sorted(argumet_dict)]) + sign_key
        sign = hashlib.md5(str2Hash.encode('utf-8')).hexdigest()
        html = get_content('http://api.letvcloud.com/gpc.php?' + '&'.join([i + '=' + argumet_dict[i] for i in argumet_dict]) + '&sign={sign}'.format(sign = sign), charset= 'utf-8')
        data = json.loads(html)
        assert data['code'] == 0, data['message']
        video_name = data['data']['video_info']['video_name']
        if '.' in video_name:
            ext = video_name.split('.')[-1]
            info.title = video_name[0:-len(ext)-1]
        else:
            ext = 'mp4'
            info.title = video_name
        available_stream_type = data['data']['video_info']['media'].keys()
        for stream in self.supported_stream_types:
            if stream in available_stream_type:
                urls = [base64.b64decode(data['data']['video_info']['media'][stream]['play_url']['main_url']).decode("utf-8")]
                info.stream_types.append(self.types_2_format[stream])
                info.streams[self.types_2_format[stream]] = {'container': ext, 'video_profile': self.types_2_profile[stream], 'src': urls, 'size' : 0}
        return info

    def prepare(self):

        if self.url and not self.vid:
            #maybe error!!
            self.vid = (vu, uu) = matchall(self.url, ["vu=([^&]+)","uu=([^&]+)"])
        return self.letvcloud_download_by_vu()

site = Letvcloud()
