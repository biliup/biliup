import random
import string
import json
import requests

import biliup.common.util
from . import logger
from ..common import tools
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m|live)\.)?acfun\.cn')
class Acfun(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    async def acheck_stream(self, is_check=False):
        if len(self.url.split("acfun.cn/live/")) < 2:
            logger.debug("直播间地址错误")
            return False
        rid = self.url.split("acfun.cn/live/")[1]
        did = "web_"+get_random_name(16)
        cookies = dict(_did=did)
        data1 = {'sid': 'acfun.api.visitor'}
        r1 = await biliup.common.util.client.post("https://id.app.acfun.cn/rest/app/visitor/login",
                                                  headers=self.fake_headers, data=data1, cookies=cookies, timeout=5)
        userid = r1.json()['userId']
        visitorst = r1.json()['acfun.api.visitor_st']
        params = {
            "subBiz": "mainApp",
            "kpn": "ACFUN_APP",
            "kpf": "PC_WEB",
            "userId": str(userid),
            "did": did,
            "acfun.api.visitor_st": visitorst
        }
        data2 = {'authorId': rid, 'pullStreamType': 'FLV'}
        self.fake_headers['referer'] = "https://live.acfun.cn/"
        r2 = await biliup.common.util.client.post("https://api.kuaishouzt.com/rest/zt/live/web/startPlay",
                                                  headers=self.fake_headers, data=data2, params=params, timeout=5)
        if r2.json().get('result') != 1:
            logger.debug(r2.json())
            return False
        d = r2.json()['data']['videoPlayRes']
        self.raw_stream_url = json.loads(d)['liveAdaptiveManifest'][0]['adaptationSet']['representation'][-1]['url']
        self.room_title = r2.json()['data']['caption']
        return True


def get_random_name(numb):
    return random.choice(string.ascii_lowercase) + \
        ''.join(random.sample(string.ascii_letters + string.digits, numb - 1))
