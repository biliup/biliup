import asyncio
import subprocess
import sys
from collections import namedtuple
import time
from urllib.parse import urlparse

import requests

from ykdl.util.match import match1
from biliup.config import config
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase, get_valid_filename, stream_gears_download
from ..plugins import logger
from .Danmaku.danmaku_main import Danmaku


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m)\.)?douyu\.com')
class Douyu(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.douyu_danmaku = config.get('douyu_danmaku', False)

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
        if not vid:
            logger.debug("直播间" + vid + "：被关闭或不存在")
            return False
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
        html_content = requests.post(f'https://www.douyu.com/lapi/live/getH5Play/{vid}', headers=self.fake_headers,
                                     params=params).json()
        live_data = html_content["data"]
        if type(live_data) is dict:
            self.raw_stream_url = f"{live_data.get('rtmp_url')}/{live_data.get('rtmp_live')}"
            self.room_title = roominfo['room_name']
            return True

    def danmaku_download_start(self, filename):
        if self.douyu_danmaku:
            self.danmaku = None
            self.danmaku = Danmaku(filename, self.url)
            self.danmaku.start()

    def danmaku_download_stop(self):
        if self.douyu_danmaku:
            asyncio.run(self.danmaku.stop())
