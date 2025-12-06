import hashlib
import time
import httpx
import asyncio
from urllib.parse import parse_qs, urlencode, quote
from async_lru import alru_cache
from typing import Union, Any, Optional


from ..common.util import client
from ..Danmaku import DanmakuClient
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import logger, match1, random_user_agent, json_loads, test_jsengine


DOUYU_DEFAULT_DID = "10000000000000000000000000001501"
DOUYU_WEB_DOMAIN = "www.douyu.com"
DOUYU_PLAY_DOMAIN = "playweb.douyucdn.cn"
DOUYU_MOBILE_DOMAIN = "m.douyu.com"


@Plugin.download(regexp=r'https?://(?:(?:www|m)\.)?douyu\.com')
class Douyu(DownloadBase):
    def __init__(self, fname, url, config, suffix='flv'):
        super().__init__(fname, url, config, suffix)
        self.room_id: str = ""
        self.douyu_danmaku = config.get('douyu_danmaku', False)
        self.douyu_disable_interactive_game = config.get('douyu_disable_interactive_game', False)
        self.douyu_cdn = config.get('douyu_cdn', 'hw-h5')
        self.douyu_rate = config.get('douyu_rate', 0)
        # 新增：是否强制构建 hs-h5 链接（即使 play_info 返回的 rtmp_cdn 已经是 hs-h5）
        self.douyu_force_hs = config.get('douyu_force_hs', False)
        self.__js_runable = test_jsengine()


    async def acheck_stream(self, is_check=False):
        if len(self.url.split("douyu.com/")) < 2:
            logger.error(f"{self.plugin_msg}: 直播间地址错误")
            return False

        self.fake_headers['referer'] = f"https://{DOUYU_WEB_DOMAIN}"

        try:
            self.room_id = str(match1(self.url, r'rid=(\d+)'))
            if not self.room_id.isdigit():
                self.room_id = await get_real_rid(self.url)
        except:
            logger.exception(f"{self.plugin_msg}: 获取房间号错误")
            return False

        for _ in range(3): # 缓解 #1376 海外请求失败问题
            try:
                room_info = await client.get(
                    f"https://{DOUYU_WEB_DOMAIN}/betard/{self.room_id}",
                    headers=self.fake_headers
                )
                room_info.raise_for_status()
            except httpx.RequestError as e:
                logger.debug(f"{self.plugin_msg}: {e}", exc_info=True)
                continue
            except:
                logger.exception(f"{self.plugin_msg}: 获取直播间信息错误")
                return False
            else:
                break
        else:
            logger.error(f"{self.plugin_msg}: 获取直播间信息失败")
            return False
        room_info = json_loads(room_info.text)['room']

        if room_info['show_status'] != 1:
            logger.debug(f"{self.plugin_msg}: 未开播")
            return False
        if room_info['videoLoop'] != 0:
            logger.debug(f"{self.plugin_msg}: 正在放录播")
            return False
        if self.douyu_disable_interactive_game:
            gift_info = (
                await client.get(
                    f"https://{DOUYU_WEB_DOMAIN}/api/interactive/web/v2/list?rid={self.room_id}",
                    headers=self.fake_headers
            )).json().get('data', {})
            if gift_info:
                logger.debug(f"{self.plugin_msg}: 正在运行互动游戏")
                return False
        self.room_title = room_info['room_name']
        if room_info['isVip'] == 1:
            async with DouyuUtils._lock:
                DouyuUtils.VipRoom.add(self.room_id)

        if is_check:
            return True

        # 提到 self 以供 hack 功能使用
        self.__req_query = {
            'cdn': self.douyu_cdn,
            # 'cdn': "akm-h5",
            'rate': str(self.douyu_rate),
            'ver': '219032101',
            'iar': '0', # ispreload? 1: 忽略 rate 参数，使用默认画质
            'ive': '0', # rate? 0~19 时、19~24 时请求数 >=3 为真
            'rid': self.room_id,
            'hevc': '0',
            'fa': '0', # isaudio
            'sov': '0', # use wasm?
        }

        for _ in range(2): # 允许多重试一次以剔除 scdn
            # self.__js_runable = False
            try:
                play_info = await self.aget_web_play_info(self.room_id, self.__req_query)
                if play_info['rtmp_cdn'].startswith('scdn'):
                    new_cdn = play_info['cdnsWithName'][-1]['cdn']
                    logger.debug(f"{self.plugin_msg}: 回避 scdn 为 {new_cdn}")
                    self.__req_query['cdn'] = new_cdn
                    continue
            except (RuntimeError, ValueError) as e:
                logger.warning(f"{self.plugin_msg}: {e}")
            except httpx.RequestError as e:
                logger.debug(f"{self.plugin_msg}: {e}", exc_info=True)
            except Exception as e:
                logger.exception(f"{self.plugin_msg}: 未处理的错误 {e}，自动重试")
            else:
                break
        else:
            # Unknown Error
            logger.error(f"{self.plugin_msg}: 获取播放信息失败")
            return False

        self.raw_stream_url = f"{play_info['rtmp_url']}/{play_info['rtmp_live']}"

        # HACK: 构造 hs-h5 cdn 直播流链接
        # self.douyu_cdn = 'hs-h5'
        # 修改：当用户选择 hs-h5 时，允许通过配置强制构造 hs 链接（即使 play_info 已经返回 hs-h5）
        if self.douyu_cdn == 'hs-h5':
            need_build = self.douyu_force_hs or play_info['rtmp_cdn'] != 'hs-h5'
            if need_build:
                if not self.__js_runable:
                    logger.warning(f"{self.plugin_msg}: 未找到 jsengine，无法构建 hs-h5 链接")
                is_tct = play_info['rtmp_cdn'] == 'tct-h5'
                try:
                    fake_host, cname_url = await self.build_hs_url(self.raw_stream_url, is_tct)
                except:
                    logger.exception(f"{self.plugin_msg}: 构建 hs-h5 链接失败")
                else:
                    self.raw_stream_url = cname_url
                    self.stream_headers['Host'] = fake_host
            else:
                logger.debug(f"{self.plugin_msg}: play_info 返回的 rtmp_cdn 已是 hs-h5，且未开启 douyu_force_hs，跳过构建 hs-h5")
        return True


    def danmaku_init(self):
        if self.douyu_danmaku:
            content = {
                'room_id': self.room_id,
            }
            self.danmaku = DanmakuClient(self.url, self.gen_download_filename(), content)


    async def aget_sign(self, rid: Union[str, int]) -> dict[str, Any]:
        '''
        :param rid: 房间号
        :return: sign dict
        '''
        if not self.__js_runable:
            raise RuntimeError("jsengine not found")
        try:
            import jsengine
            ctx = jsengine.jsengine()
            js_enc = (
                await client.get(f'https://www.douyu.com/swf_api/homeH5Enc?rids={rid}',
                                 headers=self.fake_headers)
            ).json()['data'][f'room{rid}']
            js_enc = js_enc.replace('return eval', 'return [strc, vdwdae325w_64we];')

            sign_fun, sign_v = ctx.eval(f'{js_enc};ub98484234();') # type: ignore

            tt = str(int(time.time()))
            did = hashlib.md5(tt.encode('utf-8')).hexdigest()
            rb = hashlib.md5(f"{rid}{did}{tt}{sign_v}".encode('utf-8')).hexdigest()
            sign_fun = sign_fun.rstrip(';').replace("CryptoJS.MD5(cb).toString()", f'"{rb}"')
            sign_fun += f'("{rid}","{did}","{tt}");'

            params = parse_qs(ctx.eval(sign_fun))

        except Exception as e:
            logger.exception(f"{self.plugin_msg}: 获取签名参数异常")
            raise e
        return params


    async def aget_web_play_info(
        self,
        room_id: Union[str, int],
        req_query: dict[str, Any],
        req_method: str = "POST",
        did: str = DOUYU_DEFAULT_DID,
    ) -> dict[str, Any]:
        '''
        :param room_id: 房间号
        :param req_query: 请求参数
        :param req_method: 请求方法。可选 GET, POST（默认）
        :param did: douyuid
        :return: PlayInfo
        '''
        if type(room_id) == int:
            room_id = str(room_id)
        if not self.__js_runable:
            s = await DouyuUtils.sign(type="stream", ts=int(time.time()), did=did, rid=room_id)
            logger.debug(f"{self.plugin_msg}: 免 JSEngine 签名参数 {s}")
            auth_param = {
                "enc_data": s['key']['enc_data'],
                "tt": s['ts'],
                "did": did,
                "auth": s['auth'],
            }
            req_query.update(auth_param)
        else:
            s = await self.aget_sign(room_id)
            logger.debug(f"{self.plugin_msg}: JSEngine 签名参数 {s}")
            req_query.update(s)
        api_ver = "V1" if not self.__js_runable else ""
        is_vip = room_id in DouyuUtils.VipRoom # 非 vip room 需要 e 参数，部分直播间可直接请求 hs-h5
        req_method = "GET" if is_vip and not api_ver else "POST"
        path = f"/lapi/live/getH5Play{api_ver}/{room_id}"
        url = f"https://{DOUYU_PLAY_DOMAIN}{path}" if req_method == "GET" else f"https://{DOUYU_WEB_DOMAIN}{path}"
        # url += f"?{urlencode(req_query, doseq=True, encoding='utf-8')}"
        logger.debug(f"{self.plugin_msg}: 使用参数 {str(req_query)} 请求 {url}")
        if req_method == "GET":
            rsp = await client.get(
                url,
                headers=self.fake_headers,
                params=req_query
            )
        else:
            rsp = await client.post(
                url,
                headers={**self.fake_headers, 'user-agent': DouyuUtils.UserAgent},
                params=req_query, # V1 接口需使用查询参数
                data=req_query # 原接口需使用请求体
            )
        rsp.raise_for_status()
        play_data = json_loads(rsp.text)
        if not play_data:
            raise RuntimeError(f"获取播放信息失败 {rsp}")
        if play_data['error'] != 0 or not play_data.get('data', {}):
            raise ValueError(f"获取播放信息错误 {str(play_data)}")
        return play_data['data']


    async def aget_mobile_play_info(
        self,
        req_query: dict[str, Any]
    ) -> dict[str, Any]:
        if not self.__js_runable:
            raise RuntimeError("jsengine not found")
        url = f'https://{DOUYU_MOBILE_DOMAIN}/api/room/ratestream'
        # elif preview:
        #     c_time_str = str(time.time_ns())
        #     url = f'https://playweb.douyucdn.cn/lapi/live/hlsH5Preview/{room_id}?{c_time_str[:18]}'
        #     data = {
        #         'rid': self.__room_id,
        #         'did': data.get('did', ["10000000000000000000000000001501"])[0],
        #     }
        #     req_headers.update({
        #         'Rid': self.__room_id,
        #         'Time': c_time_str[:13],
        #         'Auth': hashlib.md5(f"{self.__room_id}{c_time_str[:13]}".encode('utf-8')).hexdigest(),
        #     })
        rsp = await client.post(
            url,
            headers={**self.fake_headers, 'user-agent': random_user_agent('mobile')},
            data=req_query
        )
        rsp.raise_for_status()
        play_data = json_loads(rsp.text)
        if play_data['code'] != 0:
            raise ValueError(f"获取播放信息错误 {str(play_data)}")
        return play_data['data']


    def parse_stream_info(self, url) -> tuple[str, str, dict]:
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
        base_url, params, *_ = url.split('?')
        query = {k: v[0] for k, v in parse_qs(params).items()}
        stream_id = match1(base_url, rf"\/({self.room_id}[^\._/]+)")
        rtmp_url = url.split(stream_id)[0]
        return get_tx_app_name(rtmp_url[:-1]), stream_id, query


    async def build_tx_url(self, tx_app_name, stream_id, query) -> str:
        '''
        构建腾讯CDN URL
        return: tx_url
        '''
        origin = query.get('origin', 'unknown')
        if origin not in ['tct', 'hw', 'dy']:
            '''
            dy: 斗鱼自建
            tct: 腾讯云
            hw: 华为云
            '''
            raise ValueError(f"当前流来源 {origin} 不支持切换为腾讯云推流")
        elif origin == 'dy':
            logger.warning(f"{self.plugin_msg}: 当前流来源 {origin} 可能不存在腾讯云流")
        tx_host = "tc-tct.douyucdn2.cn"
        tx_url = f"https://{tx_host}/{tx_app_name}/{stream_id}.flv?%s"
        m_play_info = await self.aget_mobile_play_info(self.__req_query)
        _, _, m_query = self.parse_stream_info(m_play_info['url'])
        # 需要移动端的宽松验证 token
        m_query.pop('vhost', None)
        query.update({
            'fcdn': 'tct',
            **m_query,
        })
        query = urlencode(query, doseq=True, encoding='utf-8')
        return tx_url % query


    async def build_hs_url(self, url: str, is_tct: bool = False) -> tuple[str, str]:
        '''
        构建火山CDN URL
        :param url: 腾讯云 URL
        :param is_tct: 是否为 tct 流
        return: fake_hs_host, hs_cname_url
        '''
        logger.debug(f"build_hs_url: build from {url}")
        tx_app_name, stream_id, query = self.parse_stream_info(url)
        # 必须从 tct 转 hs
        if not is_tct:
            url = await self.build_tx_url(tx_app_name, stream_id, query)
        tx_host = url.split('//')[1].split('/')[0]
        hs_host = f"{tx_app_name.replace('dyliveflv', 'huos')}.douyucdn2.cn"
        hs_host = hs_host.replace('huos1.', 'huosa.')
        encoded_url = quote(url, safe='')
        query.update({
            'fp_user_url': encoded_url,
            'vhost': tx_host,
            'domain': tx_host,
        })
        query = urlencode(query, doseq=True, encoding='utf-8')
        hs_cname_host = "douyu-pull.s.volcfcdndvs.com"
        hs_cname_url = f"http://{hs_cname_host}/live/{stream_id}.flv?{query}"
        return (hs_host, hs_cname_url)


