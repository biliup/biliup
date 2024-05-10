import time
import random

import biliup.common.util
from biliup.config import config
from ..common import tools
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import logger


@Plugin.download(regexp=r'(?:https?://)?(?:(?:live|www|v)\.)?(kuaishou)\.com')
@Plugin.download(regexp=r'(?:https?://)?(?:(?:(?:livev)\.(?:m))\.)?chenzhongtech\.com')
class Kuaishou(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.fake_headers['Cookie'] = config.get('kuaishou_cookie', '')

    async def acheck_stream(self, is_check=False):
        try:
            room_id = get_kwaiId(self.url)
            if not room_id:
                logger.warning(f"Kuaishou - {self.url}: 直播间地址错误")
                return False
        except Exception as e:
            logger.error(f"Kuaishou - {self.url}: {e}")
            return False

        plugin_msg = f"Kuaishou - {room_id}"

        # with requests.Session() as s:
        biliup.common.util.client.headers = self.fake_headers.copy()
        # 首页低风控生成did
        await biliup.common.util.client.get("https://live.kuaishou.com", timeout=5)

        # 不暂停似乎容易风控
        times = 3 + random.random()
        logger.debug(f"{plugin_msg}: 暂停 {times} 秒")
        time.sleep(times)

        err_keys = ["错误代码22", "主播尚未开播"]
        html = (await biliup.common.util.client.get(f"https://live.kuaishou.com/u/{room_id}", timeout=5)).text
        for key in err_keys:
            if key in html:
                logger.debug(f"{plugin_msg}: {key}")
                return False

        room_info = (await biliup.common.util.client.get(
            f"https://live.kuaishou.com/live_api/liveroom/livedetail?principalId={room_id}",
            timeout=5)).json()['data']

        if room_info['result'] == 22:
            logger.error(f"{plugin_msg}: 直播间地址错误")
            return False
        if room_info['result'] == 671:
            logger.debug(f"{plugin_msg}: 直播间未开播或非直播")
            return False
        if room_info['result'] != 1:
            logger.error(f"{plugin_msg}: {room_info}")
            return False

        if is_check:
            return True

        try:
            self.room_title = room_info['liveStream']['caption']
        except KeyError:
            logger.warning(f"{plugin_msg}: 直播间标题获取失败，使用快手ID代替")
            self.room_title = room_id
        self.raw_stream_url = room_info['liveStream']['playUrls'][0]['adaptationSet']['representation'][-1]['url']

        return True


def get_kwaiId(url):
    split_args = ["/profile/", "/fw/live/", "/u/"]
    for key in split_args:
        if key in url:
            kwaiId = url.split(key)[1]
            return kwaiId
