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
            self.fake_headers['cookie'] =  config.get('user', {}).get('bili_cookie')
        self.customAPI_use_cookie = config.get('user', {}).get('customAPI_use_cookie', False)
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
        officialApiHost = "https://api.live.bilibili.com"
        protocol = config.get('bili_protocol', 'stream')
        perfCDN = config.get('bili_perfCDN', '')
        forceScoure = config.get('bili_forceScoure', False)
        customApiHost = (lambda a : a if a.startswith(('http://', 'https://')) else 'http://'+a)(config.get('bili_liveapi', officialApiHost).rstrip('/'))
        s = requests.Session()
        s.headers = self.fake_headers.copy()

        with s:
            # 获取直播状态与房间标题
            infoByRoomUrl = f"{officialApiHost}/xlive/web-room/v1/index/getInfoByRoom?room_id={params['room_id']}"
            try:
                roomInfo = s.get(infoByRoomUrl, timeout=5).json()
            except requests.exceptions.ConnectionError as ce:
                logger.error(ce)
                logger.error(f"在连接到 {infoByRoomUrl} 时出现错误")
                return False
            if roomInfo['code'] != 0 or roomInfo['data']['room_info']['live_status'] != 1:
                logger.debug(roomInfo['message'])
                return False
            params['room_id'] = roomInfo['data']['room_info']['room_id']
            self.room_title = roomInfo['data']['room_info']['title']
            # 当 Cookie 存在，并且自定义APi使用Cookie开关关闭时，仅使用官方 Api
            if config.get('user', {}).get('bili_cookie') and self.customAPI_use_cookie is False:
                roomPlayInfoUrl = officialApiHost
            else:
                roomPlayInfoUrl = customApiHost
            roomPlayInfoUrl += '/xlive/web-room/v2/index/getRoomPlayInfo'

            # 尝试获取直播流
            try:
                playInfo = s.get(roomPlayInfoUrl, params=params, timeout=5).json()
            except requests.exceptions.ConnectionError as ce:
                logger.error(ce)
                logger.error(f"{customApiHost}连接失败，尝试回退至官方Api")
                roomPlayInfoUrl = f"{officialApiHost}/xlive/web-room/v2/index/getRoomPlayInfo"
            playInfo = s.get(roomPlayInfoUrl, params=params, timeout=5).json()
            if playInfo['code'] != 0:
                logger.debug(playInfo['message'])
                return False
            streams = playInfo['data']['playurl_info']['playurl']['stream']
            stream = streams[1] if "hls" in protocol else streams[0]
            ### 直播开启后需要约 2Min 缓冲时间以提供 Hevc 编码 与 fmp4 封装，故仅使用 Avc 编码
            stream_info = stream['format'][0]['codec'][0]
            self.raw_stream_url = stream_info['url_info'][0]['host']+stream_info['base_url']+stream_info['url_info'][0]['extra']
            for url_info in stream_info['url_info']:
                # 跳过p2pCDN
                if 'mcdn' in url_info['host']:
                    continue
                # 匹配PerfCDN
                if perfCDN and perfCDN in url_info['extra']:
                    if forceScoure is True and "cn-gotcha01" in perfCDN:
                        stream_info['base_url'] = re.sub(r'\_bluray(?=.*m3u8)', "", stream_info['base_url'])
                    self.raw_stream_url = url_info['host']+stream_info['base_url']+url_info['extra']   
                    logger.debug(f"获取到{url_info['host']}节点,找到了prefCDN")   
                    break
                self.raw_stream_url = url_info['host']+stream_info['base_url']+url_info['extra'] 
            index = 0
            if self.bili_cdn_fallback is True:
            # 检查直播流是否可用
                while s.head(self.raw_stream_url, stream=True).status_code == 404:
                    # 以顺序尝试回退
                    if stream_info['url_info'][index]['host'] in self.raw_stream_url:
                        index+=1
                    try:
                        url_info = stream_info['url_info'][index]
                        self.raw_stream_url = url_info['host']+stream_info['base_url']+url_info['extra']
                    except Exception: # 如之后的节点无可用直播流，回退到 flv 的首个链接
                        logger.debug(f"顺序回退尝试失败，回退到 flv 的首个链接{stream_info['url_info'][0]['host']}尝试下载") 
        return True
