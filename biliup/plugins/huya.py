import base64
import hashlib
import random
import time
import html
from enum import Enum
from dataclasses import dataclass
from urllib.parse import parse_qs, unquote, quote
from async_lru import alru_cache
from typing import (
    Any,
    Dict,
    List,
    Union,
)

from ..common.util import client
from ..Danmaku import DanmakuClient
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from . import logger, match1, json_loads

from biliup.plugins.huya_wup import Wup, DEFAULT_TICKET_NUMBER
from biliup.plugins.huya_wup.packet import *
from biliup.plugins.huya_wup.wup_struct.UserId import HuyaUserId

HUYA_WEB_BASE_URL = "https://www.huya.com"
HUYA_MOBILE_BASE_URL = "https://m.huya.com"
HUYA_MP_BASE_URL = "https://mp.huya.com"
HUYA_WUP_BASE_URL = "https://wup.huya.com"
HUYA_WUP_YST_URL = "https://snmhuya.yst.aisee.tv"
HUYA_WEB_ROOM_DATA_REGEX = r"var TT_ROOM_DATA = (.*?);"

rotl64 = lambda t: ((t & 0xFFFFFFFF) << 8 | (t & 0xFFFFFFFF) >> 24) & 0xFFFFFFFF | (t & ~0xFFFFFFFF)

