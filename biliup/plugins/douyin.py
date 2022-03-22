import json
import urllib.request

import requests

from . import logger
from .. import config
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
            "referer": "https://live.douyin.com/",
            "cookie": f"{config.get('douyin_cookie')}"
        }
        r1 = requests.get('https://live.douyin.com/' + rid, headers=headers).text \
            .split('<script id="RENDER_DATA" type="application/json">')[1].split('</script>')[0]
        r2 = urllib.request.unquote(r1)

        r3 = json.loads(r2)['routeInitialProps']
        if r3.get('error'):
            logger.debug("直播间不存在")
            return False
        else:
            r4 = json.loads(r2)['initialState']['roomStore']['roomInfo']
            if r4.get('room'):
                room_info = r4['room']
                if room_info.get('stream_url'):
                    r5 = room_info['stream_url']['flv_pull_url']
                    self.raw_stream_url = list(r5.values())[0]
                    self.room_title = room_info['title']
                    return True
