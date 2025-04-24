import hashlib
import time
import random
from urllib.parse import parse_qs, urlencode, quote
from functools import lru_cache

from biliup.common.util import client
from biliup.config import config
from biliup.Danmaku import DanmakuClient
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import logger, match1, random_user_agent, json_loads


@Plugin.download(regexp=r'https?://(?:(?:www|m)\.)?douyu\.com')
class Douyu(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.__room_id = match1(url, r'rid=(\d+)')
        self.douyu_danmaku = config.get('douyu_danmaku', False)
        self.douyu_disable_interactive_game = config.get('douyu_disable_interactive_game', False)
        self.douyu_cdn = config.get('douyu_cdn', 'hw-h5')
        self.douyu_rate = config.get('douyu_rate', 0)

    async def acheck_stream(self, is_check=False):
        if len(self.url.split("douyu.com/")) < 2:
            logger.error(f"{self.plugin_msg}: 直播间地址错误")
            return False

        self.fake_headers['referer'] = f"https://www.douyu.com"
        self.fake_headers['origin'] = f"https://www.douyu.com"

        try:
            if not self.__room_id:
                self.__room_id = _get_real_rid(self.url)
        except:
            logger.exception(f"{self.plugin_msg}: 获取房间号错误")
            return False

        try:
            room_info = await client.get(f"https://www.douyu.com/betard/{self.__room_id}", headers=self.fake_headers)
            room_info.raise_for_status()
            room_info = json_loads(room_info.text)['room']
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

        params.update({
            'cdn': [self.douyu_cdn],
            'rate': [str(self.douyu_rate)],
            'iar': ['1'],
            'ive': ['0'],
            'rid': self.__room_id,
            'hevc': ['0'],
            'fa': ['0'],
            'sov': ['0'],
            'ver': ['219032101'],
        })

        self._req_data = parse_qs(urlencode(params, doseq=True, encoding='utf-8'))

        try:
            live_data = await self.get_play_info(self.__room_id, self._req_data)
            url = f"{live_data['rtmp_url']}/{live_data['rtmp_live']}"
        except:
            logger.exception(f"{self.plugin_msg}: ")
            return False

        host = None
        is_tct = (live_data['rtmp_url'].find('tc-tct.douyucdn2.cn') != -1)
        # HACK: 构建斗鱼直播流链接
        try:
            if self.douyu_cdn == 'hs-h5':
                host, _, url = await self.build_hs_url(url)
            elif self.douyu_cdn == 'tct-h5' and not is_tct:
                _, _, url = await self.build_tx_url(url)
        except (RuntimeError, ValueError) as e:
            logger.error(f"{self.plugin_msg}: {e}")

        if url:
            self.raw_stream_url = url
            if host:
                self.fake_headers['host'] = host

        return True

    def danmaku_init(self):
        if self.douyu_danmaku:
            content = {
                'room_id': self.__room_id,
            }
            self.danmaku = DanmakuClient(self.url, self.gen_download_filename(), content)

    async def get_play_info(self, room_id, data, mobile=False, preview=False):
        req_headers = self.fake_headers.copy()
        if mobile:
            url = 'https://m.douyu.com/api/room/ratestream'
        elif preview:
            c_time_str = str(time.time_ns())
            url = f'https://playweb.douyucdn.cn/lapi/live/hlsH5Preview/{room_id}?{c_time_str[:18]}'
            data = {
                'rid': self.__room_id,
                'did': data.get('did', ["10000000000000000000000000001501"])[0],
            }
            req_headers.update({
                'Rid': self.__room_id,
                'Time': c_time_str[:13],
                'Auth': hashlib.md5(f"{self.__room_id}{c_time_str[:13]}".encode('utf-8')).hexdigest(),
            })
        else:
            url = f'https://www.douyu.com/lapi/live/getH5Play/{room_id}'
        live_data = await client.post(url, headers=req_headers, data=data)
        if not live_data.is_success:
            raise RuntimeError(live_data.text)
        live_data = json_loads(live_data.text).get('data')
        if mobile or preview:
            return live_data
        if isinstance(live_data, dict):
            if live_data.get('rtmp_cdn', '').startswith('scdn'):
                import copy
                __data = copy.deepcopy(data)
                __data['cdn'] = live_data['cdnsWithName'][-1]['cdn']
                return await self.get_play_info(room_id, __data)
            return live_data
        raise RuntimeError(live_data)

    def get_p2p_play_info(self, xp2p_unity_api_domain):
        url = f"https://{xp2p_unity_api_domain}"
        pass


    def parse_stream_info(self, url) -> tuple[str, str, str]:
        '''
        解析推流信息
        '''
        def get_tx_app_name(rtmp_url) -> str:
            '''
            获取腾讯云推流应用名
            '''
            host = rtmp_url.split('//')[1].split('/')[0]
            app_name = rtmp_url.split('/')[-1]
            # group 按顺序排序
            i = match1(host, r'.+(sa|3a|1a|3|1)')
            if i:
                if i == "sa":
                    i = "1"
                return f"dyliveflv{i}"
            return app_name
        list = url.split('?')
        query = parse_qs(list[1])
        origin = query.get('origin', ['0'])[0]
        if origin not in ['tct', 'hw', 'dy']:
            '''
            dy: 斗鱼自建
            tct: 腾讯云
            hw: 华为云
            '''
            raise ValueError(f"当前流来源 {origin} 不支持切换为腾讯云推流")
        elif origin == 'dy':
            logger.warning(f"{self.plugin_msg}: 当前流来源 {origin} 可能不存在腾讯云流")
        query_str = urlencode(query, doseq=True, encoding='utf-8')
        stream_id = list[0].split('/')[-1].split('.')[0]
        rtmp_url = list[0].split(stream_id)[0]
        return get_tx_app_name(rtmp_url[:-1]), stream_id.split('_')[0], query_str

    async def build_tx_url(self, url, callback=None) -> tuple[str, str, str]:
        '''
        构建腾讯CDN URL
        return: tx_host, stream_id, tx_url
        '''
        tx_app_name, stream_id, query = self.parse_stream_info(url)
        tx_host = "tc-tct.douyucdn2.cn"
        tx_url = f"https://{tx_host}/{tx_app_name}/{stream_id}.flv?%s"
        query = parse_qs(query)
        m_data = await self.get_play_info(self.__room_id, self._req_data, mobile=True)
        _, _, m_query = self.parse_stream_info(m_data['url'])
        m_query = parse_qs(m_query)
        m_query.pop('vhost', None)
        query.update({
            'fcdn': ['tct'],
            **m_query,
        })
        query = urlencode(query, doseq=True, encoding='utf-8')
        return tx_host, stream_id, tx_url % query

    async def build_hs_url(self, url, *, tx_host=None, stream_id=None) -> tuple[str, str, str]:
        '''
        构建火山CDN URL
        return: hs_host, stream_id, hs_ip_url
        '''
        if not tx_host:
            tx_host, stream_id, tx_url = await self.build_tx_url(url)
        else:
            tx_url = url
        query = parse_qs(tx_url.split('?')[-1])
        encoded_url = quote(tx_url, safe='')
        query.update(
            {
                'fp_user_url': [encoded_url],
                'vhost': [tx_host],
                'domain': [tx_host],
            }
        )
        query = urlencode(query, doseq=True, encoding='utf-8')
        hs_host = "douyu-pull.s.volcfcdndvs.com"
        # hs_host = "huos3.douyucdn2.cn"
        hs_ip_list = [
            "98.98.121.44", # singapore
            "128.14.110.71", # singapore
            "128.14.110.70", # singapore
            "104.166.175.26", # russia
            "104.166.175.2", # russia
            "45.43.34.227", # hongkong
            "192.169.99.102", # hongkong
            "156.59.27.66", # japan
            "156.59.27.65", # japan
            "23.248.183.107", # south-africa
            "23.248.183.106", # south-africa
        ]
        hs_ip = random.choice(hs_ip_list)
        logger.info(f"{self.plugin_msg}: 使用 {hs_ip} 作为火山CDN节点")
        hs_url = f"http://{hs_host}/live/{stream_id}.flv?{query}"
        hs_ip_url = hs_url.replace(hs_host, hs_ip)
        return hs_host, stream_id, hs_ip_url


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