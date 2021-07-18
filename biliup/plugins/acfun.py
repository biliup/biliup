import random
import string
import json
import requests
from . import logger
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m|live)\.)?acfun\.cn')
class Acfun(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        if len(self.url.split("acfun.cn/live/")) < 2:
            logger.debug("直播间地址错误")
            return False
        rid = self.url.split("acfun.cn/live/")[1]
        did = "web_"+get_random_name(16)
        headers1 = {
            "user-agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
                          "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36 Edg/91.0.864.67"
        }
        cookies1 = dict(_did=did)
        data1 = {'sid': 'acfun.api.visitor'}
        r1 = requests.post("https://id.app.acfun.cn/rest/app/visitor/login",
                           headers=headers1, data=data1, cookies=cookies1)
        userId = r1.json()['userId']
        visitorSt = r1.json()['acfun.api.visitor_st']
        params = {
            "subBiz": "mainApp",
            "kpn": "ACFUN_APP",
            "kpf": "PC_WEB",
            "userId": str(userId),
            "did": did,
            "acfun.api.visitor_st": visitorSt
        }
        data2 = {'authorId': rid, 'pullStreamType': 'FLV'}
        headers2 = {
            "user-agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
                          "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36 Edg/91.0.864.67",
            "Referer": "https://live.acfun.cn/"
        }
        r2 = requests.post("https://api.kuaishouzt.com/rest/zt/live/web/startPlay",
                           headers=headers2, data=data2, params=params)
        if r2.json()['result'] != 1:
            logger.debug(r2.json()['error_msg'])
            return False
        d = r2.json()['data']['videoPlayRes']
        e = json.loads(d)['liveAdaptiveManifest'][0]['adaptationSet']['representation']
        self.raw_stream_url = e[-1]['url']
        return True


def get_random_name(l):
    return random.choice(string.ascii_lowercase) + \
        ''.join(random.sample(string.ascii_letters + string.digits, l - 1))
