import base64
import hashlib
import json
import random
import time
from urllib.parse import parse_qs, unquote

import requests

from biliup.config import config
from biliup.plugins.Danmaku import DanmakuClient
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import logger, random_user_agent

@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m)\.)?huya\.com')
class Huya(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.huya_danmaku = config.get('huya_danmaku', False)

    def check_stream(self, is_check=False):
        try:
            room_id = self.url.split('huya.com/')[1].split('/')[0].split('?')[0]
            if not room_id:
                raise
            plugin_msg = f"Huya - {room_id}"
        except:
            logger.error(f"Huya - {self.url}: 直播间地址错误")
            return False

        html_info = _get_info_in_html(room_id, self.fake_headers)
        live_rate_info = html_info.get('vMultiStreamInfo', None)
        if not live_rate_info:
            # 无流 当做没开播
            logger.debug(f"{plugin_msg} : 未开播")
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
            logger.error(f"{plugin_msg}: 在确定码率时发生错误 {e}")

        huya_cdn = config.get('huyacdn', 'AL')
        perf_cdn = config.get('huya_cdn', huya_cdn).upper()
        cdn_fallback = config.get('huya_cdn_fallback', False)
        # cdn_fallback = True

        stream_url, sCdns = _build_stream_url(room_id, perf_cdn, self.fake_headers)
        # stream_url = None
        if not stream_url:
            logger.error(f"{plugin_msg}: 无法获取流地址")
            return False

        # 虎牙直播流只允许连接一次，非常丑陋的代码
        if cdn_fallback:
            with requests.Session() as s:
                s.headers = self.fake_headers.copy()
                url_health, _ = super().check_url_healthy(s, stream_url)
                if not url_health:
                    logger.debug(f"{plugin_msg}: {list(sCdns.keys())}")
                    for sCdn in sCdns.keys():
                        if sCdn == perf_cdn:
                            continue
                        logger.warning(f"{plugin_msg}: {perf_cdn} 无法连接，尝试 {sCdn}")
                        stream_url, _ = _build_stream_url(room_id, sCdn, self.fake_headers)
                        url_health, _ = super().check_url_healthy(s, stream_url)
                        if url_health:
                            perf_cdn = sCdn
                            logger.warning(f"{plugin_msg}: CDN 切换为 {perf_cdn}")
                            stream_url, _ = _build_stream_url(room_id, perf_cdn, self.fake_headers)
                            logger.debug(f"{plugin_msg}: {stream_url}")
                            break
                    else:
                        return False

        self.room_title = html_info['data'][0]['gameLiveInfo']['introduction']
        self.raw_stream_url = stream_url

        if record_ratio != max_ratio:
            self.raw_stream_url += f"&ratio={record_ratio}"
        return True

    def danmaku_download_start(self, filename):
        if self.huya_danmaku:
            self.danmaku = DanmakuClient(self.url, filename + "." + self.suffix)
            self.danmaku.start()

    def close(self):
        if self.danmaku:
            self.danmaku.stop()

def _get_info_in_html(room_id, fake_headers):
    try:
        html = requests.get(f"https://www.huya.com/{room_id}", timeout=5, headers=fake_headers).text
        if '找不到这个主播' in html:
            logger.error(f"Huya - {room_id}: 找不到这个主播")
            return {}
    except:
        logger.exception(f"Huya - {room_id}: get_info_in_html")
        return {}
    return json.loads(html.split('stream: ')[1].split('};')[0])

def _build_stream_url(room_id, perf_cdn, fake_headers):
    html_info = _get_info_in_html(room_id, fake_headers)
    try:
        streamInfo = html_info['data'][0]['gameStreamInfoList']
    except KeyError:
        logger.exception(f"Huya - {room_id}: build_stream_url {html_info}")
        return None, None
    stream = streamInfo[0]
    sFlvUrlSuffix, sStreamName, sFlvAntiCode = \
        stream['sFlvUrlSuffix'], stream['sStreamName'], stream['sFlvAntiCode']
    sCdns = {item['sCdnType']: item['sFlvUrl'] for item in streamInfo if item['sCdnType'] != 'HY'}
    sFlvUrl = sCdns.get(perf_cdn)
    _stream_url = f'{sFlvUrl}/{sStreamName}.{sFlvUrlSuffix}?{_make_query(sStreamName, sFlvAntiCode)}'
    return _stream_url, sCdns

def _make_query(sStreamName, sFlvAntiCode):
    url_query = parse_qs(sFlvAntiCode)
    platform_id = 100
    uid = random.randint(12340000, 12349999)
    convert_uid = (uid << 8 | uid >> (32 - 8)) & 0xFFFFFFFF
    ws_time = url_query['wsTime'][0]
    seq_id = uid + int(time.time() * 1000)
    ws_secret_prefix = base64.b64decode(unquote(url_query['fm'][0]).encode()).decode().split('_')[0]
    ws_secret_hash = hashlib.md5(f"{seq_id}|{url_query['ctype'][0]}|{platform_id}".encode()).hexdigest()
    ws_secret = hashlib.md5(f'{ws_secret_prefix}_{convert_uid}_{sStreamName}_{ws_secret_hash}_{ws_time}'.encode()).hexdigest()
    # &codec=av1
    # &codec=264
    # &codec=265
    # dMod: wcs-25 浏览器解码信息
    # sdkPcdn: 1_1 第一个1连接次数 第二个1是因为什么连接
    # t: 100 平台信息 100 web
    # sv: 2401090219 版本
    # sdk_sid:  _sessionId sdkInRoomTs 当前毫秒时间
    return f"wsSecret={ws_secret}&wsTime={ws_time}&seqid={seq_id}&ctype={url_query['ctype'][0]}&ver=1&fs={url_query['fs'][0]}&u={convert_uid}&t={platform_id}&sv=2401090219&sdk_sid={int(time.time() * 1000)}&codec=264"