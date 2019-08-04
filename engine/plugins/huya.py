import json
import requests

import engine.plugins
from engine.plugins import FFmpegdl
from common import logger

VALID_URL_BASE = r'(?:https?://)?(?:(?:www|m)\.)?huya\.com'

user_agent = {"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
                            " (KHTML, like Gecko) Chrome/64.0.3282.140 Safari/537.36 Edge/17.17134"}


class Huya(FFmpegdl):
    def __init__(self, fname, url, suffix='mp4'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        logger.debug(self.fname)
        res = requests.get(self.url, timeout=5, headers=user_agent)
        res.close()
        data = res.text
        huya = engine.plugins.match1(data, r'({"sCdnType":"TX".*?})')
        if huya:
            huyajson = json.loads(huya)
            self.ydl_opts["absurl"] = huyajson["sHlsUrl"] + '/' + huyajson["sStreamName"] + '.' + \
                huyajson["sHlsUrlSuffix"] + '?' + huyajson["sHlsAntiCode"]
            return True


__plugin__ = Huya
