import json
import re

import biliup.common.util
from . import logger
from ..common import tools
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www)\.)?inke\.cn')
class inke(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    async def acheck_stream(self, is_check=False):
        logger.debug(self.fname)
        rid = re.search(r'uid=([a-zA-Z0-9]+)', self.url).group(1)
        r1 = await biliup.common.util.client.get(
            f"https://webapi.busi.inke.cn/web/live_share_pc?uid={rid}",
            timeout = 5,
            headers = self.fake_headers
        )
        # r1.close()
        jsons = json.loads(r1.text)
        if jsons:
            if jsons.get('error_code') != 0:
                logger.error("直播间地址错误")
                return False
            if jsons['data']['status']:
                self.room_title = jsons['data']['live_name']
                self.raw_stream_url = jsons['data']['live_addr'][0]['stream_addr']
                return True

        logger.debug("主播未开播")
        return False
