import base64
import hashlib
import html
import json
import random
import time
from urllib.parse import parse_qs, unquote

import requests

from biliup.config import config
from biliup.plugins.Danmaku import DanmakuClient
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import match1, logger


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m)\.)?huya\.com')
class Huya(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.huya_danmaku = config.get('huya_danmaku', False)
        self.fake_headers['User-Agent'] = 'Mozilla/5.0 (Linux; Android 10; SM-G981B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/80.0.3987.162 Mobile Safari/537.36'

    def check_stream(self, is_check=False):
        try:
            room_id = self.url.split('huya.com/')[1].split('/')[0].split('?')[0]
        except:
            logger.warning(f"{Huya.__name__}: {self.url}: 直播间地址错误")
            return False
        try:
            res = requests.get(f'https://m.huya.com/{room_id}', timeout=5, headers=self.fake_headers)
            res.close()
        except:
            logger.warning(f"{Huya.__name__}: {self.url}: 获取错误，本次跳过")
            return False

        if '"exceptionType":0' in res.text:
            logger.warning(f"{Huya.__name__}: {self.url}: 直播间地址错误")
            return False

        if '"eLiveStatus":2' not in res.text:
            # 没开播
            return False

        live_info = json.loads(res.text.split('"tLiveInfo":')[1].split(',"_classname":"LiveRoom.LiveInfo"}')[0] + '}')
        if live_info:
            try:
                # 最大录制码率
                huya_max_ratio = config.get('huya_max_ratio', 0)
                # 码率信息
                ratio_items = live_info['tLiveStreamInfo']['vBitRateInfo']['value']
                # 最大码率(不含hdr)
                max_ratio = live_info['iBitRate']
                # 录制码率
                record_ratio = 0
                # 如果限制了最大码率
                if huya_max_ratio != 0:
                    # 挑选合适的码率
                    for ratio_item in ratio_items:
                        # iBitRate = 0的就是最大码率
                        if ratio_item['iBitRate'] == 0:
                            ratio_item['iBitRate'] = max_ratio
                        # 不录制大于最大码率的
                        if huya_max_ratio >= ratio_item['iBitRate'] > record_ratio:
                            record_ratio = ratio_item['iBitRate']

                    # 原画
                    if record_ratio == max_ratio:
                        record_ratio = 0

                # 自选cdn
                huya_cdn = config.get('huyacdn', 'AL')
                # 流信息
                stream_items = live_info['tLiveStreamInfo']['vStreamInfo']['value']
                # 自选的流
                stream_selected = stream_items[0]
                for stream_item in stream_items:
                    if stream_item['sCdnType'] == huya_cdn:
                        stream_selected = stream_item
                        break

                url_query = parse_qs(stream_selected["sFlvAntiCode"])
                uid = random.randint(1400000000000, 1499999999999)
                ws_time = hex(int(time.time() + 21600))[2:]
                seq_id = round(time.time() * 1000) + uid
                ws_secret_prefix = base64.b64decode(unquote(url_query['fm'][0]).encode()).decode().split("_")[0]
                ws_secret_hash = hashlib.md5(
                    f'{seq_id}|{url_query["ctype"][0]}|{url_query["t"][0]}'.encode()).hexdigest()
                ws_secret = hashlib.md5(
                    f'{ws_secret_prefix}_{uid}_{stream_selected["sStreamName"]}_{ws_secret_hash}_{ws_time}'.encode()).hexdigest()

                self.room_title = live_info['sIntroduction']
                self.raw_stream_url = f'{stream_selected["sFlvUrl"]}/{stream_selected["sStreamName"]}.{stream_selected["sFlvUrlSuffix"]}?wsSecret={ws_secret}&wsTime={ws_time}&seqid={seq_id}&ctype={url_query["ctype"][0]}&ver=1&fs={url_query["fs"][0]}&t={url_query["t"][0]}&uid={uid}&ratio={record_ratio}'
                return True
            except:
                logger.warning(f"{Huya.__name__}: {self.url}: 解析错误")

        return False

    async def danmaku_download_start(self, filename):
        if self.huya_danmaku:
            logger.info("开始弹幕录制")
            self.danmaku = DanmakuClient(self.url, filename + "." + self.suffix)
            await self.danmaku.start()

    def close(self):
        if self.huya_danmaku:
            self.danmaku.stop()
            logger.info("结束弹幕录制")