@Plugin.download(regexp=r'https?://(?:(?:www|m)\.)?huya\.com')
class Huya(DownloadBase):
    def __init__(self, fname, url, config, suffix='flv'):
        super().__init__(fname, url, config, suffix)
        self.__room_id = url.split('huya.com/')[1].split('?')[0]
        self.huya_danmaku = config.get('huya_danmaku', False)
        self.huya_max_ratio = config.get('huya_max_ratio', 0)
        self.huya_cdn = config.get('huya_cdn', "").upper()  # 不填写时使用主播的CDN优先级
        self.huya_protocol = 'Hls' if config.get('huya_protocol') == 'Hls' else 'Flv'
        self.huya_imgplus = config.get('huya_imgplus', True)
        self.huya_cdn_fallback = config.get('huya_cdn_fallback', False)
        self.huya_mobile_api = config.get('huya_mobile_api', False)
        self.huya_codec = config.get('huya_codec', '264')
        self.huya_use_wup = (not self.huya_mobile_api)

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
        # 过滤回放
        for key in ["回放", "重播"]:
            # 前三或后三
            if key in self.room_title[:3] or key in self.room_title[-3:]:
                logger.debug(f"{self.plugin_msg}: {self.room_title}")
                return False

        if is_check:
            return True

        stream_urls = await self.build_stream_urls(room_profile['streams_info'])
        print(stream_urls)
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
            stream_urls = await self.build_stream_urls(room_profile['streams_info'])

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
            streams_info: List[Dict[str, Any]]
        ) -> List[Dict[str, Any]]:
        '''
        构建流地址
        :param streams_info: 流信息
        :return: 流地址
        '''
        proto = self.huya_protocol
        streams = {}
        weights = {} # https://cdnweb.huya.com/getUidsDomainList?anchor_uid={anchor_uid}
        cached_anticode = ""
        for stream in streams_info:
            # 优先级<0代表不可用
            priority = stream['iWebPriorityRate']
            if priority < 0:
                continue
            stream_name = self.get_stream_name(stream['sStreamName'])
            cdn = stream['sCdnType']
            suffix = stream[f's{proto}UrlSuffix']
            if not cached_anticode:
                # 小程序API不修改
                if self.huya_mobile_api and self.huya_imgplus:
                    cached_anticode = stream[f's{proto}AntiCode']
                else:
                    cached_anticode = self.build_anticode(
                        stream_name,
                        (await self.get_cdn_token_info_ex(stream_name)),
                        stream['lPresenterUid']
                    )
                cached_anticode += f"&codec={self.huya_codec}"
            base_url = stream[f's{proto}Url'].replace('http://', 'https://') # 强制https
            streams[cdn] = f"{base_url}/{stream_name}.{suffix}?{cached_anticode}"
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


    async def get_cdn_token_info_ex(self, stream_name: str) -> str:
        '''
        getCdnTokenInfoEx
        :param stream_name: stream_name
        :param presenter_uid: 主播uid
        :return: sFlvToken
        '''
        servant = "liveui"
        func = "getCdnTokenInfoEx"
        tid = HuyaUserId()
        # Generate random sHuYaUA using UAGenerator
        tid.sHuYaUA = UAGenerator.get_random_hyapp_ua()
        print(f"sHuYaUA: {tid.sHuYaUA}")
        wup_req = Wup()
        wup_req.requestid = abs(DEFAULT_TICKET_NUMBER)
        wup_req.servant = servant
        wup_req.func = func
        getCdnTokenInfoExReq = HuyaGetCdnTokenExReq()
        getCdnTokenInfoExReq.sStreamName = stream_name
        # getCdnTokenInfoExReq.iLoopTime = 15 * 60    # 防盗链过期时间
        getCdnTokenInfoExReq.tId = tid
        wup_req.put(
            vtype=HuyaGetCdnTokenExReq,
            name="tReq",
            value=getCdnTokenInfoExReq
        )
        data = wup_req.encode_v3()
        url = HUYA_WUP_BASE_URL
        if random.random() > 0.5:
            url = f"{HUYA_WUP_YST_URL}/{servant}/{func}"
        print(f"send requests to {url}")
        rsp = await client.post(url, data=data)
        rsp_bytes = rsp.content
        wup_rsp = Wup()
        wup_rsp.decode_v3(rsp_bytes)
        getCdnTokenInfoExRsp = wup_rsp.get(
            vtype=HuyaGetCdnTokenExRsp,
            name="tRsp"
        )
        cdn_token_info_ex = getCdnTokenInfoExRsp.as_dict()
        print(f"{self.plugin_msg}: wup token_info {cdn_token_info_ex}")
        return cdn_token_info_ex['sFlvToken']


    def build_anticode(
            self,
            stream_name: str,
            anti_code: str,
            uid: Union[str, int] = 0,
            random_platform: bool = False
        ) -> str:
        '''
        构建anticode
        :param stream_name: streamname
        :param anti_code: anticode
        :param uid: 用户uid
        :return: Parsed anticode
        '''
        url_query = parse_qs(anti_code)
        if not url_query.get("fm"):
            return anti_code

        ctype = url_query.get('ctype', [])
        platform_id = url_query.get('t', [])
        if len(ctype) == 0 or random_platform:
            ctype, platform_id = PLATFORM.get_random_as_tuple()
        elif len(platform_id) == 0:
            ctype = ctype[0]
            platform_id = PLATFORM.get_platform_id(ctype)
        else:
            ctype = ctype[0]
            platform_id = platform_id[0]

        is_wap = int(platform_id) in {103}
        clac_start_time = time.time()

        if isinstance(uid, str):
            uid = int(uid) if uid.isdigit() else 0
        if uid == 0:
            uid = self.generate_random_uid()
        print(f"Using {uid} as uid for calculation")
        seq_id = uid + int(clac_start_time * 1000)
        secret_hash = hashlib.md5(f"{seq_id}|{ctype}|{platform_id}".encode()).hexdigest()
        convert_uid = rotl64(uid)
        clac_uid = uid if is_wap else convert_uid

        fm = unquote(url_query['fm'][0])
        secret_prefix = base64.b64decode(fm.encode()).decode().split('_')[0]

        ws_time = url_query['wsTime'][0]
        # 修复 hls m3u8 链接过期时间
        if int(ws_time, 16) - int(clac_start_time) < (20 * 60):
            # 如果过期时间小于 20 分钟，调整过期时间为 1 天
            ws_time = hex(24 * 60 * 60 + int(clac_start_time))[2:]
        secret_str = f'{secret_prefix}_{clac_uid}_{stream_name}_{secret_hash}_{ws_time}'
        ws_secret = hashlib.md5(secret_str.encode()).hexdigest()

        ct = int((int(ws_time, 16) + random.random()) * 1000)
        uuid = str(int((ct % 1e10 + random.random()) * 1e3 % 0xffffffff))

        anti_code = {
            "wsSecret": ws_secret,
            "wsTime": ws_time,
            "seqid": seq_id,
            "ctype": ctype,
            "ver": "1",
            "fs": url_query['fs'][0],
            "fm": quote(url_query['fm'][0], encoding='utf-8'),
            "t": platform_id,
        }
        if is_wap:
            anti_code.update({
                "uid": uid,
                "uuid": uuid,
            })
        else:
            anti_code.update({
                "u": convert_uid,
            })

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
    def generate_random_uid() -> int:
        return int(f"1234{random.randint(0, 9999):04d}") \
               if random.random() > 0.5 else \
               int(f"140000{random.randint(0, 9999999):07d}")


    # async def get_anonymous_uid(self) -> int:
    #     try:
    #         rsp = await client.post(
    #             url='https://udblgn.huya.com/web/anonymousLogin',
    #             headers=self.fake_headers,
    #             json={
    #                 "appId": 5002,
    #                 "byPass": 3,
    #                 "context": "",
    #                 "version": "2.4",
    #                 "data": {},
    #             }
    #         )
    #         rsp = json_loads(rsp.text)
    #     except:
    #         rsp = {}
    #     return rsp.get('data', {}).get('uid', self.get_uid())

    # def update_headers(self, headers: dict):
    #     if self.huya_use_wup:
    #         user_agent = UAGenerator.build_user_agent(UAType.HYSDK, Platform.WINDOWS)
    #         # user_agent = f"{Huya.get_hysdk_ua()}_APP({Huya.get_hyapp_ua()})_SDK({Huya.get_hy_trans_mod_ua()})"
    #         headers['user-agent'] = user_agent
    #         headers['origin'] = HUYA_WEB_BASE_URL

