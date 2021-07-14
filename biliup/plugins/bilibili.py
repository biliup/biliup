import random

import requests

from . import match1, logger
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m|live)\.)?bilibili\.com')
class Bilibili(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        rid = match1(self.url, r'/(\d+)')
        api1_data = requests.get(f"https://api.live.bilibili.com/room/v1/Room/room_init?id={rid}").json()
        if api1_data['code'] == 0:
            vid = api1_data['data']['room_id']
        else:
            logger.info('Get room ID from API failed: %s', api1_data['msg'])
            vid = rid
        api2_data = requests.get(f"https://api.live.bilibili.com/room/v1/Room/get_info?room_id={vid}").json()
        if api2_data['code'] != 0:
            logger.debug(api2_data['msg'])
            return False
        api2_data = api2_data['data']
        if api2_data['live_status'] != 1:
            return False
        title = api2_data['title']
        api3_data = \
            requests.get(f"https://api.live.bilibili.com/live_user/v1/UserInfo/get_anchor_in_room?roomid={vid}").json()
        if api3_data['code'] == 0:
            artist = api3_data['data']['info']['uname']
            title = '{} - {}'.format(title, artist)
        logger.debug(title)
        params = {
            'https_url_req': 1,
            'cid': vid,
            'platform': 'h5',
            'qn': 10000,
            'ptype': '16'
        }
        data = requests.get("https://api.live.bilibili.com/xlive/web-room/v1/playUrl/playUrl", params=params).json()
        if data['code'] != 0:
            logger.debug(data['msg'])
            return False
        data = data['data']
        qlt = data['current_qn']
        aqlts = {x['qn']: x['desc'] for x in data['quality_description']}
        logger.debug(qlt)
        logger.debug(aqlts)
        self.raw_stream_url = random.choice(data['durl'])['url']
        return True
