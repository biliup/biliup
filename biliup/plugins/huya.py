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
        except:
            logger.warning(f"{Huya.__name__}: {self.url}: 直播间地址错误")
            return False
        try:
            html = requests.get(f'https://www.huya.com/{room_id}', timeout=5, headers=self.fake_headers).text
        except:
            logger.warning(f"{Huya.__name__}: {self.url}: 获取错误，本次跳过")
            return False

        if '找不到这个主播' in html:
            logger.warning(f"{Huya.__name__}: {self.url}: 直播间地址错误")
            return False

        try:
            html_info = json.loads(html.split('stream: ')[1].split('};')[0])
            live_info = html_info['data'][0]
            live_rate_info = html_info['vMultiStreamInfo']
            if not live_rate_info:
                # 无流 当做没开播
                logger.debug(f"{Huya.__name__}: {self.url}: 未开播")
                return False

            # 最大录制码率
            huya_max_ratio = config.get('huya_max_ratio', 0)
            # 最大码率(不含hdr)
            max_ratio = live_info['gameLiveInfo']['bitRate']
            # 码率信息
            ratio_items = [r.get("iBitRate", 0) if r.get("iBitRate", 0) != 0 else max_ratio for r in live_rate_info]
            # 符合条件的码率
            ratio_in_items = [x for x in ratio_items if x <= huya_max_ratio]
            # 录制码率
            if ratio_in_items:
                record_ratio = max(ratio_in_items)
            else:
                record_ratio = max_ratio

            # 自选cdn
            huya_cdn = config.get('huyacdn', 'AL')
            # 流信息
            stream_items = live_info['gameStreamInfoList']
            # 自选的流
            stream_selected = stream_items[0]
            for stream_item in stream_items:
                if stream_item['sCdnType'] == huya_cdn:
                    stream_selected = stream_item
                    break

            url_query = parse_qs(stream_selected["sFlvAntiCode"])
            platform_id = 100
            uid = random.randint(12340000, 12349999)
            convert_uid = (uid << 8 | uid >> (32 - 8)) & 0xFFFFFFFF
            ws_time = url_query['wsTime'][0]
            seq_id = uid + int(time.time() * 1000)
            ws_secret_prefix = base64.b64decode(unquote(url_query['fm'][0]).encode()).decode().split("_")[0]
            ws_secret_hash = hashlib.md5(f'{seq_id}|{url_query["ctype"][0]}|{platform_id}'.encode()).hexdigest()
            ws_secret = hashlib.md5(
                f'{ws_secret_prefix}_{convert_uid}_{stream_selected["sStreamName"]}_{ws_secret_hash}_{ws_time}'.encode()).hexdigest()
            # &codec=av1
            # &codec=264
            # &codec=265
            # dMod: wcs-25 浏览器解码信息
            # sdkPcdn: 1_1 第一个1连接次数 第二个1是因为什么连接
            # t: 100 平台信息 100 web
            # sv: 2401090219 版本
            # sdk_sid:  _sessionId sdkInRoomTs 当前毫秒时间
            self.room_title = live_info['gameLiveInfo']['introduction']
            self.raw_stream_url = f'{stream_selected["sFlvUrl"]}/{stream_selected["sStreamName"]}.{stream_selected["sFlvUrlSuffix"]}?wsSecret={ws_secret}&wsTime={ws_time}&seqid={seq_id}&ctype={url_query["ctype"][0]}&ver=1&fs={url_query["fs"][0]}&u={convert_uid}&t={platform_id}&sv=2401090219&sdk_sid={int(time.time() * 1000)}&codec=264'
            if record_ratio != max_ratio:
                self.raw_stream_url += f"&ratio={record_ratio}"
            return True
        except:
            logger.warning(f"{Huya.__name__}: {self.url}: 解析错误")

        return False

    def danmaku_download_start(self, filename):
        if self.huya_danmaku:
            self.danmaku = DanmakuClient(self.url, filename + "." + self.suffix)
            self.danmaku.start()

    def close(self):
        if self.danmaku:
            self.danmaku.stop()