class PLATFORM(Enum):
    HUYA_PC_EXE = 0
    HUYA_ADR = 2
    HUYA_IOS = 3
    TV_HUYA_NFTV = 10
    HUYA_WEBH5 = 100
    HUYA_LIVE = 100
    TARS_MP = 102
    TARS_MOBILE = 103
    HUYA_LIVESHAREH5 = 104

    @classmethod
    def get_random_as_tuple(cls):
        _ = random.choice(list(cls))
        return _.name.lower(), _.value

    @classmethod
    def get_platform_id(cls, platform: str) -> int:
        return cls[platform.upper()].value if platform.upper() in cls.__members__ else 100

    @property
    def short_name(self) -> str:
        """获取平台短名称"""
        name = self.name.lower()
        idx = name.find('_')
        return name[idx + 1:] if idx != -1 else name


class UAGenerator:
    # Configuration dictionary mapping PLATFORM enum to UA components
    HYAPP_CONFIGS = {
        PLATFORM.HUYA_ADR: {
            'version': '13.1.0',  # LocalVersion or "0.0.0" + hotfix_version
        },
        PLATFORM.HUYA_IOS: {
            'version': '13.1.0',
        },
        PLATFORM.TV_HUYA_NFTV: {
            'version': '2.6.10',
        },
        PLATFORM.HUYA_PC_EXE: {
            'version': '7000000',
        },
        # PLATFORM.HUYA_WEBH5: {       # 星秀区不可用
        #     'version': '%y%m%d%H%M', # 2410101630
        #     'channel': 'websocket'
        # }
    }


    @staticmethod
    def generate_hyapp_ua(platform: PLATFORM) -> str:
        '''
        Generate hyapp user agent string
        :param platform: Platform type from PLATFORM enum
        :return: User agent string
        '''
        config = UAGenerator.HYAPP_CONFIGS.get(platform)
        if not config:
            # Fallback if platform not supported
            platform = random.choice(list(UAGenerator.HYAPP_CONFIGS.keys()))
            config = UAGenerator.HYAPP_CONFIGS[platform]

        hyapp_platform = platform.short_name
        hyapp_version = config.get("version", "0.0.0")   # TODO: 日期格式化
        hyapp_channel = config.get("channel", "official")

        # Add random build number for version
        if platform in {PLATFORM.HUYA_ADR, PLATFORM.TV_HUYA_NFTV}:
            hyapp_version += f".{random.randint(3000, 5000)}"

        ua = f"{hyapp_platform}&{hyapp_version}&{hyapp_channel}"
        
        # Add android_api_level for android platforms
        if platform in {PLATFORM.HUYA_ADR, PLATFORM.TV_HUYA_NFTV}:
            android_api_level = random.randint(28, 36)
            ua = f"{ua}&{android_api_level}"

        return ua


    @staticmethod
    def get_random_hyapp_ua() -> str:
        '''
        Generate random hyapp user agent string by randomly selecting a platform
        :return: User agent string
        '''
        random_platform = random.choice(list(PLATFORM))
        return UAGenerator.generate_hyapp_ua(random_platform)


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