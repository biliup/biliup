from collections import namedtuple

import requests
from ykdl.util.match import match1

from biliup.config import config
from biliup.plugins.Danmaku import DanmakuClient
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import logger


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m)\.)?douyu\.com')
class Douyu(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.douyu_danmaku = config.get('douyu_danmaku', False)

    def check_stream(self):
        from ykdl.extractors.douyu.util import ub98484234
        if len(self.url.split("douyu.com/")) < 2:
            logger.error("直播间地址:" + self.url + " 错误")
            return False
        html = requests.get(self.url).text
        vid = match1(html, r'\$ROOM\.room_id\s*=\s*(\d+)',
                     r'room_id\s*=\s*(\d+)',
                     r'"room_id.?":(\d+)',
                     r'data-onlineid=(\d+)')
        if not vid:
            logger.error("直播间" + vid + "：被关闭或不存在")
            return False
        roominfo = requests.get(f"https://www.douyu.com/betard/{vid}").json()['room']
        if roominfo['show_status'] != 1 or roominfo['videoLoop'] != 0:
            logger.error("直播间" + vid + "：未开播或正在放录播")
            return False
        self.room_title = roominfo['room_name']
        html_h5enc = requests.get(f'https://www.douyu.com/swf_api/homeH5Enc?rids={vid}').json()
        js_enc = html_h5enc['data']['room' + vid]
        params = {
            'cdn': config.get('douyucdn', ''),
            'iar': 0,
            'ive': 0,
            'rate': 0,
        }
        Extractor = namedtuple('Extractor', ['vid', 'logger'])
        ub98484234(js_enc, Extractor(vid, logger), params)
        live_data = get_play_info(vid, self.fake_headers, params)
        if type(live_data) is not dict:
            return False
        self.raw_stream_url = f"{live_data.get('rtmp_url')}/{live_data.get('rtmp_live')}"
        return True

    async def danmaku_download_start(self, filename):
        if self.douyu_danmaku:
            logger.info("开始弹幕录制")
            self.danmaku = DanmakuClient(self.url, filename + "." + self.suffix)
            await self.danmaku.start()

    def close(self):
        if self.douyu_danmaku:
            self.danmaku.stop()
            logger.info("结束弹幕录制")

def get_play_info(vid, fake_headers, params):
    import random
    try:
        html_content = requests.post(f'https://www.douyu.com/lapi/live/getH5Play/{vid}', headers=fake_headers,
                                params=params).json()
        live_data = html_content["data"]
        # 尝试规避斗鱼自建scdn
        # scdn 仅在该省市的ISP首次访问上方API后才会新增，且在新增后两分钟内无流可用（404）
        if not live_data['rtmp_cdn'].endswith('h5'):
            while not params['cdn'].endswith('h5'):
                params['cdn'] = random.choice(live_data['cdnsWithName']).get('cdn')
            return get_play_info(vid, fake_headers, params)
    except Exception:
        return None
    return live_data