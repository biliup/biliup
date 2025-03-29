import base64
import hashlib
import json
import random
import time
from urllib.parse import parse_qs, unquote
from functools import lru_cache

from biliup.common.util import client
from biliup.config import config
from biliup.Danmaku import DanmakuClient
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import logger, match1, random_user_agent

HUYA_WEB_BASE_URL = "https://www.huya.com"
HUYA_MOBILE_BASE_URL = "https://m.huya.com"


@Plugin.download(regexp=r'https?://(?:(?:www|m)\.)?huya\.com')
class Huya(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.fake_headers['referer'] = url
        # self.fake_headers['cookie'] = config.get('user', {}).get('huya_cookie', '')
        self.__room_id = url.split('huya.com/')[1].split('?')[0]
        self.huya_danmaku = config.get('huya_danmaku', False)
        self.huya_max_ratio = config.get('huya_max_ratio', 0)
        self.huya_cdn = config.get('huya_cdn', "").upper()  # 不填写时使用主播的CDN优先级
        self.huya_protocol = 'Hls' if config.get('huya_protocol') == 'Hls' else 'Flv'
        self.huya_imgplus = config.get('huya_imgplus', True)
        self.huya_cdn_fallback = config.get('huya_cdn_fallback', False)
        self.huya_mobile_api = config.get('huya_mobile_api', False)

    async def acheck_stream(self, is_check=False):
        try:
            if not self.__room_id.isdigit():
                self.__room_id = _get_real_rid(self.url)
            room_profile = await self.get_room_profile(use_api=True)
        except Exception as e:
            logger.error(f"{self.plugin_msg}: {e}")
            return False

        if room_profile['realLiveStatus'] != 'ON' or room_profile['liveStatus'] != 'ON':
            '''
            ON: 直播
            REPLAY: 重播
            OFF: 未开播
            '''
            logger.debug(f"{self.plugin_msg}: 未开播")
            self.raw_stream_url = None
            return False

        if not room_profile['liveData'].get('bitRateInfo'):
            # 主播未推流
            logger.debug(f"{self.plugin_msg}: 未推流")
            return False

        self.room_title = room_profile['liveData']['introduction']
        # 虎牙回放
        if self.room_title.startswith('【回放】'):
            logger.debug(f"{self.plugin_msg}: {self.room_title}")
            return False

        if is_check:
            return True

        if self.huya_max_ratio:
            try:
                self.huya_max_ratio = int(self.huya_max_ratio)
                # 最大码率(不含hdr)
                # max_ratio = html_info['data'][0]['gameLiveInfo']['bitRate']
                max_ratio = room_profile['liveData']['bitRate']
                # 可选择的码率
                live_rate_info = json.loads(room_profile['liveData']['bitRateInfo'])
                # 码率信息
                ratio_items = [r.get('iBitRate', 0) if r.get('iBitRate', 0) != 0 else max_ratio for r in live_rate_info]
                # 符合条件的码率
                ratio_in_items = [x for x in ratio_items if x <= self.huya_max_ratio]
                # 录制码率
                if ratio_in_items:
                    record_ratio = max(ratio_in_items)
                else:
                    record_ratio = max_ratio
            except (json.JSONDecodeError, KeyError) as e:
                logger.error(f"{self.plugin_msg}: 在确定码率时发生错误 {e}")
                return False

        is_xingxiu = (room_profile['liveData']['gid'] == 1663)

        try:
            stream_urls = await self.get_stream_urls(self.huya_protocol, self.huya_mobile_api, self.huya_imgplus, is_xingxiu)
        except Exception as e:
            logger.error(f"{self.plugin_msg}: 没有可用的链接 {e}")
            return False

        cdn_name = list(stream_urls.keys())
        if not self.huya_cdn or self.huya_cdn not in cdn_name:
            self.huya_cdn = cdn_name[0]

        # Thx stream-rec
        update_user_agent(self.fake_headers)

        # 虎牙直播流只允许连接一次
        if self.huya_cdn_fallback:
            _url = await self.acheck_url_healthy(stream_urls[self.huya_cdn])
            if _url is None:
                logger.info(f"{self.plugin_msg}: cdn_fallback 顺序尝试 {cdn_name}")
                for cdn in cdn_name:
                    logger.info(f"{self.plugin_msg}: cdn_fallback-{cdn}")
                    if (await self.acheck_url_healthy(stream_urls[cdn])) is None:
                        continue
                    self.huya_cdn = cdn
                    logger.info(f"{self.plugin_msg}: cdn_fallback 回退到 {self.huya_cdn}")
                    break
                else:
                    logger.error(f"{self.plugin_msg}: cdn_fallback 所有链接无法使用")
                    return False
            stream_urls = await self.get_stream_urls(self.huya_protocol, self.huya_mobile_api, self.huya_imgplus, is_xingxiu)

        self.raw_stream_url = stream_urls[self.huya_cdn]

        if self.huya_max_ratio and record_ratio != max_ratio:
            self.raw_stream_url += f"&ratio={record_ratio}"
        return True

    def danmaku_init(self):
        if self.huya_danmaku:
            self.danmaku = DanmakuClient(self.url, self.gen_download_filename())

    async def get_room_profile(self, use_api=False) -> dict:
        if use_api:
            params = {
                'm': 'Live',
                'do': 'profileRoom',
                'roomid': self.__room_id,
                'showSecret': 1,
            }
            resp = (await client.get(f"https://mp.huya.com/cache.php",
                                     headers=self.fake_headers, params=params)).json()
            if resp['status'] != 200:
                raise Exception(f"{resp['message']}")
            return resp['data']
        else:
            html = (await client.get(f"https://www.huya.com/{self.__room_id}", headers=self.fake_headers)).text
            if "找不到这个主播" in html:
                raise Exception(f"找不到这个主播")
            return json.loads(html.split('stream: ')[1].split('};')[0])

    async def get_stream_urls(self, protocol, use_api=False, allow_imgplus=True, is_xingxiu=False) -> dict:
        '''
        返回指定协议的所有CDN流
        '''
        streams = {}
        weights = {}  # https://cdnweb.huya.com/getUidsDomainList?anchor_uid={anchor_uid}
        room_profile = await self.get_room_profile(use_api=use_api)
        if not use_api:
            try:
                stream_info = room_profile['data'][0]['gameStreamInfoList']
            except KeyError:
                raise Exception(f"{room_profile}")
        else:
            stream_info = room_profile['stream']['baseSteamInfoList']
            weights = json.loads(room_profile['liveData'].get('mStreamRatioWeb', '{}'))
        stream = stream_info[0]
        stream_name = stream['sStreamName']
        suffix, anti_code = stream[f's{protocol}UrlSuffix'], stream[f's{protocol}AntiCode']
        if not allow_imgplus:
            stream_name = stream_name.replace('-imgplus', '')
        anti_code = anti_code + "&codec=264" \
            if is_xingxiu else \
            self.__build_query(stream_name, anti_code, _get_uid(self.fake_headers.get('cookie', ''), stream_name))
        for stream in stream_info:
            # 优先级<0代表不可用
            priority = stream['iWebPriorityRate']
            if priority < 0:
                continue
            cdn_name = stream['sCdnType']
            secure_url = stream[f's{protocol}Url'].replace('http://', 'https://')
            stream_url = f"{secure_url}/{stream_name}.{suffix}?{anti_code}"
            streams[cdn_name] = stream_url
            if cdn_name not in weights:
                weights[cdn_name] = priority
        return _weight_sorting(streams, weights)

    @staticmethod
    def __build_query(stream_name, anti_code, uid: int) -> str:
        url_query = parse_qs(anti_code)
        # platform_id = 100
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
            "codec": "264",
        }
        return '&'.join([f"{k}={v}" for k, v in anti_code.items()])


