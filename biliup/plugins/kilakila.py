import json

from biliup.common.util import client
from . import match1, logger
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'https?://(live\.kilakila\.cn|www\.hongdoufm\.com)')
class Kilakila(DownloadBase):
    def __init__(self, fname, url, config, suffix='flv'):
        self._room_id: str = match1(url, r'(\d+)')
        self.kila_protocol = config.get('kila_protocol', 'hls')
        super().__init__(fname, url, config, suffix)

    async def acheck_stream(self, is_check=False):
        for path in['/PcLive/index/detail', '/room/']:
            if path in self.url:
                break
        else:
            logger.error(f"{self.plugin_msg}: Unsupported Type")
            return False
        self.fake_headers['referer'] = 'https://live.kilakila.cn/'
        try:
            r = await client.get(
                'https://live.kilakila.cn/LiveRoom/getRoomInfo',
                params={'roomId': self._room_id},
                headers=self.fake_headers
            )
            if r.status_code != 200:
                logger.debug(f"{self.plugin_msg}: {r.status_code}")
                return False
            r = r.json()
            if r['h']['code'] != 200:
                logger.debug(f"{self.plugin_msg}: {r}")
                return False
            if r['b']['status'] != 4:
                '''
                ROOM_STATUS: {
                    REST_ROOM: 0,
                    LIVE_ROOM: 4,
                    PREVIEW_ROOM: 1,
                    PLAYBACK_ROOM: 10,
                    REMOVE_ROOM: 19
                },
                '''
                logger.debug(f"{self.plugin_msg}: 未开播")
                return False
            self.room_title = r['b']['title']

            if is_check:
                logger.info(f"{self.plugin_msg}: 直播背景图链接: {r['b']['defaultBackgroundPicUrl']}")
                return True

            # 直播视频流为 320*240 黑色背景，可搭配背景图重新压制视频
            # self.background_pic_url = r['b']['defaultBackgroundPicUrl']
            self.live_cover_url = r['b']['backPic']
            if self.kila_protocol == 'flv':
                self.raw_stream_url = r['b']['flvPlayUrl']
            else:
                self.raw_stream_url = r['b']['hlsPlayUrl']
        except json.JSONDecodeError:
            logger.error(f"{self.plugin_msg}: {r.text}")
        except:
            logger.error(f"{self.plugin_msg}: 获取房间信息失败", exc_info=True)

        return True