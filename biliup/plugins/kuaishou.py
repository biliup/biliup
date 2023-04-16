import json
import requests

from ..engine.decorators import Plugin
from ..plugins import logger
from ..engine.download import DownloadBase

@Plugin.download(regexp=r'(?:https?://)?(?:(?:live|www)\.)?kuaishou\.com')
class Kuaishou(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        with requests.Session() as s:
            if "/profile/" in self.url:
                self.url = f"https://live.kuaishou.com/u/{self.url.split('/profile/')[1]}"
            res = s.get(self.url, timeout=5, headers=self.fake_headers)
        initial_state = res.text.split('window.__INITIAL_STATE__=')[1].split(';(')[0]
        liveroom = json.loads(initial_state)['liveroom']
        if liveroom['errorType']['type'] != 1:
            logger.error("直播间不存在或链接错误")
            return False
        liveStream = liveroom['liveStream']
        if not liveroom['isLiving'] or liveStream['type'] not in 'live':
            logger.error("直播间未开播或播放的不是直播")
            return False
        self.raw_stream_url = liveStream['playUrls'][0]['adaptationSet']['representation'][-1]['url']
        self.room_title = liveStream['caption']
        # author = liveroom['author']
        # if self.use_live_cover is True:
        #     try:
        #         self.live_cover_path = \
        #         super().get_live_cover(author['name'], \
        #                                author['id'], \
        #                                self.room_title, \
        #                                author['timestamp'], \
        #                                liveStream['coverUrl'])
        #     except:
        #         logger.error(f"获取直播封面失败")
        return True