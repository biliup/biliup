import hashlib
import time
from urllib.parse import parse_qs
from functools import lru_cache

from biliup.common.util import client
from biliup.config import config
from biliup.Danmaku import DanmakuClient
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import logger, match1, random_user_agent


@Plugin.download(regexp=r'https?://(?:(?:www|m)\.)?douyu\.com')
class Douyu(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.__room_id = match1(url, r'rid=(\d+)')
        self.douyu_danmaku = config.get('douyu_danmaku', False)
        self.douyu_disable_interactive_game = config.get('douyu_disable_interactive_game', False)
        self.douyu_cdn = config.get('douyu_cdn', 'tct-h5')
        self.douyu_rate = config.get('douyu_rate', 0)

    async def acheck_stream(self, is_check=False):
        if len(self.url.split("douyu.com/")) < 2:
            logger.error(f"{self.plugin_msg}: 直播间地址错误")
            return False

        try:
            if not self.__room_id:
                self.__room_id = _get_real_rid(self.url)
        except:
            logger.exception(f"{self.plugin_msg}: 获取房间号错误")
            return False

        try:
            room_info = (
                await client.get(f"https://www.douyu.com/betard/{self.__room_id}", headers=self.fake_headers)
            ).json()['room']
        except:
            logger.exception(f"{self.plugin_msg}: 获取直播间信息错误")
            return False

        if room_info['show_status'] != 1:
            logger.debug(f"{self.plugin_msg}: 未开播")
            return False
        if room_info['videoLoop'] != 0:
            logger.debug(f"{self.plugin_msg}: 正在放录播")
            return False
        if self.douyu_disable_interactive_game:
            gift_info = (
                await client.get(f"https://www.douyu.com/api/interactive/web/v2/list?rid={self.__room_id}",
                                headers=self.fake_headers)
            ).json().get('data', {})
            if gift_info:
                logger.debug(f"{self.plugin_msg}: 正在运行互动游戏")
                return False
        self.room_title = room_info['room_name']

        if is_check:
            try:
                import jsengine
                try:
                    jsengine.jsengine()
                except jsengine.exceptions.RuntimeError as e:
                    extra_msg = "如需录制斗鱼直播，"
                    logger.error(f"\n{e}\n{extra_msg}请至少安装一个 Javascript 解释器，如 pip install quickjs")
                    return False
            except:
                logger.exception(f"{self.plugin_msg}: ")
                return False
            return True

        try:
            import jsengine
            ctx = jsengine.jsengine()
            js_enc = (
                await client.get(f'https://www.douyu.com/swf_api/homeH5Enc?rids={self.__room_id}',
                                 headers=self.fake_headers)
            ).json()['data'][f'room{self.__room_id}']
            js_enc = js_enc.replace('return eval', 'return [strc, vdwdae325w_64we];')

            sign_fun, sign_v = ctx.eval(f'{js_enc};ub98484234();')

            tt = str(int(time.time()))
            did = hashlib.md5(tt.encode('utf-8')).hexdigest()
            rb = hashlib.md5(f"{self.__room_id}{did}{tt}{sign_v}".encode('utf-8')).hexdigest()
            sign_fun = sign_fun.rstrip(';').replace("CryptoJS.MD5(cb).toString()", f'"{rb}"')
            sign_fun += f'("{self.__room_id}","{did}","{tt}");'

            params = parse_qs(ctx.eval(sign_fun))
        except:
            logger.exception(f"{self.plugin_msg}: 获取签名参数异常")
            return False

        params['cdn'] = self.douyu_cdn
        params['rate'] = int(self.douyu_rate)

        try:
            live_data = await self.get_play_info(self.__room_id, params)
            self.raw_stream_url = f"{live_data['rtmp_url']}/{live_data['rtmp_live']}"
        except:
            logger.exception(f"{self.plugin_msg}: ")
            return False

        return True

    def danmaku_init(self):
        if self.douyu_danmaku:
            content = {
                'room_id': self.__room_id,
            }
            self.danmaku = DanmakuClient(self.url, self.gen_download_filename(), content)

    async def get_play_info(self, room_id, params):
        live_data = await client.post(
            f'https://www.douyu.com/lapi/live/getH5Play/{room_id}', headers=self.fake_headers, params=params)
        if not live_data.is_success:
            raise RuntimeError(live_data.text)
        live_data = live_data.json().get('data')
        if isinstance(live_data, dict):
            if not live_data['rtmp_cdn'].endswith('h5') or 'akm' in live_data['rtmp_cdn']:
                params['cdn'] = live_data['cdnsWithName'][-1]['cdn']
                return await self.get_play_info(room_id, params)
            return live_data
        raise RuntimeError(live_data)

@lru_cache(maxsize=None)
def _get_real_rid(url):
    import requests
    headers = {
        "user-agent": random_user_agent('mobile'),
    }
    rid = url.split('douyu.com/')[1].split('/')[0].split('?')[0] or match1(url, r'douyu.com/(\d+)')
    resp = requests.get(f"https://m.douyu.com/{rid}", headers=headers)
    real_rid = match1(resp.text, r'roomInfo":{"rid":(\d+)')
    return real_rid