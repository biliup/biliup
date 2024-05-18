import hashlib
import time
from urllib.parse import parse_qs

from biliup.common.util import client
from biliup.config import config
from biliup.plugins.Danmaku import DanmakuClient
from ..common import tools
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import logger, match1
from httpx._exceptions import *


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m)\.)?douyu\.com')
class Douyu(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.douyu_danmaku = config.get('douyu_danmaku', False)

    async def acheck_stream(self, is_check=False):
        if len(self.url.split("douyu.com/")) < 2:
            logger.error(f"{self.plugin_msg}: 直播间地址错误")
            return False

        try:
            if 'm.douyu.com' in self.url:
                room_id = self.url.split('m.douyu.com/')[1].split('/')[0].split('?')[0]
            else:
                resp = await client.get(self.url, headers=self.fake_headers, timeout=5)
                room_id = match1(resp.text, r'\$ROOM\.room_id\s*=\s*(\d+)', r'apm_room_id\s*=\s*(\d+)')[0]
            if not room_id:
                logger.error(f"{self.plugin_msg}: 直播间不存在或已关闭")
                return False
        except:
            logger.exception(f"{self.plugin_msg}: 获取房间号错误")
            return False

        try:
            room_info = (
                await client.get(f"https://www.douyu.com/betard/{room_id}",
                                 headers=self.fake_headers, timeout=5)
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
        self.room_title = room_info['room_name']

        if is_check:
            try:
                import jsengine
                try:
                    ctx = jsengine.jsengine()
                except jsengine.exceptions.RuntimeError as e:
                    logger.error(f"{e}\n请至少安装一个 Javascript 解释器，如 pip install quickjs")
                    return False
            except:
                logger.exception(f"{self.plugin_msg}: ")
                return False
            return True

        try:
            import jsengine
            ctx = jsengine.jsengine()
            js_enc = (
                await client.get(f'https://www.douyu.com/swf_api/homeH5Enc?rids={room_id}',
                                headers=self.fake_headers, timeout=5)
                     ).json()['data'][f'room{room_id}']
            js_enc = js_enc.replace('return eval', 'return [strc, vdwdae325w_64we];')

            sign_fun, sign_v = ctx.eval(f'{js_enc};ub98484234();')

            tt = str(int(time.time()))
            did = hashlib.md5(tt.encode('utf-8')).hexdigest()
            rb = hashlib.md5(f"{room_id}{did}{tt}{sign_v}".encode('utf-8')).hexdigest()
            sign_fun = sign_fun.rstrip(';').replace("CryptoJS.MD5(cb).toString()", f'"{rb}"')
            sign_fun += f'("{room_id}","{did}","{tt}");'

            params = parse_qs(ctx.eval(sign_fun))
        except:
            logger.exception(f"{self.plugin_msg}: 获取签名参数异常")
            return False

        params['cdn'] = config.get('douyucdn', 'tct-h5')
        params['cdn'] = config.get('douyu_cdn', params['cdn'])
        params['rate'] = config.get('douyu_rate', 0)

        try:
            live_data = await self.get_play_info(room_id, params)
            self.raw_stream_url = f"{live_data['rtmp_url']}/{live_data['rtmp_live']}"
        except:
            logger.exception(f"{self.plugin_msg}: ")
            return False

        return True

    def danmaku_init(self):
        if self.douyu_danmaku:
            self.danmaku = DanmakuClient(self.url, self.gen_download_filename())

    async def get_play_info(self, room_id, params):
        __cdn_check = lambda _name, _list: any(_name in _item['cdn'] for _item in _list)

        live_data = (
            await client.post(f'https://www.douyu.com/lapi/live/getH5Play/{room_id}',
                                headers=self.fake_headers, params=params, timeout=5)
                    ).json().get('data')
        if isinstance(live_data, dict):
            if not live_data['rtmp_cdn'].endswith('h5') or 'akm' in live_data['rtmp_cdn']:
                params['cdn'] = 'tct-h5' if __cdn_check('tct-h5', live_data['cdnsWithName']) \
                                         else live_data['cdnsWithName'][-1]['cdn']
                return await self.get_play_info(room_id, params)
            return live_data
        return None
