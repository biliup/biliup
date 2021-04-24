import base64
import html
import json

import requests

from ..engine.decorators import Plugin
from ..plugins import match1, logger, fake_headers
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m)\.)?huya\.com')
class Huya(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        logger.debug(self.fname)
        res = requests.get(self.url, timeout=5, headers=fake_headers)
        res.close()
        huya = match1(res.text, '"stream": "([a-zA-Z0-9+=/]+)"')
        if huya:
            huyajson = json.loads(base64.b64decode(huya).decode())['data'][0]['gameStreamInfoList'][0]
            absurl = u'{}/{}.{}?{}'.format(
                huyajson["sFlvUrl"], huyajson["sStreamName"], huyajson["sFlvUrlSuffix"], huyajson["sFlvAntiCode"])
            self.raw_stream_url = html.unescape(absurl)
            return True
