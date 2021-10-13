import json
import urllib.request

import requests

from . import logger
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m|live)\.)?douyin\.com')
class Douyin(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        if len(self.url.split("live.douyin.com/")) < 2:
            logger.debug("直播间地址错误")
            return False
        rid = self.url.split("live.douyin.com/")[1]
        headers = {
            "user-agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) "
                          "Chrome/94.0.4606.71 Safari/537.36 Edg/94.0.992.38",
            "referer": "https://live.douyin.com/"
        }
        try:
            r1 = requests.get('https://live.douyin.com/' + rid, headers=headers).text \
                .split('<script id="RENDER_DATA" type="application/json">')[1].split('</script>')[0]
        except IndexError:
            logger.debug("连接异常")
            return False
        r2 = urllib.request.unquote(r1)
        try:
            r3 = json.loads(r2)['routeInitialProps']['error']
            if r3:
                logger.debug("直播间不存在")
                return False
        except KeyError:
            logger.debug("直播间存在")
        try:
            r5 = json.loads(r2)['initialState']['roomStore']['roomInfo']['room']['stream_url']['flv_pull_url']
            i = 0
            for k in r5:
                if i < 1:
                    r6 = k
                    i = i + 1
            self.raw_stream_url = r5[r6]
            return True
        except KeyError:
            logger.debug("主播未开播")
            return False
