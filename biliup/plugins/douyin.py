import requests
import urllib.request
import json
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
        r1 = requests.get('https://live.douyin.com/' + rid).text \
            .split('<script id="RENDER_DATA" type="application/json">')[1].split('</script>')[0]
        r2 = urllib.request.unquote(r1)
        r3 = json.loads(r2)['routeInitialProps']['errorType']
        if r3 != 'none':
            logger.debug("直播间不存在")
            return False
        r4 = json.loads(r2)['routeInitialProps']['roomInfo']['roomId']
        if r4 == '':
            logger.debug("未开播")
            return False
        r5 = json.loads(r2)['routeInitialProps']['roomInfo']['room']['stream_url']['flv_pull_url']
        i = 0
        for k in r5:
            if i < 1:
                r6 = k
                i = i + 1
        self.raw_stream_url = r5[r6]
        return True
