import base64
import html
import json

import requests

from biliup.config import config
from ..engine.decorators import Plugin
from ..plugins import match1, logger
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m)\.)?huya\.com')
class Huya(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        logger.debug(self.fname)
        res = requests.get(self.url, timeout=5, headers=self.fake_headers)
        res.close()
        huya = None
        if match1(res.text, '"stream": "([a-zA-Z0-9+=/]+)"'):
            huya = base64.b64decode(match1(res.text, '"stream": "([a-zA-Z0-9+=/]+)"')).decode()
        elif match1(res.text, 'stream: ([\w\W]+)'):
            huya = res.text.split('stream: ')[1].split('};')[0].strip()
            if json.loads(huya)['vMultiStreamInfo']:
                huya = res.text.split('stream: ')[1].split('};')[0].strip()
            else:
                huya = None
        if huya:
            huyacdn = config.get('huyacdn') if config.get('huyacdn') else 'AL'
            huyajson1 = json.loads(huya)['data'][0]['gameStreamInfoList']
            huyajson2 = json.loads(huya)['vMultiStreamInfo']
            ratio = huyajson2[0]['iBitRate']
            ibitrate_list = []
            sdisplayname_list = []
            for key in huyajson2:
                ibitrate_list.append(key['iBitRate'])
                sdisplayname_list.append(key['sDisplayName'])
                if len(sdisplayname_list) > len(set(sdisplayname_list)):
                    ratio = max(ibitrate_list)
            huyajson = huyajson1[0]
            for cdn in huyajson1:
                if cdn['sCdnType'] == huyacdn:
                    huyajson = cdn
            absurl = f'{huyajson["sFlvUrl"]}/{huyajson["sStreamName"]}.{huyajson["sFlvUrlSuffix"]}?' \
                     f'{huyajson["sFlvAntiCode"]}'
            self.raw_stream_url = html.unescape(absurl) + "&ratio=" + str(ratio)
            self.room_title = json.loads(huya)['data'][0]['gameLiveInfo']['introduction']
            return True
