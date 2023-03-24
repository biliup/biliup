import re
import time
import requests

from biliup.config import config
from . import match1, logger
from biliup.plugins.Danmaku import DanmakuClient
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m|live)\.)?bilibili\.com')
class Bilibili(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.fake_headers['Referer'] = 'https://live.bilibili.com'
        self.use_live_cover = config.get('use_live_cover', False)
        self.bilibili_danmaku = config.get('bilibili_danmaku', False)
        if config.get('user', {}).get('bili_cookie') is not None:
            self.fake_headers['cookie'] = config.get('user', {}).get('bili_cookie')

    def check_stream(self):
        # 预读配置
        params = {
            'room_id': match1(self.url, r'/(\d+)'),
            'protocol': '0,1',# 0: http_stream, 1: http_hls
            'format': '0,1,2',# 0: flv, 1: ts, 2: fmp4
            'codec': '0', # 0: avc, 1: hevc
            'qn': '10000',
            'platform': config.get('biliplatform', 'web'),
            'dolby': '5',
            'panorama': '1'
        }
        protocol = config.get('bili_protocol', 'stream')
        perf_cdn = config.get('bili_perfCDN')
        bili_cdn_fallback = config.get('bili_cdn_fallback', True)
        force_source = config.get('bili_force_source', False)
        ov05_ip = config.get('bili_force_ov05_ip')
        cn01_domains = config.get('bili_force_cn01_domains', '').split(",")
        official_api_host = "https://api.live.bilibili.com"
        with requests.Session() as s:
            s.headers = self.fake_headers.copy()
            # 获取直播状态与房间标题
            info_by_room_url = f"{official_api_host}/xlive/web-room/v1/index/getInfoByRoom?room_id={params['room_id']}"
            try:
                room_info = s.get(info_by_room_url, timeout=3).json()
            except requests.exceptions.ConnectionError:
                logger.error(f"在连接到 {info_by_room_url} 时出现错误")
                return False
            if room_info['code'] != 0 or room_info['data']['room_info']['live_status'] != 1:
                logger.debug(room_info['message'])
                return False
            cover_url = room_info['data']['room_info']['cover']
            live_start_time = room_info['data']['room_info']['live_start_time']
            uname = room_info['data']['anchor_info']['base_info']['uname']
            self.room_title = room_info['data']['room_info']['title']
            if self.use_live_cover is True:  # 获取直播封面并保存到cover目录下
                try:
                    self.live_cover_path = \
                    super().get_live_cover(uname, params['room_id'], self.room_title, live_start_time, cover_url)
                except:
                    logger.error(f"获取直播封面失败")
            # 当 Cookie 存在，并且自定义APi使用Cookie开关关闭时，仅使用官方 Api
            isallow = True if s.headers.get('cookie') is None else config.get('user', {}).get('customAPI_use_cookie', False)
            play_info = get_play_info(s, isallow, official_api_host, params)
        if play_info['code'] != 0:
            logger.debug(play_info['message'])
            return False
        streams = play_info['data']['playurl_info']['playurl']['stream']
        stream = streams[1] if protocol.startswith('hls') else streams[0]
        if protocol == "hls_fmp4":
            if len(stream['format']) > 1:
                stream_info = stream['format'][1]['codec'][0]
            elif int(time.time()) - live_start_time <= 45:  # 等待45s，如果还没有fmp4流就回退到flv流
                return False
            else:
                stream = streams[0]
                stream_info = stream['format'][0]['codec'][0]
                logger.debug(f"获取{uname}房间fmp4流失败，回退到flv流")
        else:
            stream_info = stream['format'][0]['codec'][0]
        stream_url = {'base_url': stream_info['base_url']}
        if perf_cdn is not None:
            for url_info in stream_info['url_info']:
                if perf_cdn in url_info['extra']:
                    stream_url['host'] = url_info['host']
                    stream_url['extra'] = url_info['extra']
                    logger.debug(f"找到了perfCDN{stream_url['host']}")
        if len(stream_url) < 3:
            stream_url['host'] = stream_info['url_info'][-1]['host']
            stream_url['extra'] = stream_info['url_info'][-1]['extra']
        if "cn-gotcha01" in stream_url['extra']:
            # 强制替换cn-gotcha01的节点为指定节点 注意：只有大陆ip才能获取到cn-gotcha01的节点。
            if cn01_domains[0] != '':
                import random
                i = len(cn01_domains)
                while i:  # 测试节点是否可用
                    host = cn01_domains.pop(random.choice(range(i)))
                    i-=1
                    try:
                        if s.get(f"https://{host}{stream_url['base_url']}{stream_url['extra']}",
                                            stream=True).status_code == 200:  # 如果响应状态码是 200，跳出循环
                            stream_url['host'] = "https://" + host
                            logger.debug(f"节点 {host} 可用，替换为该节点")
                            break
                    except requests.exceptions.ConnectionError:  # 如果发生连接异常，继续下一次循环
                        logger.debug(f"节点 {host} 无法访问，尝试下一个节点。")
                        continue
                else:
                    logger.error(f"配置文件中的cn-gotcha01节点均不可用")
            # 强制去除 hls_ts 的 _bluray 文件名
            if force_source:
                stream_url['base_url'] = re.sub(r'_bluray(?=\.m3u8)', "", stream_url['base_url'], 1)
        self.raw_stream_url = stream_url['host'] + stream_url['base_url'] + stream_url['extra']
        # 强制替换ov05 302redirect之后的真实地址为指定的域名或ip达到自选ov05节点的目的
        if ov05_ip and "ov-gotcha05" in stream_url['host']:
            r = s.get(self.raw_stream_url, stream=True)
            self.raw_stream_url = re.sub(r".*(?=/d1--ov-gotcha05)", f"http://{ov05_ip}", r.url, 1)
            logger.debug(f"将ov-gotcha05的节点ip替换为了{ov05_ip}")
        if bili_cdn_fallback:
            try:
                if s.get(self.raw_stream_url, stream=True).status_code == 404:
                    stream_info['url_info'].reverse()
                    for url_info in stream_info['url_info']:
                        self.raw_stream_url = url_info['host'] + stream_info['base_url'] + url_info['extra']
                        if s.get(self.raw_stream_url, stream=True).status_code == 200:
                            break
            except Exception:
                logger.error('')
        return True

    async def danmaku_download_start(self, filename):
        if self.bilibili_danmaku:
            logger.info("开始弹幕录制")
            self.danmaku = DanmakuClient(self.url, filename + "." + self.suffix)
            await self.danmaku.start()

    def close(self):
        if self.bilibili_danmaku:
            self.danmaku.stop()
            logger.info("结束弹幕录制")
            # asyncio.run(self.danmaku.stop())


def get_play_info(s, isallow, official_api_host, params):
    if isallow:
        custom_api_host = \
            (lambda a: a if a.startswith(('http://', 'https://')) else 'http://' + a) \
            (config.get('bili_liveapi', official_api_host).rstrip('/'))
        try:
            return s.get(custom_api_host + '/xlive/web-room/v2/index/getRoomPlayInfo', params=params,
                         timeout=5).json()
        except requests.exceptions.ConnectionError:
            logger.error(f"{custom_api_host}连接失败，尝试回退至官方Api")
    return s.get(official_api_host + '/xlive/web-room/v2/index/getRoomPlayInfo', params=params, timeout=5).json()
