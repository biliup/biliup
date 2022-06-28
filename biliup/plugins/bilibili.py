import requests

from . import match1, logger
from biliup.config import config
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m|live)\.)?bilibili\.com')
class Bilibili(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        rid = match1(self.url, r'/(\d+)')
        room_info = requests.get(f"https://api.live.bilibili.com/xlive/web-room/v1/index/getInfoByRoom?room_id={rid}").json()
        if room_info['code'] == 0:
            vid = room_info['data']['room_info']['room_id']
        else:
            logger.debug(room_info['message'])
            return False
        if room_info['data']['room_info']['live_status'] != 1:
            return False
        self.room_title = room_info['data']['room_info']['title']
        biliplatform = config.get('biliplatform') if config.get('biliplatform') else 'web'
        params = {
            'room_id': vid,
            'qn': '10000',
            'platform': biliplatform,
            'codec': '0,1',
            'protocol': '0,1',
            'format': '0,1,2',
            'ptype': '8',
            'dolby': '5'
        }
        data = requests.get("https://api.live.bilibili.com/xlive/web-room/v2/index/getRoomPlayInfo", params=params).json()
        if data['code'] != 0:
            logger.debug(data['msg'])
            return False
        data = data['data']['playurl_info']['playurl']['stream'][0]['format'][0]['codec'][0]
        stream_number = 0
        if "mcdn" in data['url_info'][0]['host']:
            stream_number += 1
        self.raw_stream_url = data['url_info'][stream_number]['host'] + data['base_url'] + data['url_info'][stream_number]['extra']
        self.fake_headers['Referer'] = 'https://live.bilibili.com'
        return True
