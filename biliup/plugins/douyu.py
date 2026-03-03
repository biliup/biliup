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
            print(f"isVip: {self.room_id}")
            async with DouyuUtils._lock:
                DouyuUtils.VipRoom.add(self.room_id)

        if is_check:
            return True

        # 提到 self 以供 hack 功能使用
        self.__req_query = {
            'cdn': self.douyu_cdn,
            'rate': str(self.douyu_rate),
            'ver': 'Douyu_new',
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

        return True


    def danmaku_init(self):
        if self.douyu_danmaku:
            content = {
                'room_id': self.room_id,
            }
            self.danmaku = DanmakuClient(self.url, self.gen_download_filename(), content)


    async def aclac_sign(self, rid: Union[str, int]) -> dict[str, Any]:
        '''
        :param rid: 房间号
        :return: sign dict
        '''
        if not self.__js_runable:
            raise RuntimeError("jsengine not found")
        try:
            import jsengine
            ctx = jsengine.jsengine()
            h5enc_url = f"https://{DOUYU_WEB_DOMAIN}/swf_api/homeH5Enc?rids={rid}"
            js_enc = (
                await client.get(h5enc_url, headers=self.fake_headers)
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
        did: str = DOUYU_DEFAULT_DID,
    ) -> dict[str, Any]:
        '''
        :param room_id: 房间号
        :param req_query: 请求参数
        :param did: douyuid
        :return: PlayInfo
        '''
        if type(room_id) == int:
            room_id = str(room_id)

        if self.__js_runable and room_id in DouyuUtils.VipRoom:
            s = await self.aclac_sign(room_id)
            logger.debug(f"{self.plugin_msg}: JSEngine 签名参数 {s}")
            req_query = {
                **req_query,
                **s
            }
            url = f"https://{DOUYU_PLAY_DOMAIN}/lapi/live/getH5Play/{room_id}"
            rsp = await client.get(
                url,
                headers=self.fake_headers,
                params=req_query
            )
        else:
            s = await DouyuUtils.sign(sign_type="stream", ts=int(time.time()), did=did, rid=room_id)
            logger.debug(f"{self.plugin_msg}: 免 JSEngine 签名参数 {s}")
            auth_param = {
                "enc_data": s['key']['enc_data'],
                "tt": s['ts'],
                "did": did,
                "auth": s['auth'],
            }
            req_query = {
                **req_query,
                **auth_param,
            }
            url = f"https://{DOUYU_WEB_DOMAIN}/lapi/live/getH5PlayV1/{room_id}"
            rsp = await client.post(
                url,
                headers={**self.fake_headers, 'user-agent': DouyuUtils.UserAgent},
                # params=req_query, # V1 接口需使用查询参数
                data=req_query # 原接口需使用请求体
            )

        rsp.raise_for_status()
        play_data = json_loads(rsp.text)
        if not play_data:
            raise RuntimeError(f"获取播放信息失败 {rsp}")
        if (err := play_data['error']) != 0 or not play_data.get('data', {}):
            msg = play_data.get('msg', '')
            if err == -5:
                raise RuntimeError("[closeRoom] 主播未开播")
            elif err == -9:
                raise RuntimeError("[room_bus_checksevertime] 用户本机时间戳不对")
            elif err == 126:
                raise RuntimeError(f"版权原因，该地域不允许播放：{msg}")
            else:
                raise RuntimeError(f"获取播放信息错误: code={err}, msg={msg}, raw_obj={play_data}")
        return play_data['data']


class DouyuUtils:
    '''
    逆向实现 //shark2.douyucdn.cn/front-publish/live-master/js/player_first_preload_stream/player_first_preload_stream_6cd7aab.js
    更新    //shark2.douyucdn.cn/front-publish/douyu-web-first-stream-master/web-encrypt-57bbddd0.js
    '''
    WhiteEncryptKey: dict = dict()
    VipRoom: set = set()
    # enc_data 会校验 UA
    UserAgent: str = ""
    # 防止并发访问
    _lock: asyncio.Lock = asyncio.Lock()
    _update_key_event: Optional[asyncio.Event] = None

    @staticmethod
    def is_key_valid(sign_type: str = "stream") -> bool:
        if not DouyuUtils.WhiteEncryptKey:
            return False
        if sign_type == "stream":
            expire_at = DouyuUtils.WhiteEncryptKey.get('expire_at', 0)
        else:
            expire_at = DouyuUtils.WhiteEncryptKey.get('cpp', {}).get('expire_at', 0)
        return expire_at > int(time.time())

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
            DouyuUtils.UserAgent = random_user_agent() # 防风控，每次更新随机 UA
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
                DouyuUtils._update_key_event.set()
                DouyuUtils._update_key_event = None


    @staticmethod
    async def sign(
        sign_type: str,
        ts: int,
        did: str,
        rid: Union[str, int],
    ) -> dict[str, Any]:
        '''
        :param sign_type: 签名类型，可选 stream / login / heartbeat
        :param ts: 10位Unix时间戳
        :param did: douyuid
        :param rid: 房间号
        '''
        if not rid:
            raise ValueError("rid is None")
        if not sign_type:
            sign_type = "stream"
        if not ts:
            ts = int(time.time())
        if not did:
            did = DOUYU_DEFAULT_DID

        # 确保密钥有效
        for _ in range(2):
            if DouyuUtils.is_key_valid(sign_type) or await DouyuUtils.update_key():
                break
        else:
            raise RuntimeError("获取加密密钥失败")

        rand_str = DouyuUtils.WhiteEncryptKey['rand_str']
        enc_time = DouyuUtils.WhiteEncryptKey['enc_time']
        key_data = {k: v for k, v in DouyuUtils.WhiteEncryptKey.items() if k != "cpp"}

        _CPP_SECTION = {"login": "danmu", "heartbeat": "heartbeat"}

        if sign_type == "stream":
            salt = "" if DouyuUtils.WhiteEncryptKey['is_special'] == 1 else f"{rid}{ts}"
            key = DouyuUtils.WhiteEncryptKey['key']
            key_ver = ""
        elif cpp_section := _CPP_SECTION.get(sign_type):
            cpp = DouyuUtils.WhiteEncryptKey['cpp'][cpp_section]
            salt = f"{rid}{did}{ts}"
            key, key_ver = cpp['key'], cpp['key_ver']
        else:
            raise ValueError(f"wrong sign type: {sign_type}")

        secret = rand_str
        for _ in range(enc_time):
            secret = hashlib.md5(f"{secret}{key}".encode('utf-8')).hexdigest()
        auth = hashlib.md5(f"{secret}{key}{salt}".encode('utf-8')).hexdigest()

        return {
            'key': key_data,
            'alg_ver': "1.0",
            'key_ver': key_ver,
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
