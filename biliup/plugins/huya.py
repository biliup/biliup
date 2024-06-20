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
        self.__room_id = None

    async def acheck_stream(self, is_check=False):
        try:
            self.__room_id = self.url.split('huya.com/')[1].split('/')[0].split('?')[0]
            if not self.__room_id:
                raise
        except:
            logger.error(f"Huya - {self.url}: 直播间地址错误")
            return False

        print(self.__room_id)
        html_info = await self._get_info_in_html()
        live_rate_info = html_info.get('vMultiStreamInfo', [])
        if not live_rate_info:
            # 无流 当做没开播
            logger.debug(f"{self.plugin_msg} : 未开播")
            return False

        if is_check:
            return True

        try:
            # 最大录制码率
            huya_max_ratio = config.get('huya_max_ratio', 0)
            # 最大码率(不含hdr)
            max_ratio = html_info['data'][0]['gameLiveInfo']['bitRate']
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
        # cdn_fallback = True

        stream_urls = await self._build_stream_url(protocol, allow_imgplus)
        if not stream_urls:
            # 一个错误要打印三行日志，太繁琐了
            logger.error(f"{self.plugin_msg}: 没有可用的链接")
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
            stream_urls = await self._build_stream_url(protocol, allow_imgplus)

        self.room_title = html_info['data'][0]['gameLiveInfo']['introduction']
        self.raw_stream_url = stream_urls[perf_cdn]

        if record_ratio != max_ratio:
            self.raw_stream_url += f"&ratio={record_ratio}"
        return True

    def danmaku_init(self):
        if self.huya_danmaku:
            self.danmaku = DanmakuClient(self.url, self.gen_download_filename())


    async def _get_info_in_html(self) -> dict:
        try:
            html = (await client.get(f"https://www.huya.com/{self.__room_id}", timeout=5, headers=self.fake_headers)).text
            if '找不到这个主播' in html:
                logger.error(f"{self.plugin_msg}: 找不到这个主播")
                return {}
            return json.loads(html.split('stream: ')[1].split('};')[0])
        except IndexError:
            logger.debug(f"{self.plugin_msg}: {html}")
        except:
            logger.exception(f"{self.plugin_msg}: get_info_in_html")
        return {}


    async def _build_stream_url(self, protocol, allow_imgplus=True) -> dict:
        '''
        返回指定协议的所有CDN流
        '''
        html_info = await self._get_info_in_html()
        if not html_info:
            return {}
        try:
            stream_info = html_info['data'][0]['gameStreamInfoList']
        except KeyError:
            logger.exception(f"{self.plugin_msg}: build_stream_url {html_info}")
            return {}
        stream = stream_info[0]
        stream_name = stream['sStreamName']
        suffix, anti_code = stream[f's{protocol}UrlSuffix'], stream[f's{protocol}AntiCode']
        if not allow_imgplus:
            stream_name = stream_name.replace('-imgplus', '')
        anti_code = build_query(stream_name, anti_code)
        if not anti_code:
            logger.error(f"{self.plugin_msg}: build_stream_url {stream_name} {anti_code}")
            return {}
        # HY 和 HYZJ 均为 P2P
        streams = {item['sCdnType']: \
                    f"{item[f's{protocol}Url']}/{stream_name}.{suffix}?{anti_code}" \
                    for item in stream_info if 'HY' not in item['sCdnType']}
        return streams


def build_query(sStreamName, sAntiCode) -> str:
    try:
        url_query = parse_qs(sAntiCode)
        platform_id = 100
        uid = random.randint(12340000, 12349999)
        convert_uid = (uid << 8 | uid >> (32 - 8)) & 0xFFFFFFFF
        ws_time = url_query['wsTime'][0]
        ct = int((int(ws_time, 16) + random.random()) * 1000)
        seq_id = uid + int(time.time() * 1000)
        ws_secret_prefix = base64.b64decode(unquote(url_query['fm'][0]).encode()).decode().split('_')[0]
        ws_secret_hash = hashlib.md5(f"{seq_id}|{url_query['ctype'][0]}|{platform_id}".encode()).hexdigest()
        ws_secret = hashlib.md5(f'{ws_secret_prefix}_{convert_uid}_{sStreamName}_{ws_secret_hash}_{ws_time}'.encode()).hexdigest()
    except:
        logger.exception("build_query")
        return ""
    # &codec=av1
    # &codec=264
    # &codec=265
    # dMod: wcs-25 浏览器解码信息
    # sdkPcdn: 1_1 第一个1连接次数 第二个1是因为什么连接
    # t: 100 平台信息 100 web
    # sv: 2401090219 版本
    # sdk_sid:  _sessionId sdkInRoomTs 当前毫秒时间

    # return f"wsSecret={ws_secret}&wsTime={ws_time}&seqid={seq_id}&ctype={url_query['ctype'][0]}&ver=1&fs={url_query['fs'][0]}&u={convert_uid}&t={platform_id}&sv=2401090219&sdk_sid={int(time.time() * 1000)}&codec=264"

    # https://github.com/hua0512/stream-rec/blob/ff0eb668e1f0fc160fe9b406bad79b5f570a4711/platforms/src/main/kotlin/github/hua0512/plugins/huya/download/HuyaExtractor.kt#L309
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
