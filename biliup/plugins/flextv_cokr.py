import biliup.common.util
from ..common import tools
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import logger, match1


@Plugin.download(regexp=r'(?:https?://)?www\.flextv\.co\.kr')
class FlexTvCoKr(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    async def acheck_stream(self, is_check=False):
        room_id = match1(self.url, r"/channels/(\d+)/live")
        if not room_id:
            logger.warning(f"{FlexTvCoKr.__name__}: {self.url}: 直播间地址错误")
        response = await biliup.common.util.client.get(f"https://api.flextv.co.kr/api/channels/{room_id}/stream?option=all",
                                                       timeout=5,
                                                       headers=self.fake_headers)
        if response.status_code != 200:
            if response.status_code == 400:
                logger.debug(f"{FlexTvCoKr.__name__}: {self.url}: 未开播或直播间不存在")
                return False
            else:
                logger.warning(f"{FlexTvCoKr.__name__}: {self.url}: 获取错误，本次跳过")
                return False

        room_info = response.json()
        self.room_title = room_info['title']
        self.live_cover_url = room_info['thumbUrl']
        if is_check:
            return True

        m3u8_content = (await biliup.common.util.client.get(room_info['sources'][0]['url'], timeout=5, headers=self.fake_headers)).text
        import m3u8
        m3u8_obj = m3u8.loads(m3u8_content)
        if m3u8_obj.is_variant:
            # 取码率最大的流
            max_ratio_stream = max(m3u8_obj.playlists, key=lambda x: x.stream_info.bandwidth)
            self.raw_stream_url = max_ratio_stream.uri
        else:
            logger.warning(f"{FlexTvCoKr.__name__}: {self.url}: 解析错误")
            return False

        return True