@lru_cache(maxsize=None)
def _get_real_rid(url) -> str:
    import requests
    headers = {
        'user-agent': random_user_agent(),
    }
    resp = requests.get(url, headers=headers)
    if '找不到这个主播' in resp.text:
        raise Exception(f"找不到这个主播")
    hy_config = json.loads(resp.text.split('stream: ')[1].split('};')[0])
    return str(hy_config['data'][0]['gameLiveInfo']['profileRoom'])


def _weight_sorting(data: dict, weights: dict) -> dict:
    if data:
        data = {k: v for k, v in data.items() if k not in ['HY', 'HUYA', 'HYZJ']}
        return dict(sorted(data.items(), key=lambda x: weights[x[0]], reverse=True))
    return {}


def _get_uid(cookie: str, stream_name: str) -> int:
    try:
        if cookie and "yyuid=" in cookie:
            return int(match1(r'yyuid=(\d+)', cookie))
        if stream_name:
            anchor_uid = int(stream_name.split('-')[0])
            if anchor_uid > 0:
                return anchor_uid
    except:
        pass
    return random.randint(1400000000000, 1499999999999)
    # udbAnonymousUid = requests.post(
    #     url='https://udblgn.huya.com/web/anonymousLogin',
    #     headers={
    #         'user-agent': random_user_agent(),
    #     },
    #     json={
    #         "appId": 5002,
    #         "byPass": 3,
    #         "context": "",
    #         "version": "2.4",
    #         "data": {},
    #     }
    # )['data']['uid']


def update_user_agent(headers: dict):
    headers['User-Agent'] = f"HYSDK(Windows, {int(time.time())})"
