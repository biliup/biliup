import requests
import os
import re
import random
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
        self.use_live_cover = config.get('use_live_cover', False)

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
        force_ov05_ip = config.get('bili_force_ov05_ip')
        force_cn01_domain = config.get('bili_force_cn01_domains')
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
            if self.use_live_cover is True: # 获取直播封面并保存到bili_cover_cache目录下
                try:
                    room_id = room_info['data']['room_info']['room_id']
                    cover_url = room_info['data']['room_info']['cover']
                    live_start_time = room_info['data']['room_info']['live_start_time']
                    self.live_cover_path = get_live_cover(room_id, live_start_time, cover_url)
                except:
                    logger.error(f"获取直播封面失败")
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
            find = False
            for url_info in stream_info['url_info']:
                # 跳过p2pCDN
                if 'mcdn' in url_info['host']:
                    continue
                # 哔哩哔哩直播强制原画（仅限HLS流的 cn-gotcha01 CDN). 并且仅当主播有二压的时候才自动去掉m3u8的_bluray前缀，避免stream-gears的疯狂分段bug
                if force_source is True and "cn-gotcha01" in url_info['extra'] and "_bluray" in stream_info['base_url']:
                    stream_info['base_url'] = re.sub(r'_bluray(?=.*m3u8)', "", stream_info['base_url'])
                    find = True
                # 强制替换hls流的cn-gotcha01的节点为指定节点 注意：只有大陆ip才能获取到cn-gotcha01的节点。
                if force_cn01_domain and "cn-gotcha01" in perf_cdn and protocol == "hls":
                    i = 0
                    while i < 6:  # 测试随机到的节点是否可用
                        random_choice = random.choice(force_cn01_domain.split(","))
                        i += 1
                        try:  # 发起 HEAD 请求，并获取响应状态码
                            status_code = s.head(f"https://{random_choice}{stream_info['base_url']}{url_info['extra']}",
                                                 stream=True).status_code
                            if status_code == 200:  # 如果响应状态码是 200，跳出循环
                                break
                        except requests.exceptions.ConnectionError:  # 如果发生连接异常，继续下一次循环
                            logger.debug(f"随机到的域名 {random_choice} 无法访问，尝试重新发起随机。")
                            continue
                    else:
                        logger.error(f"强制替换hls流的cn-gotcha01的节点为指定节点失败啦")
                        return False
                    logger.debug(f"随机到的域名 {random_choice} 返回了 200 状态码。")
                    url_info['host'] = "https://" + random_choice
                    find = True
                # 强制替换ov05 302redirect之后的真实地址为指定的域名或ip达到自选ov05节点的目的
                if force_ov05_ip and "ov-gotcha05" in url_info['host']:
                    response = requests.get(url_info['host'] + stream_info['base_url'] + url_info['extra'])
                    self.raw_stream_url = re.sub(r"https://([a-z0-9]+)\.ourdvsss\.com", f"http://{force_ov05_ip}",
                                                 response.url)
                    logger.debug(f"将ov-gotcha05的节点ip替换为了{force_ov05_ip}")
                    break
                # 匹配PerfCDN
                if perf_cdn and perf_cdn in url_info['extra']:
                    find = True
                    logger.debug(f"获取到{url_info['host']}节点,找到了prefCDN")
                self.raw_stream_url = url_info['host'] + stream_info['base_url'] + url_info['extra']
                if find:
                    break
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

def get_live_cover(room_id, live_start_time, cover_url):
    headers = {
        "origin": "https://www.bilibili.com",
        "user-agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/110.0.0.0 Safari/537.36 Edg/110.0.1587.41"
    }
    response = requests.get(cover_url, headers=headers, timeout=5)
    save_dir = f'bili_cover_cache/'
    if not os.path.exists(save_dir):
        os.makedirs(save_dir)
    live_cover_path = f'{save_dir}{room_id}_{live_start_time}.png'
    if os.path.exists(live_cover_path):
        return live_cover_path
    else:
        with open(live_cover_path, 'wb') as f:
            f.write(response.content)       
            return live_cover_path 
