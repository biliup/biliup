import json
import re

import biliup.common.util
from . import logger
from ..common import tools
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'(?:https?://)?(?:now\.)?qq\.com')
class now(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    async def acheck_stream(self, is_check=False):
        logger.debug(self.fname)
        rid = re.search(r'roomid=([a-zA-Z0-9]+)', self.url).group(1)
        r1 = await biliup.common.util.client.get(
            f"https://now.qq.com/cgi-bin/now/web/room/get_room_info_v2?room_id={rid}",
            timeout=5,
            headers=self.fake_headers
        )
        # r1.close()
        jsons = json.loads(r1.text)
        if jsons:
            if jsons.get('retcode') == 100001:
                logger.error("直播间地址错误")
                return False
            if jsons['result']['is_on_live']:
                self.room_title = jsons['result']['room_name']
                r2 = await biliup.common.util.client.get(
                    f"https://now.qq.com/cgi-bin/now/web/room/get_live_room_url?platform=8&room_id={rid}",
                    timeout=5,
                    headers=self.fake_headers
                )
                # r2.close()
                self.raw_stream_url = json.loads(r2.text)['result']['raw_flv_url']
                return True

        logger.debug("主播未开播")
        return False
