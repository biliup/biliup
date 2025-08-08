import base64
import hashlib
import random
import time
import html
from enum import Enum
from dataclasses import dataclass
from urllib.parse import parse_qs, unquote
from async_lru import alru_cache
from typing import (
    Any,
    Dict,
    List,
    Union,
)

from ..common.util import client
from ..config import config
from ..Danmaku import DanmakuClient
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from . import logger, match1, json_loads

from biliup.plugins.huya_wup import Wup, DEFAULT_TICKET_NUMBER
from biliup.plugins.huya_wup.packet import (
    HuyaGetCdnTokenReq,
    HuyaGetCdnTokenRsp
)

HUYA_WEB_BASE_URL = "https://www.huya.com"
HUYA_MOBILE_BASE_URL = "https://m.huya.com"
HUYA_MP_BASE_URL = "https://mp.huya.com"
HUYA_WUP_BASE_URL = "https://wup.huya.com"
HUYA_WEB_ROOM_DATA_REGEX = r"var TT_ROOM_DATA = (.*?);"

@Plugin.download(regexp=r'https?://(?:(?:www|m)\.)?huya\.com')
class Huya(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.__room_id = url.split('huya.com/')[1].split('?')[0]
        self.huya_danmaku = config.get('huya_danmaku', False)
        self.huya_max_ratio = config.get('huya_max_ratio', 0)
        self.huya_cdn = config.get('huya_cdn', "").upper()  # 不填写时使用主播的CDN优先级
        self.huya_protocol = 'Hls' if config.get('huya_protocol') == 'Hls' else 'Flv'
        self.huya_imgplus = config.get('huya_imgplus', True)
        self.huya_cdn_fallback = config.get('huya_cdn_fallback', False)
        self.huya_mobile_api = config.get('huya_mobile_api', False)
        self.huya_codec = config.get('huya_codec', '264')
        self.huya_use_wup = config.get('huya_use_wup', True)

    async def acheck_stream(self, is_check=False):

        try:
            if not self.__room_id.isdigit():
                client.headers.update(self.fake_headers)
                self.__room_id = await _get_real_rid(self.url)
                logger.debug(f"{self.plugin_msg}: {_get_real_rid.cache_info()}")
            self.fake_headers['referer'] = self.url
            room_profile = await self.get_room_profile(self.huya_mobile_api)
        except Exception as e:
            logger.error(f"{self.plugin_msg}: {e}", exc_info=True)
            return False

        if not room_profile['live']:
            logger.debug(f"{self.plugin_msg}: {room_profile['message']}")
            self.raw_stream_url = None
            return False

        # 兼容 biliup/biliup#1200
        self.room_title = room_profile['room_title']
        # 虎牙回放
        if self.room_title.startswith('【回放】'):
            logger.debug(f"{self.plugin_msg}: {self.room_title}")
            return False

        if is_check:
            return True

        # self.room_title = room_profile['room_title']

        # is_xingxiu = (room_profile['gid'] == 1663)
        gid_blacklist = [1663, ]
        skip_query_build = room_profile['gid'] in gid_blacklist
        stream_urls = await self.build_stream_urls(room_profile['streams_info'], skip_query_build)
        cdn_list = list(stream_urls.keys())
        if not self.huya_cdn or self.huya_cdn not in cdn_list:
            self.huya_cdn = cdn_list[0]

        try:
            self.raw_stream_url = self.add_ratio(
                stream_urls[self.huya_cdn],
                room_profile['bitrate_info'],
                room_profile['max_bitrate']
            )
        except KeyError as e:
            logger.error(f"{self.plugin_msg}: {e}", exc_info=True)
            return False

        # HTTPS的直播流只允许连接一次
        if self.huya_cdn_fallback:
            _url = await self.acheck_url_healthy(self.raw_stream_url)
            if _url is None:
                logger.info(f"{self.plugin_msg}: cdn_fallback 顺序尝试 {cdn_list}")
                for cdn in cdn_list:
                    if cdn == self.huya_cdn:
                        continue
                    logger.info(f"{self.plugin_msg}: cdn_fallback-{cdn}")
                    if (await self.acheck_url_healthy(stream_urls[cdn])) is None:
                        continue
                    self.huya_cdn = cdn
                    logger.info(f"{self.plugin_msg}: cdn_fallback 回退到 {self.huya_cdn}")
                    break
                else:
                    logger.error(f"{self.plugin_msg}: cdn_fallback 所有链接无法使用")
                    return False
            room_profile = await self.get_room_profile(self.huya_mobile_api)
            if not room_profile['live']:
                logger.debug(f"{self.plugin_msg}: {room_profile['message']}")
                return False
            stream_urls = await self.build_stream_urls(room_profile['streams_info'], skip_query_build)

        self.raw_stream_url = self.add_ratio(
            stream_urls[self.huya_cdn],
            room_profile['bitrate_info'],
            room_profile['max_bitrate']
        )

        return True

    def danmaku_init(self):
        if self.huya_danmaku:
            self.danmaku = DanmakuClient(self.url, self.gen_download_filename())

    def add_ratio(self, url: str, bitrate_info: Dict[str, Any], max_bitrate: int) -> str:
        '''
        添加码率
        :param url: 流地址
        :param bitrate_info: 可选择的码率信息
        :param max_bitrate: 最大码率(不含hdr)
        :return: 添加码率后的流地址
        '''
        if self.huya_max_ratio and "&ratio" not in url:
            def __get_ratio(info: Dict[str, Any]) -> int:
                return info.get('iBitRate', 0) or max_bitrate
            try:
                selected_ratio = 0
                self.huya_max_ratio = int(self.huya_max_ratio)
                # 符合条件的码率
                allowed_ratio_list = [
                    __get_ratio(x) \
                    for x in bitrate_info \
                    if __get_ratio(x) <= self.huya_max_ratio
                ]
                # 录制码率
                if allowed_ratio_list:
                    selected_ratio = max(allowed_ratio_list)
                if selected_ratio:
                    return f"{url}&ratio={selected_ratio}"
            except KeyError as e:
                raise KeyError(f"确定码率时发生错误") from e
        return url

    def get_stream_name(self, stream_name: str) -> str:
        if self.huya_imgplus:
            return stream_name
        return stream_name.replace('-imgplus', '')

    async def build_stream_urls(
            self,
            streams_info: List[Dict[str, Any]],
            skip_query_build: bool
        ) -> List[Dict[str, Any]]:
        '''
        构建流地址
        :param streams_info: 流信息
        :param skip_query_build: 跳过构建anti_code
        :return: 流地址
        '''
        proto = self.huya_protocol
        streams = {}
        weights = {} # https://cdnweb.huya.com/getUidsDomainList?anchor_uid={anchor_uid}
        for stream in streams_info:
            # 优先级<0代表不可用
            priority = stream['iWebPriorityRate']
            if priority < 0:
                continue
            stream_name = self.get_stream_name(stream['sStreamName'])
            cdn = stream['sCdnType']
            suffix = stream[f's{proto}UrlSuffix']
            # 默认不修改 anticode
            anti_code = stream[f's{proto}AntiCode']
            if (
                # 禁用 imgplus
                not self.huya_imgplus
                or
                # 禁用 wup ，流信息不来自移动端，不在分区黑名单中
                not (self.huya_use_wup or self.huya_mobile_api or skip_query_build)
            ):
                logger.debug(f"{self.plugin_msg}: 构建 anticode")
                anti_code = self.build_query(stream_name, anti_code, self.get_uid(stream['lPresenterUid']))
            # 启用 imgplus、wup 且非 mobile api
            elif self.huya_use_wup and not self.huya_mobile_api:
                # 使用 Wup 获取的 anti_code，必须使用 Wup UA 进行连接
                anti_code = await self.get_true_anticode(cdn, stream_name, self.get_uid(stream['lPresenterUid']), proto)
            anti_code = f"{anti_code}&codec={self.huya_codec}"
            base_url = stream[f's{proto}Url'].replace('http://', 'https://') # 强制https
            streams[cdn] = f"{base_url}/{stream_name}.{suffix}?{anti_code}"
            weights[cdn] = priority
        return self.__weight_sorting(streams, weights)

    def extract_room_profile(self, data: Union[str, Dict[str, Any]]) -> Dict[str, Any]:
        '''
        ON: 直播
        REPLAY: 重播
        OFF: 未开播
        '''
        # PC web
        if isinstance(data, str):
            room_data = json_loads(match1(data, HUYA_WEB_ROOM_DATA_REGEX))
            s = data.split('stream: ')[1].split('};')[0]
            s_json = json_loads(s)
            bitrate_info = s_json.get('vMultiStreamInfo')
            if room_data['state'] != 'ON' or not bitrate_info:
                return {
                    'live': False,
                    'message': '未开播' if room_data['state'] != 'ON' else '未推流',
                }
            live_info = s_json['data'][0]['gameLiveInfo']
            streams_info = s_json['data'][0]['gameStreamInfoList']
        # Mobile API（微信小程序？）
        elif isinstance(data, dict):
            data = data['data']
            if data['liveStatus'] != 'ON' or not data.get('liveData', {}).get('bitRateInfo'):
                return {
                    'live': False,
                    'message': '未开播' if data['liveStatus'] != 'ON' else '未推流',
                }
            live_info = data['liveData']
            bitrate_info = json_loads(live_info['bitRateInfo'])
            streams_info = data['stream']['baseSteamInfoList']
        return {
            'artist': live_info['nick'],
            'artist_img': live_info['avatar180'].replace('http://', 'https://'),
            'bitrate_info': bitrate_info,
            'gid': live_info['gid'],
            'live': True,
            'live_start_time': live_info['startTime'],
            'max_bitrate': live_info['bitRate'],
            'room_cover': live_info['screenshot'].replace('http://', 'https://'),
            'room_title': live_info['introduction'],
            'streams_info': streams_info,
        }

    async def get_room_profile(self, use_api=False) -> dict:
        '''
        获取房间信息
        :param use_api: 是否使用API
        :return: 房间信息
        '''
        if use_api:
            params = {
                'm': 'Live',
                'do': 'profileRoom',
                'roomid': self.__room_id,
                'showSecret': 1,
            }
            resp = await client.get(
                f"{HUYA_MP_BASE_URL}/cache.php",
                headers=self.fake_headers, params=params)
            resp.raise_for_status()
            resp = json_loads(html.unescape(resp.text))
            if resp['status'] != 200:
                raise Exception(f"{resp['message']}")
        else:
            resp = await client.get(
                f"{HUYA_WEB_BASE_URL}/{self.__room_id}",
                headers=self.fake_headers)
            resp.raise_for_status()
            resp = html.unescape(resp.text)
            _raise_for_room_block(resp)
        return self.extract_room_profile(resp)

    async def get_true_anticode(
        self,
        cdn: str,
        stream_name: str,
        presenter_uid: int,
        proto: str,
    ) -> str:
        '''
        获取 wup anti_code
        :param cdn: cdn类型
        :param stream_name: 流名称
        :param presenter_uid: 主播uid
        :param proto: 协议类型
        :return: wup anti_code
        '''
        proto = "hls" if proto == "Hls" else "flv"
        headers = {}
        self.update_headers(headers)
        wup_req = Wup()
        wup_req.requestid = abs(DEFAULT_TICKET_NUMBER)
        wup_req.servant = "liveui"
        wup_req.func = "getCdnTokenInfo"
        token_info_req = HuyaGetCdnTokenReq()
        token_info_req.cdnType = cdn
        token_info_req.streamName = stream_name
        token_info_req.presenterUid = presenter_uid
        wup_req.put(HuyaGetCdnTokenReq, "tReq", token_info_req)
        data = wup_req.encode_v3()
        rsp = await client.post(HUYA_WUP_BASE_URL, data=data, headers=headers)
        wup_rsp = Wup()
        wup_rsp.decode_v3(rsp.content)
        token_info_rsp = wup_rsp.get(HuyaGetCdnTokenRsp,"tRsp")
        # print(token_info_rsp.as_dict())
        token_info = token_info_rsp.as_dict()
        logger.debug(f"{self.plugin_msg}: wup token_info {token_info}")
        return token_info[f'{proto}AntiCode']

    def build_query(self, stream_name, anti_code, uid: int) -> str:
        '''
        构建anti_code
        :param stream_name: 流名称
        :param anti_code: 原始anti_code
        :param uid: 主播uid
        :return: 构建后的anti_code
        '''
        url_query = parse_qs(anti_code)
        platform_id = url_query.get('t', [100])[0]
        ws_time = url_query['wsTime'][0]
        convert_uid = (uid << 8 | uid >> (32 - 8)) & 0xFFFFFFFF
        seq_id = uid + int(time.time() * 1000)
        ctype = url_query['ctype'][0]
        fm = unquote(url_query['fm'][0])
        ct = int((int(ws_time, 16) + random.random()) * 1000)
        ws_secret_prefix = base64.b64decode(fm.encode()).decode().split('_')[0]
        ws_secret_hash = hashlib.md5(f"{seq_id}|{ctype}|{platform_id}".encode()).hexdigest()
        secret_str = f'{ws_secret_prefix}_{convert_uid}_{stream_name}_{ws_secret_hash}_{ws_time}'
        ws_secret = hashlib.md5(secret_str.encode()).hexdigest()

        # &codec=av1
        # &codec=264
        # &codec=265
        # dMod: wcs-25 / mesh-0 DecodeMod-SupportMod
        # chrome > 104 or safari = mseh, chrome = mses
        # sdkPcdn: 1_1 第一个1连接次数 第二个1是因为什么连接
        # t: 平台信息 100 web(ctype=huya_live/huya_webh5) 102 小程序(ctype=tars_mp)
        # PLATFORM_TYPE = {'adr': 2, 'huya_liveshareh5': 104, 'ios': 3, 'mini_app': 102, 'wap': 103, 'web': 100}
        # sv: 2401090219 版本
        # sdk_sid:  _sessionId sdkInRoomTs 当前毫秒时间
        # return f"wsSecret={ws_secret}&wsTime={ws_time}&seqid={seq_id}&ctype={url_query['ctype'][0]}&ver=1&fs={url_query['fs'][0]}&u={convert_uid}&t={platform_id}&sv=2401090219&sdk_sid={int(time.time() * 1000)}&codec=264"
        anti_code = {
            "wsSecret": ws_secret,
            "wsTime": ws_time,
            "seqid": str(seq_id),
            "ctype": ctype,
            "ver": "1",
            "fs": url_query['fs'][0],
            "t": platform_id,
            "u": convert_uid,
            "uuid": str(int((ct % 1e10 + random.random()) * 1e3 % 0xffffffff)),
            "sdk_sid": str(int(time.time() * 1000)),
            # "codec": self.huya_codec,
        }
        return '&'.join([f"{k}={v}" for k, v in anti_code.items()])

    @staticmethod
    def __weight_sorting(
        data: Dict[str, Any], weights: Dict[str, Any]
    ) -> Dict[str, Any]:
        if data:
            data = {k: v for k, v in data.items() if k not in ['HY', 'HUYA', 'HYZJ']}
            return dict(sorted(data.items(), key=lambda x: weights[x[0]], reverse=True))
        return {}

    @staticmethod
    def get_uid(uid: Union[str, int, None] = None) -> int:
        try:
            if isinstance(uid, str):
                uid = int(uid)
        except ValueError:
            pass
        return uid or random.randint(1400000000000, 1499999999999)

    async def get_anonymous_uid(self) -> int:
        try:
            rsp = await client.post(
                url='https://udblgn.huya.com/web/anonymousLogin',
                headers=self.fake_headers,
                json={
                    "appId": 5002,
                    "byPass": 3,
                    "context": "",
                    "version": "2.4",
                    "data": {},
                }
            )
            rsp = json_loads(rsp.text)
        except:
            rsp = {}
        return rsp.get('data', {}).get('uid', self.get_uid())

    def update_headers(self, headers: dict):
        if self.huya_use_wup:
            user_agent = UAGenerator.build_user_agent(UAType.HYSDK, Platform.WINDOWS)
            # user_agent = f"{Huya.get_hysdk_ua()}_APP({Huya.get_hyapp_ua()})_SDK({Huya.get_hy_trans_mod_ua()})"
            headers['user-agent'] = user_agent
            headers['origin'] = HUYA_WEB_BASE_URL

class UAType(Enum):
    MEDIA_PLAYER = 'media_player'
    HYSDK = 'hysdk'

class Platform(Enum):
    ANDROID = 'adr'
    HUYA_NFTV = 'huya_nftv'
    WEBSOCKET = 'webh5'
    WINDOWS = 'pc_exe'

class UAGenerator:
    # 配置字典
    HYAPP_CONFIGS = {
        Platform.ANDROID: {
            'platform': Platform.ANDROID,
            'version': '0.0.0',  # LocalVersion or "0.0.0" + hotfix_version
            'channel': 'live'
        },
        Platform.HUYA_NFTV: {
            'platform': Platform.HUYA_NFTV,
            'version': '2.5.1.3141',
            'channel': 'official'
        },
        Platform.WINDOWS: {
            'platform': Platform.WINDOWS,
            'version': '6100301',
            'channel': 'official'
        },
        Platform.WEBSOCKET: { # UnUsed
            'platform': Platform.WEBSOCKET,
            'version': '2505091506',
            'channel': 'websocket'
        }
    }

    HYSDK_CONFIGS = {
        Platform.ANDROID: {
            'platform': 'Android',
            'version': '30000002'
        },
        Platform.WINDOWS: {
            'platform': 'Windows',
            'version': '30000002'
        }
    }

    TRANS_MOD_CONFIGS = {
        Platform.HUYA_NFTV: {
            'name': 'trans',
            'version': '1.24.99-rel-tv'
        },
        Platform.ANDROID: {
            'name': 'trans',
            'version': '2.22.13-rel'
        },
        Platform.WINDOWS: {
            'name': 'trans',
            'version': '2.24.0.5157'
        }
    }

    @staticmethod
    def get_hyapp_ua(platform: Platform = Platform.WINDOWS) -> str:
        '''
        生成 hyapp 用户代理字符串
        :param platform: 平台类型
        :return: 用户代理字符串
        '''
        config = UAGenerator.HYAPP_CONFIGS.get(platform)
        if not config:
            raise ValueError(f"不支持的平台: {platform}")

        hyapp_platform = config['platform']
        hyapp_version = config['version']
        hyapp_channel = config['channel']

        ua = f"{hyapp_platform}&{hyapp_version}&{hyapp_channel}"
        # windows 和 websocket 不需要添加 android_api_level
        if platform not in {Platform.WINDOWS, Platform.WEBSOCKET}:
            android_api_level = random.randint(28, 35)
            ua = f"{ua}&{android_api_level}"

        return ua

    @staticmethod
    def get_hysdk_ua(platform: Platform = Platform.WINDOWS) -> str:
        '''
        生成 hysdk 用户代理字符串
        :param platform: 平台类型 (Android 或 Windows)
        :return: 用户代理字符串
        '''
        config = UAGenerator.HYSDK_CONFIGS.get(platform)
        if not config:
            raise ValueError(f"HYSDK 不支持的平台: {platform}")

        hysdk_platform = config['platform']
        hysdk_version = config['version']

        return f"HYSDK({hysdk_platform}, {hysdk_version})"

    @staticmethod
    def get_hy_media_player_ua(platform: Platform = Platform.WINDOWS) -> str:
        '''
        生成 hy_media_player 用户代理字符串
        :param platform: 平台类型
        :return: 用户代理字符串
        '''
        # 目前只支持 android 平台
        hy_mp_platform = 'android'
        hy_mp_version = '20000313'

        return f"{hy_mp_platform}, {hy_mp_version}"

    @staticmethod
    def get_hy_trans_mod_ua(platform: Platform = Platform.WINDOWS) -> str:
        '''
        生成 hy_trans_mod 用户代理字符串
        :param platform: 平台类型
        :return: 用户代理字符串
        '''
        config = UAGenerator.TRANS_MOD_CONFIGS.get(platform)
        if not config:
            raise ValueError(f"Trans mod 不支持的平台: {platform}")

        hy_trans_mod_name = config['name']
        hy_trans_mod_version = config['version']

        return f"{hy_trans_mod_name}&{hy_trans_mod_version}"

    @staticmethod
    def build_user_agent(
        ua_type: UAType = UAType.HYSDK,
        platform: Platform = Platform.WINDOWS
    ) -> str:
        '''
        构建完整的用户代理字符串
        :param ua_type: UA 类型 (MEDIA_PLAYER 或 HYSDK)
        :param platform: 平台类型
        :return: 完整的用户代理字符串
        '''

        # 获取各个组件的 UA
        hyapp_ua = UAGenerator.get_hyapp_ua(platform)

        trans_mod_ua = UAGenerator.get_hy_trans_mod_ua(platform)

        if ua_type == UAType.MEDIA_PLAYER:
            media_player_ua = UAGenerator.get_hy_media_player_ua(platform)
            return f"{media_player_ua}_APP({hyapp_ua})_SDK({trans_mod_ua})"

        elif ua_type == UAType.HYSDK:
            sdk_platform = platform if platform in {Platform.ANDROID, Platform.HUYA_NFTV} else Platform.WINDOWS
            hysdk_ua = UAGenerator.get_hysdk_ua(sdk_platform)
            return f"{hysdk_ua}_APP({hyapp_ua})_SDK({trans_mod_ua})"

        else:
            raise ValueError(f"不支持的 UA 类型: {ua_type}")


def _raise_for_room_block(text: str):
    for err_key in ("找不到这个主播", "该主播涉嫌违规，正在整改中"):
        if err_key in text:
            raise Exception(err_key)


@alru_cache(maxsize=None)
async def _get_real_rid(url) -> str:
    resp = await client.get(url)
    resp.raise_for_status()
    _raise_for_room_block(resp.text)
    room_data = match1(resp.text, HUYA_WEB_ROOM_DATA_REGEX)
    room_data = json_loads(room_data)
    if not room_data.get('profileRoom'):
        raise Exception("找不到这个主播")
    return str(room_data['profileRoom'])