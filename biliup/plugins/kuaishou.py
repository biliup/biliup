import requests

from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import logger


@Plugin.download(regexp=r'(?:https?://)?(?:(?:live|www|v)\.)?(kuaishou)\.com')
@Plugin.download(regexp=r'(?:https?://)?(?:(?:(?:livev)\.(?:m))\.)?chenzhongtech\.com')
class Kuaishou(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self, is_check=False):
        try:
            room_id = get_kwaiId(self.url)
            if not room_id:
                logger.warning(f"{Kuaishou.__name__}: {self.url}: 直播间地址错误")
                return False

            with requests.Session() as s:
                s.headers = self.fake_headers.copy()
                # 首页低风控生成did
                s.get("https://live.kuaishou.com", timeout=5)

                room_info = s.get(
                    f"https://live.kuaishou.com/live_api/liveroom/livedetail?principalId={room_id}",
                    timeout=5).json()['data']

            if room_info['result'] == 22:
                logger.warning(f"{Kuaishou.__name__}: {self.url}: 直播间地址错误")
                return False
            if room_info['result'] == 671:
                logger.debug(f"{Kuaishou.__name__}: {self.url}: 直播间未开播或非直播")
                return False
            if room_info['result'] != 1:
                logger.warning(f"{Kuaishou.__name__}: {self.url}: 错误{room_info['result']}")
                return False

            self.room_title = room_info['liveStream']['caption']
            self.raw_stream_url = room_info['liveStream']['playUrls'][0]['adaptationSet']['representation'][-1]['url']
        except:
            logger.warning(f"{Kuaishou.__name__}: {self.url}: 获取错误，本次跳过")
            return False

        return True


def get_kwaiId(url):
    split_args = ["/profile/", "/fw/live/", "/u/"]
    for key in split_args:
        if key in url:
            kwaiId = url.split(key)[1]
            return kwaiId
