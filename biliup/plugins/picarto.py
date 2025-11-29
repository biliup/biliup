import re
from typing import AsyncGenerator, List

from . import logger
from ..common.util import client
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase, BatchCheck

VALID_URL_BASE = r"(?:https?://)?(?:www\.)?picarto\.tv/(?P<id>[^/?&]+)"
API_CHANNEL = "https://ptvintern.picarto.tv/api/channel/detail/{username}"
API_EXPLORE = "https://ptvintern.picarto.tv/api/explore?first=100&page={page}&filter_params%5Badult%5D=true&order_by%5Bfield%5D=viewers&order_by%5Border%5D=DESC&type=stream"
CHANNEL_URL = "https://picarto.tv/{user_name}"
HLS_URL = "https://{netloc}.picarto.tv/stream/hls/{file_name}/index.m3u8"

@Plugin.download(regexp=VALID_URL_BASE)
class Picarto(DownloadBase, BatchCheck):

    def __init__(self, fname, url, config, suffix="flv"):
        super().__init__(fname, url, config, suffix)

    async def acheck_stream(self, is_check=False):
        username = re.match(VALID_URL_BASE, self.url).group("id")
        channel_detail = (await client.get(
            API_CHANNEL.format(username=username), timeout=5
        )).json()
        channel = channel_detail.get("channel", {})
        loadbalancer = channel_detail.get("getLoadBalancerUrl", {})
        multistreams = channel_detail.get("getMultiStreams", {})

        # 檢查response
        if not channel or not multistreams or not loadbalancer:
            return False
        elif channel.get("private"):
            logger.warning("This is a private stream")
            return False

        user_id = channel.get("id")
        if not (
            stream := next(
                (
                    stream
                    for stream in multistreams.get("streams")
                    if stream.get("channelId") == user_id
                ),
                None,
            )
        ):
            logger.warning("No available stream found in 'multistreams' data")
            return False

        self.room_title = channel.get("title")
        self.live_cover_url = stream.get("thumbnail_image")
        if is_check:
            return True

        self.raw_stream_url = HLS_URL.format(
            netloc=loadbalancer.get("origin"), file_name=stream.get("stream_name")
        )
        return True

    @staticmethod
    async def abatch_check(check_urls: List[str]) -> AsyncGenerator[str, None]:
        explore = []
        page = 1

        while True:
            explore_detail = (await client.get(API_EXPLORE.format(page=page), timeout=5)).json()
            explore += explore_detail.get("data", [])
            if explore_detail.get("next_page_url"):
                page += 1
            else:
                break

        for data in explore:
            if (
                channel := CHANNEL_URL.format(user_name=data.get("name"))
            ) in check_urls:
                yield channel
