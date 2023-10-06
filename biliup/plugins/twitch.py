import random
import re
import subprocess
import time
from typing import Generator, List
from urllib.parse import urlencode

import requests
import yt_dlp

from . import logger
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from biliup.config import config
from biliup.plugins.Danmaku import DanmakuClient

VALID_URL_BASE = r'(?:https?://)?(?:(?:www|go|m)\.)?twitch\.tv/(?P<id>[0-9_a-zA-Z]+)'
VALID_URL_VIDEOS = r'https?://(?:(?:www|go|m)\.)?twitch\.tv/(?P<id>[^/]+)/(?:videos|profile|clips)'
_CLIENT_ID = 'kimne78kx3ncx6brgo4mv6wki5h1ko'

# Twitch 授权信息是否到期
AUTH_EXPIRE_STATUS = False


@Plugin.download(regexp=VALID_URL_VIDEOS)
class TwitchVideos(DownloadBase):
    def __init__(self, fname, url, suffix='mp4'):
        DownloadBase.__init__(self, fname, url, suffix=suffix)
        self.is_download = True

    def check_stream(self, is_check=False):
        # TODO 这里原本的批量检测是有问题的 先用yt_dlp实现 等待后续新增新的批量检测方式 后续这里的auth信息和直播一样采用twitch_cookie
        with yt_dlp.YoutubeDL({'download_archive': 'archive.txt'}) as ydl:
            try:
                info = ydl.extract_info(self.url, download=False, process=False)
                for entry in info['entries']:
                    if ydl.in_download_archive(entry):
                        continue
                    if not is_check:
                        download_info = ydl.extract_info(entry['url'], download=False)
                        self.room_title = download_info['title']
                        self.raw_stream_url = download_info['url']
                        thumbnails = download_info.get('thumbnails')
                        if type(thumbnails) is list and len(thumbnails) > 0:
                            self.live_cover_url = thumbnails[len(thumbnails) - 1].get('url')
                        ydl.record_download_archive(entry)
                    return True
            except:
                logger.warning(f"{self.url}：获取错误")
                return False
        return False



@Plugin.download(regexp=VALID_URL_BASE)
class Twitch(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        DownloadBase.__init__(self, fname, url, suffix=suffix)
        self.twitch_danmaku = config.get('twitch_danmaku', False)
        self.twitch_disable_ads = config.get('twitch_disable_ads', True)
        self.proc = None

    def check_stream(self, is_check=False):
        channel_name = re.match(VALID_URL_BASE, self.url).group('id').lower()
        user = post_gql({
            "query": '''
                query query($channel_name:String!) {
                    user(login: $channel_name){
                        stream {
                            id
                            title
                            type
                            previewImageURL(width: 0,height: 0)
                            playbackAccessToken(
                                params: {
                                    platform: "web",
                                    playerBackend: "mediaplayer",
                                    playerType: "site"
                                }
                            ) {
                                signature
                                value             
                            }
                        }
                    }
                }
            ''',
            'variables': {'channel_name': channel_name}
        }).get('data', {}).get('user')
        if not user:
            logger.warning(f"{Twitch.__name__}: {self.url}: 获取错误")
            return False
        elif not user['stream'] or user['stream']['type'] != 'live':
            return False

        self.room_title = user['stream']['title']
        self.live_cover_url = user['stream']['previewImageURL']
        if is_check:
            return True

        if self.downloader == 'ffmpeg':
            port = random.randint(1025, 65535)
            stream_shell = [
                "streamlink",
                "--player-external-http",  # 为外部程序提供流媒体数据
                # "--twitch-disable-ads",                     # 去广告，去掉、跳过嵌入的广告流
                # "--twitch-disable-hosting",               # 该参数从5.0起已被禁用
                "--twitch-disable-reruns",  # 如果该频道正在重放回放，不打开流
                "--player-external-http-port", str(port),  # 对外部输出流的端口
                self.url, "best"  # 流链接
            ]
            if self.twitch_disable_ads:  # 去广告，去掉、跳过嵌入的广告流
                stream_shell.insert(1, "--twitch-disable-ads")

            twitch_cookie = config.get('user', {}).get('twitch_cookie')
            # 在设置且有效的情况下使用
            if twitch_cookie and not AUTH_EXPIRE_STATUS:
                stream_shell.insert(1, "--twitch-api-header=Authorization=OAuth " + twitch_cookie)

            self.proc = subprocess.Popen(stream_shell)
            self.raw_stream_url = f"http://localhost:{port}"
            i = 0
            while i < 5:
                if not (self.proc.poll() is None):
                    return False
                time.sleep(1)
                i += 1
            return True
        else:
            query = {
                "player": "twitchweb",
                "p": random.randint(1000000, 10000000),
                "allow_source": "true",
                "allow_audio_only": "true",
                "allow_spectre": "false",
                'fast_bread': "true",
                'sig': user.get('stream').get('playbackAccessToken').get('signature'),
                'token': user.get('stream').get('playbackAccessToken').get('value'),
            }
            self.raw_stream_url = f'https://usher.ttvnw.net/api/channel/hls/{channel_name}.m3u8?{urlencode(query)}'
            return True

    @staticmethod
    def batch_check(check_urls: List[str]) -> Generator[str, None, None]:
        ops = []
        for url in check_urls:
            channel_name = re.match(VALID_URL_BASE, url).group('id')
            op = {
                "query": '''
                    query query($login:String!) {
                        user(login: $login){
                            stream {
                              type
                            }
                        }
                    }
                ''',
                'variables': {'login': channel_name.lower()}
            }
            ops.append(op)
        gql = post_gql(ops)
        for index, data in enumerate(gql):
            user = data.get('data', {}).get('user')
            if not user:
                logger.warning(f"{Twitch.__name__}: {check_urls[index]}: 获取错误")
                continue
            elif not user['stream'] or user['stream']['type'] != 'live':
                continue
            yield check_urls[index]

    def danmaku_download_start(self, filename):
        if self.twitch_danmaku:
            self.danmaku = DanmakuClient(self.url, filename + "." + self.suffix)
            self.danmaku.start()

    def close(self):
        if self.danmaku:
            self.danmaku.stop()
        try:
            if self.proc is not None:
                self.proc.terminate()
        except:
            logger.exception(f'terminate {self.fname} failed')


def post_gql(ops):
    global AUTH_EXPIRE_STATUS
    headers = {
        'Content-Type': 'text/plain;charset=UTF-8',
        'Client-ID': _CLIENT_ID,
    }
    twitch_cookie = config.get('user', {}).get('twitch_cookie')
    if not AUTH_EXPIRE_STATUS and twitch_cookie:
        headers['Authorization'] = f'OAuth {twitch_cookie}'

    gql = requests.post(
        'https://gql.twitch.tv/gql',
        json=ops,
        headers=headers,
        timeout=15)
    gql.close()
    data = gql.json()

    if isinstance(data, dict) and data.get('error') == 'Unauthorized':
        AUTH_EXPIRE_STATUS = True
        logger.warning("Twitch Cookie已失效请及时更换，之后操作将忽略Twitch Cookie")
        return post_gql(ops)

    return data
