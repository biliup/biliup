import requests
import re

from . import match1, logger
from biliup.config import config
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m|live)\.)?bilibili\.com')
class Bilibili(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        # 预读配置
        params = {
            'room_id': match1(self.url, r'/(\d+)'),
            'protocol': '0,1',
            'format': '0,1,2',
            'codec': '0,1',
            'qn': '10000',
            'platform': config.get('biliplatform') if config.get('biliplatform') else "web",
            # 'ptype': '8',
            'dolby': '5',
            'panorama': '1'
        }
        protocol = config.get('bili_protocol') if config.get('bili_protocol') else "stream"
        perfCDN = config.get('bili_perfCDN') if config.get('bili_perfCDN') else ""
        forceScoure = config.get('bili_forceScoure') if config.get('bili_forceScoure') else False
        liveapi = config.get('bili_liveapi').rstrip('/') if config.get('bili_liveapi') else 'https://api.live.bilibili.com'
        self.fake_headers['Referer'] = 'https://live.bilibili.com'

        # 获取直播状态与房间标题
        roomInfo = requests.get(f"https://api.live.bilibili.com/xlive/web-room/v1/index/getInfoByRoom?room_id={params['room_id']}").json()
        if roomInfo['code'] == 0:
            if roomInfo['data']['room_info']['live_status'] != 1:
                return False
            params['room_id'] = roomInfo['data']['room_info']['room_id']
            self.room_title = roomInfo['data']['room_info']['title']
        else:
            logger.debug(roomInfo['message'])
            return False


        playInfo = requests.get(f"{liveapi}/xlive/web-room/v2/index/getRoomPlayInfo", params=params).json()
        if playInfo['code'] != 0:
            logger.debug(playInfo['message'])
            return False
        streams = playInfo['data']['playurl_info']['playurl']['stream']
        stream = streams[1] if "hls" in protocol else streams[0]
        stream_info = stream['format'][0]['codec'][0]
        for url_info in stream_info['url_info']:
            if 'mcdn' in url_info['host']:
                continue
            if perfCDN in url_info['extra']:
                if forceScoure and "cn-gotcha01" in perfCDN:
                    stream_info['base_url'] = re.sub(r'\_bluray(?=.*m3u8)', "", stream_info['base_url'])
                self.raw_stream_url = url_info['host']+stream_info['base_url']+url_info['extra']
                return True
        stream_number = len(stream_info['url_info']) - 1
        self.raw_stream_url = stream_info['url_info'][stream_number]['host']+stream_info['base_url']+stream_info['url_info'][stream_number]['extra']
        return True
