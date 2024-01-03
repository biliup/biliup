import requests

from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import logger


@Plugin.download(regexp=r'(?:https?://)?www\.bigo\.tv')
class Bigo(DownloadBase):
    def __init__(self, fname, url, suffix='ts'):
        super().__init__(fname, url, suffix)

    def check_stream(self, is_check=False):
        try:
            room_id = self.url.split('/')[-1].split('?')[0]
        except:
            logger.warning(f"{Bigo.__name__}: {self.url}: 直播间地址错误")
            return False

        try:
            room_info = requests.post(f'https://ta.bigo.tv/official_website/studio/getInternalStudioInfo', timeout=10,
                                      headers={
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/119.0',
        'Accept-Language': 'zh-CN,zh;q=0.8,zh-TW;q=0.7,zh-HK;q=0.5,en-US;q=0.3,en;q=0.2',
        'Content-Type': 'application/x-www-form-urlencoded; charset=UTF-8',
        'Referer': 'https://www.bigo.tv/',}, data={"siteId": room_id}).json()
            if room_info['code'] != 0:
                raise
        except:
            logger.warning(f"{Bigo.__name__}: {self.url}: 获取错误，本次跳过")
            return False

        try:
            if room_info['data']['alive'] is None:
                raise
        except:
                logger.warning(f"{Bigo.__name__}: {self.url}: 直播间不存在")
                return False
        try:
            if room_info['data']['alive'] != 1 or not room_info['data']['hls_src']:
                raise
        except:
                logger.debug(f"{Bigo.__name__}: {self.url}: 直播间未开播")
                return False
        try:
            self.raw_stream_url = room_info['data']['hls_src']
            self.room_title = room_info['data']['roomTopic']
        except:
            logger.warning(f"{Bigo.__name__}: {self.url}: 解析错误")
            return False

        return True