class DouyuUtils:
    '''
    逆向实现 //shark2.douyucdn.cn/front-publish/live-master/js/player_first_preload_stream/player_first_preload_stream_6cd7aab.js
    '''
    WhiteEncryptKey: dict = dict()
    VipRoom: set = set()
    # enc_data 会校验 UA
    UserAgent: str = ""
    # 防止并发访问
    _lock: asyncio.Lock = asyncio.Lock()
    _update_key_event: Optional[asyncio.Event] = None

    @staticmethod
    def is_key_valid():
        return (
            bool(DouyuUtils.WhiteEncryptKey) # Key 存在
            and
            DouyuUtils.WhiteEncryptKey.get('expire_at', 0) > int(time.time()) # Key 过期
        )

    @staticmethod
    async def update_key(
        domain: str = DOUYU_WEB_DOMAIN,
        did: str = DOUYU_DEFAULT_DID
    ) -> bool:
        # single-flight
        async with DouyuUtils._lock:
            if DouyuUtils._update_key_event is not None:
                evt = DouyuUtils._update_key_event
                leader = False
            else:
                DouyuUtils._update_key_event = asyncio.Event()
                evt = DouyuUtils._update_key_event
                leader = True
        if not leader:
            await evt.wait()
            return DouyuUtils.is_key_valid()

        try:
            # 防风控
            async with DouyuUtils._lock:
                DouyuUtils.UserAgent = random_user_agent()

            rsp = await client.get(
                f"https://{domain}/wgapi/livenc/liveweb/websec/getEncryption",
                params={"did": did},
                headers={
                    "User-Agent": DouyuUtils.UserAgent
                },
            )
            rsp.raise_for_status()
            data = json_loads(rsp.text)
            if data['error'] != 0:
                raise RuntimeError(f'getEncryption error: code={data["error"]}, msg={data["msg"]}')
            data['data']['cpp']['expire_at'] = int(time.time()) + 86400

            async with DouyuUtils._lock:
                DouyuUtils.WhiteEncryptKey = data['data']
            return True
        except Exception:
            logger.exception(f"{DouyuUtils.__name__}: 获取加密密钥失败")
            return False
        finally:
            async with DouyuUtils._lock:
                if DouyuUtils._update_key_event is not None:
                    DouyuUtils._update_key_event.set()
                    DouyuUtils._update_key_event = None


    @staticmethod
    async def sign(
        type: str, # unused
        ts: int,
        did: str,
        rid: Union[str, int],
    ) -> dict[str, Any]:
        '''
        :param type: unused
        :param ts: 10位Unix时间戳
        :param did: douyuid
        :param rid: 房间号
        '''
        if not rid:
            raise ValueError("rid is None")

        # 确保密钥有效
        for _ in range(2): # 重试两次
            if not DouyuUtils.is_key_valid():
                if not (await DouyuUtils.update_key()):
                    continue
            break
        else:
            raise RuntimeError("获取加密密钥失败")

        if not type:
            type = "stream"
        if not ts:
            ts = int(time.time())
        if not did:
            did = DOUYU_DEFAULT_DID

        rand_str = DouyuUtils.WhiteEncryptKey['rand_str']
        enc_time = DouyuUtils.WhiteEncryptKey['enc_time']
        key = DouyuUtils.WhiteEncryptKey['key']
        is_special = DouyuUtils.WhiteEncryptKey['is_special']
        key_data = {k: v for k, v in DouyuUtils.WhiteEncryptKey.items() if k not in ["cpp"]}

        secret = rand_str
        salt = "" if is_special else f"{rid}{ts}"
        for _ in range(enc_time):
            secret = hashlib.md5(f"{secret}{key}".encode('utf-8')).hexdigest()
        auth = hashlib.md5(f"{secret}{key}{salt}".encode('utf-8')).hexdigest()

        return {
            'key': key_data,
            'alg_ver': "1.0",
            "key_ver": "",
            'auth': auth,
            'ts': ts,
        }


@alru_cache(maxsize=None)
async def get_real_rid(url: str) -> str:
    rid = url.split('douyu.com/')[1].split('/')[0].split('?')[0] or match1(url, r'douyu.com/(\d+)')
    resp = await client.get(f"https://{DOUYU_MOBILE_DOMAIN}/{rid}", headers={
        "User-Agent": random_user_agent('mobile')
    })
    real_rid = match1(resp.text, r'roomInfo":{"rid":(\d+)')
    return str(real_rid)
