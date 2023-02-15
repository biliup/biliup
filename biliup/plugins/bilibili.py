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
        self.fake_headers['Referer'] = 'https://live.bilibili.com'
        if config.get('user', {}).get('bili_cookie'):
            self.fake_headers['cookie'] = config.get('user', {}).get('bili_cookie')
        self.customAPI_use_cookie = config.get('user', {}).get('customAPI_use_cookie')
        self.bili_cdn_fallback = config.get('bili_cdn_fallback', True)

    def check_stream(self):
        # 预读配置
        params = {
            'room_id': match1(self.url, r'/(\d+)'),
            'protocol': '0,1',
            'format': '0,1,2',
            'codec': '0,1',
            'qn': '10000',
            'platform': config.get('biliplatform', 'web'),
            # 'ptype': '8',
            'dolby': '5',
            'panorama': '1'
        }
        protocol = config.get('bili_protocol', 'stream')
        perf_cdn = config.get('bili_perfCDN')
        force_source = config.get('bili_force_source')
        official_api_host = "https://api.live.bilibili.com"

        with requests.Session() as s:
            s.headers = self.fake_headers.copy()
            # 获取直播状态与房间标题
            info_by_room_url = f"{official_api_host}/xlive/web-room/v1/index/getInfoByRoom?room_id={params['room_id']}"
            try:
                room_info = s.get(info_by_room_url, timeout=5).json()
            except requests.exceptions.ConnectionError:
                logger.error(f"在连接到 {info_by_room_url} 时出现错误")
                return False
            if room_info['code'] != 0 or room_info['data']['room_info']['live_status'] != 1:
                logger.debug(room_info['message'])
                return False
            params['room_id'] = room_info['data']['room_info']['room_id']
            self.room_title = room_info['data']['room_info']['title']
            custom_api = config.get('bili_liveapi') is not None
            # 当 Cookie 存在，并且自定义APi使用Cookie开关关闭时，仅使用官方 Api
            if config.get('user', {}).get('bili_cookie') and self.customAPI_use_cookie is not True:
                custom_api = False
            play_info = get_play_info(s, custom_api, official_api_host, params)
            if play_info['code'] != 0:
                logger.debug(play_info['message'])
                return False
            streams = play_info['data']['playurl_info']['playurl']['stream']
            stream = streams[1] if "hls" in protocol else streams[0]
            # 直播开启后需要约 2Min 缓冲时间以提供 Hevc 编码 与 fmp4 封装，故仅使用 Avc 编码
            stream_info = stream['format'][0]['codec'][0]
            self.raw_stream_url = stream_info['url_info'][0]['host'] + stream_info['base_url'] \
                                  + stream_info['url_info'][0]['extra']
            for url_info in stream_info['url_info']:
                # 跳过p2pCDN
                if 'mcdn' in url_info['host']:
                    continue
                # 匹配PerfCDN
                if perf_cdn and perf_cdn in url_info['extra']:
                    if force_source is True and "cn-gotcha01" in perf_cdn:
                        stream_info['base_url'] = re.sub(r'_bluray(?=.*m3u8)', "", stream_info['base_url'])
                    self.raw_stream_url = url_info['host'] + stream_info['base_url'] + url_info['extra']
                    logger.debug(f"获取到{url_info['host']}节点,找到了prefCDN")
                    break
                self.raw_stream_url = url_info['host'] + stream_info['base_url'] + url_info['extra']
            if self.bili_cdn_fallback is False:
                return True
            stream_info['url_info'].reverse()
            # 检查直播流是否可用以倒叙尝试回退
            for stream_url in stream_info['url_info']:
                self.raw_stream_url = stream_url['host'] + stream_info['base_url'] + stream_url['extra']
                if s.get(self.raw_stream_url, stream=True).status_code == 404:
                    continue
                break
        return True


def get_play_info(s, custom_api, official_api_host, params):
    if custom_api:
        custom_api_host = \
            (lambda a: a if a.startswith(('http://', 'https://')) else 'http://' + a)(config.get('bili_liveapi')
                                                                                      .rstrip('/'))
        # 尝试获取直播流
        try:
            return s.get(custom_api_host + '/xlive/web-room/v2/index/getRoomPlayInfo', params=params,
                         timeout=5).json()
        except requests.exceptions.ConnectionError:
            logger.error(f"{custom_api_host}连接失败，尝试回退至官方Api")
    return s.get(official_api_host + '/xlive/web-room/v2/index/getRoomPlayInfo', params=params, timeout=5).json()
