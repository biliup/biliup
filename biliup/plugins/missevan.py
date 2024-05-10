import biliup.common.util
from . import match1, logger
from ..common import tools
# from biliup.config import config
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase

@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|fm)\.)?missevan\.com')
class Missevan(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    async def acheck_stream(self, is_check=False):
        rid = 0
        # 用户主页获取直播间地址
        if self.url.split('www'):
            user_page = await biliup.common.util.client.get(self.url, timeout=30, headers=self.fake_headers)
            # 取硬编码在网页内的直播间号
            if user_page.status_code == 200:
                start = user_page.text.find('data-id="') + 9
                end = user_page.text.find('"', start)
                rid = user_page.text[start:end]
            else:
                logger.debug(user_page.status_code)
        if self.url.split("live"):
            rid = match1(self.url, r'/(\d+)')

        room_info = (await biliup.common.util.client.get(f"https://fm.missevan.com/api/v2/live/{rid}", timeout=30, headers=self.fake_headers)).json()

        # 无直播间的情况
        if room_info['code'] != 0:
            logger.debug(room_info['info'])
            return False

        # 开播状态
        if room_info['info']['room']['status']['open'] == 0:
            creator_username = room_info['info']['room']['creator_username']
            logger.debug(f"主播{creator_username}未开播")
            return False

        self.room_title = room_info['info']['room']['name']
        # if (config.get('missevanChannel') == 'flv'):
        #     self.raw_stream_url = room_info['info']['room']['channel']['flv_pull_url']
        # else:
        #     self.raw_stream_url = room_info['info']['room']['channel']['hls_pull_url']
        self.raw_stream_url = room_info['info']['room']['channel']['flv_pull_url']
        return True
