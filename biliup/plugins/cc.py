from biliup.common.util import client
from biliup.config import config
from . import logger, match1
from ..common import tools
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'https?://cc\.163\.com')
class CC(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.cc_protocol = config.get('cc_protocol', 'hls')

    async def acheck_stream(self, is_check=False):
        rid = match1(self.url, r"(\d{4,})")
        room_info = (await client.get(
            f"https://api.cc.163.com/v1/activitylives/anchor/lives?anchor_ccid={rid}",
            timeout=5,
            headers=self.fake_headers
        )).json()
        if len(room_info["data"][rid]) <= 1:
            logger.debug(f"{self.plugin_msg}: 未开播")
            return False

        if is_check:
            return True

        try:
            channel_id = room_info["data"][rid]["channel_id"]
            channel_info = (await client.get(
                f"https://cc.163.com/live/channel/?channelids={channel_id}",
                timeout=5,
                headers=self.fake_headers
            )).json()["data"][0]
            self.room_title = channel_info["title"]
            if self.cc_protocol == "hls":
                self.raw_stream_url = channel_info["sharefile"]
            else:
                original = {"vbr": 0}
                for level in channel_info["quickplay"]["resolution"].values():
                    original = level if level["vbr"] > original["vbr"] else original
                self.raw_stream_url = list(original["cdn"].values())[0]
        except:
            logger.exception(f"{self.plugin_msg}: ")
            return False

        return True
