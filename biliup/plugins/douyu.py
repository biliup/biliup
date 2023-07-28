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

    def check_stream(self, is_check=False):
        if len(self.url.split("douyu.com/")) < 2:
            logger.error("斗鱼：" + self.url + "：地址错误")
            return False

        try:
            html = requests.get(self.url).text
            vid = match1(html, r'\$ROOM\.room_id\s*=\s*(\d+)',
                        r'room_id\s*=\s*(\d+)',
                        r'"room_id.?":(\d+)',
                        r'data-onlineid=(\d+)')
            if not vid:
                logger.debug("斗鱼：" + self.url + "：被关闭或不存在")
                return False
        except:
            logger.warning("斗鱼：" + self.url + "：获取vid错误")
            return False

        try:
            roominfo = requests.get(f"https://www.douyu.com/betard/{vid}").json()['room']
            if roominfo['show_status'] != 1 or roominfo['videoLoop'] != 0:
                logger.debug("斗鱼：" + vid + "：未开播或正在放录播")
                return False
            self.room_title = roominfo['room_name']
            html_h5enc = requests.get(f'https://www.douyu.com/swf_api/homeH5Enc?rids={vid}').json()
            js_enc = html_h5enc['data']['room' + vid]
            params = {
                'cdn': config.get('douyucdn', 'tct-h5'),
                'iar': 0,
                'ive': 0,
                'rate': config.get('douyu_rate', 0),
            }
        except:
            logger.warning("斗鱼：" + vid + "：获取roominfo错误")
            return False

        try:
            ub98484234(js_enc, vid, params)
        except TypeError:
            logger.error("请安装至少一个 Javascript 解释器")
            return False
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

def get_play_info(vid, headers, params):
    try:
        html_content = requests.post(f'https://www.douyu.com/lapi/live/getH5Play/{vid}', headers=headers, params=params).json()
        live_data = html_content["data"]
    except:
        logger.warning(f"douyu：get_play_info：请求出错：https://www.douyu.com/lapi/live/getH5Play/{vid}")
        return None
    # 禁用斗鱼主线路
    if not live_data['rtmp_cdn'].endswith('h5') or 'akm' in live_data['rtmp_cdn']:
        params['cdn'] = 'tct-h5'
        return get_play_info(vid, headers, params)
    return live_data

def ub98484234(js_enc, vid, params):
    import jsengine
    import uuid
    import time
    from urllib.parse import parse_qs
    did = uuid.uuid4().hex
    tt = str(int(time.time()))
    crypto_js = requests.get('https://cdn.staticfile.org/crypto-js/4.1.1/crypto-js.min.js').text
    js = jsengine.JSEngine(js_enc + crypto_js)
    query = js.call('ub98484234', vid, did, tt)
    params.update({k: v[0] for k, v in parse_qs(query).items()})