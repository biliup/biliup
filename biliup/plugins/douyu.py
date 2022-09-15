from collections import namedtuple
import requests

from ykdl.util.match import match1
from biliup.config import config
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import logger


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m)\.)?douyu\.com')
class Douyu(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        logger.debug(self.fname)
        from ykdl.extractors.douyu.util import ub98484234
        if len(self.url.split("douyu.com/")) < 2:
            logger.debug("直播间地址:" + self.url + " 错误")
            return False
        html = requests.get(self.url).text
        vid = match1(html, r'\$ROOM\.room_id\s*=\s*(\d+)',
                     r'room_id\s*=\s*(\d+)',
                     r'"room_id.?":(\d+)',
                     r'data-onlineid=(\d+)')
        roominfo = requests.get(f"https://www.douyu.com/betard/{vid}").json()['room']
        videoloop = roominfo['videoLoop']
        show_status = roominfo['show_status']
        if show_status != 1 or videoloop != 0:
            logger.debug("直播间" + vid + "：未开播或正在放录播")
            return False
        # tct-h5
        douyucdn = config.get('douyucdn') if config.get('douyucdn') else ''
        html_h5enc = requests.get(f'https://www.douyu.com/swf_api/homeH5Enc?rids={vid}').json()
        js_enc = html_h5enc['data']['room' + vid]
        params = {
            'cdn': douyucdn,
            'iar': 0,
            'ive': 0,
        }
        # print(js_enc)
        Extractor = namedtuple('Extractor', ['vid', 'logger'])
        ub98484234(js_enc, Extractor(vid, logger), params)
        params['rate'] = 0
        html_content = requests.post(f'https://www.douyu.com/lapi/live/getH5Play/{vid}', headers=self.fake_headers, params=params).json()
        live_data = html_content["data"]
        if type(live_data) is dict:
            self.raw_stream_url = f"{live_data.get('rtmp_url')}/{live_data.get('rtmp_live')}"
            self.room_title = roominfo['room_name']
            return True
