import base64
import hashlib
import json
import random
import time
from urllib.parse import parse_qs, unquote

from biliup.common.util import client
from biliup.config import config
from biliup.Danmaku import DanmakuClient
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import logger


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m)\.)?huya\.com')
class Huya(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.huya_danmaku = config.get('huya_danmaku', False)
        self.fake_headers['referer'] = url
        self.fake_headers['cookie'] = config.get('user', {}).get('huya_cookie', '')
        self.__room_id = None

    async def acheck_stream(self, is_check=False):
        try:
            self.__room_id = self.url.split('huya.com/')[1]
            if not self.__room_id:
                raise
            if not self.__room_id.isdigit():
                html_info = await self.get_room_profile()
                if not html_info:
                    raise
                self.__room_id = html_info['data'][0]['gameLiveInfo']['profileRoom']
        except Exception as e:
            logger.error(f"{self.plugin_msg}: {e}")
            return False

        room_profile = await self.get_room_profile(use_api=True)
        if room_profile['realLiveStatus'] != 'ON' or room_profile['liveStatus'] != 'ON':
            '''
            ON: 直播
            REPLAY: 重播
            OFF: 未开播
            '''
            logger.debug(f"{self.plugin_msg} : 未开播")
            self.raw_stream_url = None
            return False

        # live_rate_info = html_info.get('vMultiStreamInfo', [])

        # if not live_rate_info:
        #     # 无流 当做没开播
        #     logger.debug(f"{self.plugin_msg} : 未开播")
        #     return False

        if is_check:
            return True

        try:
            # 最大录制码率
            huya_max_ratio = config.get('huya_max_ratio', 0)
            # 最大码率(不含hdr)
            # max_ratio = html_info['data'][0]['gameLiveInfo']['bitRate']
            max_ratio = room_profile['liveData']['bitRate']
            # 可选择的码率
            live_rate_info = json.loads(room_profile['liveData']['bitRateInfo'])
            # 码率信息
            ratio_items = [r.get('iBitRate', 0) if r.get('iBitRate', 0) != 0 else max_ratio for r in live_rate_info]
            # 符合条件的码率
            ratio_in_items = [x for x in ratio_items if x <= huya_max_ratio]
            # 录制码率
            if ratio_in_items:
                record_ratio = max(ratio_in_items)
            else:
                record_ratio = max_ratio
        except Exception as e:
            logger.error(f"{self.plugin_msg}: 在确定码率时发生错误 {e}")
            return False

        huya_cdn = config.get('huyacdn', 'AL')
        perf_cdn = config.get('huya_cdn', huya_cdn).upper()
        protocol = 'Hls' if config.get('huya_protocol') == 'Hls' else 'Flv'
        # protocol = 'Hls'
        allow_imgplus = config.get('huya_imgplus', True)
        cdn_fallback = config.get('huya_cdn_fallback', False)
        use_api = True if config.get('huya_obtain_method', '') == 'api' else False
        use_api = True

        try:
            stream_urls = await self.get_stream_urls(protocol, use_api, allow_imgplus)
        except:
            logger.exception(f"{self.plugin_msg}: 没有可用的链接")
            return False

        cdn_name_list = list(stream_urls.keys())
        if perf_cdn not in cdn_name_list:
            logger.warning(f"{self.plugin_msg}: {perf_cdn} CDN不存在，自动切换到 {cdn_name_list[0]}")
            perf_cdn = cdn_name_list[0]

        # 虎牙直播流只允许连接一次
        if cdn_fallback:
            _url = await self.acheck_url_healthy(stream_urls[perf_cdn])
            if _url is None:
                logger.info(f"{self.plugin_msg}: 提供如下CDN {cdn_name_list}")
                for cdn in cdn_name_list:
                    if cdn == perf_cdn:
                        continue
                    logger.info(f"{self.plugin_msg}: cdn_fallback 尝试 {cdn}")
                    if (await self.acheck_url_healthy(stream_urls[cdn])) is None:
                        continue
                    perf_cdn = cdn
                    logger.info(f"{self.plugin_msg}: CDN 切换为 {perf_cdn}")
                    break
                else:
                    logger.error(f"{self.plugin_msg}: cdn_fallback 所有链接无法使用")
                    return False
            stream_urls = await self.get_stream_urls(protocol, use_api, allow_imgplus)

        # self.room_title = html_info['data'][0]['gameLiveInfo']['introduction']
        self.room_title = room_profile['liveData']['introduction']
        self.raw_stream_url = stream_urls[perf_cdn]

        if record_ratio != max_ratio:
            self.raw_stream_url += f"&ratio={record_ratio}"
        return True

    def danmaku_init(self):
        if self.huya_danmaku:
            self.danmaku = DanmakuClient(self.url, self.gen_download_filename())


    async def get_room_profile(self, use_api=False) -> dict:
        if use_api:
            resp = (await client.get(f"https://mp.huya.com/cache.php?m=Live&do=profileRoom&roomid={self.__room_id}")).json()
            if resp['status'] != 200:
                raise Exception(f"{self.plugin_msg}: {resp['message']}")
            return resp['data']
        else:
            html = (await client.get(f"https://www.huya.com/{self.__room_id}", headers=self.fake_headers)).text
            if '找不到这个主播' in html:
                raise Exception(f"{self.plugin_msg}: 找不到这个主播")
            return json.loads(html.split('stream: ')[1].split('};')[0])

    async def get_stream_urls(self, protocol, use_api=False, allow_imgplus=True) -> dict:
        '''
        返回指定协议的所有CDN流
        '''
        streams = {}
        room_profile = await self.get_room_profile(use_api=use_api)
        if not use_api:
            try:
                stream_info = room_profile['data'][0]['gameStreamInfoList']
            except KeyError:
                raise Exception(f"{self.plugin_msg}: {room_profile}")
        else:
            stream_info = room_profile['stream']['baseSteamInfoList']
            streams = self.__dict_sorting(json.loads(room_profile['liveData']['mStreamRatioWeb']))
        stream = stream_info[0]
        stream_name = stream['sStreamName']
        suffix, anti_code = stream[f's{protocol}UrlSuffix'], stream[f's{protocol}AntiCode']
        if not allow_imgplus:
            stream_name = stream_name.replace('-imgplus', '')
        anti_code = self.__build_query(stream_name, anti_code, )
        for stream in stream_info:
            if stream['sCdnType'] in ['HY', 'HUYA', 'HYZJ']: continue
            streams[stream['sCdnType']] = f"{stream[f's{protocol}Url']}/{stream_name}.{suffix}?{anti_code}"
        return streams

    def __build_query(self, stream_name, anti_code) -> str:
        url_query = parse_qs(anti_code)
        platform_id = 100
        uid = self.__get_uid(self.fake_headers['cookie'])
        convert_uid = (uid << 8 | uid >> (32 - 8)) & 0xFFFFFFFF
        ws_time = url_query['wsTime'][0]
        ct = int((int(ws_time, 16) + random.random()) * 1000)
        seq_id = uid + int(time.time() * 1000)
        ws_secret_prefix = base64.b64decode(unquote(url_query['fm'][0]).encode()).decode().split('_')[0]
        ws_secret_hash = hashlib.md5(f"{seq_id}|{url_query['ctype'][0]}|{platform_id}".encode()).hexdigest()
        ws_secret = hashlib.md5(f'{ws_secret_prefix}_{convert_uid}_{stream_name}_{ws_secret_hash}_{ws_time}'.encode()).hexdigest()
        # &codec=av1
        # &codec=264
        # &codec=265
        # dMod: wcs-25 浏览器解码信息
        # sdkPcdn: 1_1 第一个1连接次数 第二个1是因为什么连接
        # t: 100 平台信息 100 web
        # sv: 2401090219 版本
        # sdk_sid:  _sessionId sdkInRoomTs 当前毫秒时间

        # return f"wsSecret={ws_secret}&wsTime={ws_time}&seqid={seq_id}&ctype={url_query['ctype'][0]}&ver=1&fs={url_query['fs'][0]}&u={convert_uid}&t={platform_id}&sv=2401090219&sdk_sid={int(time.time() * 1000)}&codec=264"

        anti_code = {
            "wsSecret": ws_secret,
            "wsTime": ws_time,
            "seqid": str(seq_id),
            "ctype": url_query['ctype'][0],
            "fs": url_query['fs'][0],
            "u": convert_uid,
            "t": platform_id,
            "ver": "1",
            "uuid": str(int((ct % 1e10 + random.random()) * 1e3 % 0xffffffff)),
            "sdk_sid": str(int(time.time() * 1000)),
            "codec": "264",
        }
        return '&'.join([f"{k}={v}" for k, v in anti_code.items()])

    @staticmethod
    def __dict_sorting(data: dict) -> dict:
        data = {k: v for k, v in data.items() if k not in ['HY', 'HUYA', 'HYZJ']}
        return dict(sorted(data.items(), key=lambda x: x[1], reverse=True))

    @staticmethod
    def __get_uid(cookie: str) -> int:
        if cookie:
            cookie_dict = {k.strip(): v for k, v in (item.split('=') for item in cookie.split(';'))}
            for key in ['udb_uid', 'yyuid']:
                if key in cookie_dict:
                    return int(cookie_dict[key])
        return random.randint(1400000000000, 1499999999999)