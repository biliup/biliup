import base64
import html
import json

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
        self.fake_headers['Referer'] = 'https://www.huya.com/'

    def check_stream(self, is_check=False):
        try:
            res = requests.get(self.url, timeout=5, headers=self.fake_headers)
            res.close()
        except:
            logger.warning("虎牙 " + self.url.split("huya.com/")[1] + "：获取错误，本次跳过")
            return False

        huya = None
        if match1(res.text, '"stream": "([a-zA-Z0-9+=/]+)"'):
            huya = base64.b64decode(match1(res.text, '"stream": "([a-zA-Z0-9+=/]+)"')).decode()
        elif match1(res.text, 'stream: ([\w\W]+)'):
            huya = res.text.split('stream: ')[1].split('};')[0].strip()
            if json.loads(huya)['vMultiStreamInfo']:
                huya = res.text.split('stream: ')[1].split('};')[0].strip()
            else:
                huya = None
        if huya:
            try:
                # 自选cdn
                huyacdn = config.get('huyacdn', 'AL')
                # 最大录制码率
                huya_max_ratio = config.get('huya_max_ratio', 0)

                # 流信息
                stream_items = json.loads(huya)['data'][0]['gameStreamInfoList']
                # 码率信息
                ratio_items = json.loads(huya)['vMultiStreamInfo']
                # 最大码率(不含hdr)
                max_ratio = json.loads(huya)['data'][0]['gameLiveInfo']['bitRate']

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

                huyajson = stream_items[0]
                for cdn in stream_items:
                    if cdn['sCdnType'] == huyacdn:
                        huyajson = cdn
                        break

                absurl = f'{huyajson["sFlvUrl"]}/{huyajson["sStreamName"]}.{huyajson["sFlvUrlSuffix"]}?{huyajson["sFlvAntiCode"]}'
                self.raw_stream_url = html.unescape(absurl) + "&ratio=" + str(record_ratio)
                self.room_title = json.loads(huya)['data'][0]['gameLiveInfo']['introduction']
                return True
            except:
                logger.warning("虎牙 " + self.url.split("huya.com/")[1] + "：json解析错误")
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
