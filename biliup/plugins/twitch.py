import io
import random
import re
import socket
import subprocess
import time
from typing import AsyncGenerator, List
from urllib.parse import urlencode

import yt_dlp

import biliup.common.util
from biliup.config import config
from biliup.plugins.Danmaku import DanmakuClient
from . import logger
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase, BatchCheck

VALID_URL_BASE = r'(?:https?://)?(?:(?:www|go|m)\.)?twitch\.tv/(?P<id>[0-9_a-zA-Z]+)'
VALID_URL_VIDEOS = r'https?://(?:(?:www|go|m)\.)?twitch\.tv/(?P<id>[^/]+)/(?:videos|profile|clips)'
_CLIENT_ID = 'kimne78kx3ncx6brgo4mv6wki5h1ko'


@Plugin.download(regexp=VALID_URL_VIDEOS)
class TwitchVideos(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        DownloadBase.__init__(self, fname, url, suffix=suffix)
        self.is_download = True
        self.twitch_download_entry = None

    async def acheck_stream(self, is_check=False):
        while True:
            auth_token = TwitchUtils.get_auth_token()
            if auth_token:
                cookie = io.StringIO(f"""# Netscape HTTP Cookie File
.twitch.tv	TRUE	/	FALSE	0	auth-token	{auth_token}
""")
            else:
                cookie = None

            with yt_dlp.YoutubeDL({'download_archive': 'archive.txt', 'cookiefile': cookie}) as ydl:
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
                            self.twitch_download_entry = entry
                        return True
                except Exception as e:
                    if 'Unauthorized' in str(e):
                        TwitchUtils.invalid_auth_token()
                        continue
                    else:
                        logger.warning(f"{self.url}：获取错误", exc_info=True)
                return False

    def download_success_callback(self):
        with yt_dlp.YoutubeDL({'download_archive': 'archive.txt'}) as ydl:
            ydl.record_download_archive(self.twitch_download_entry)


@Plugin.download(regexp=VALID_URL_BASE)
class Twitch(DownloadBase, BatchCheck):
    def __init__(self, fname, url, suffix='flv'):
        DownloadBase.__init__(self, fname, url, suffix=suffix)
        self.twitch_danmaku = config.get('twitch_danmaku', False)
        self.twitch_disable_ads = config.get('twitch_disable_ads', True)
        self.__proc = None

    async def acheck_stream(self, is_check=False):
        channel_name = re.match(VALID_URL_BASE, self.url).group('id').lower()
        user = (await TwitchUtils.post_gql({
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
        })).get('data', {}).get('user')
        if not user:
            logger.warning(f"{Twitch.__name__}: {self.url}: 获取错误", exc_info=True)
            return False
        elif not user['stream'] or user['stream']['type'] != 'live':
            return False

        self.room_title = user['stream']['title']
        self.live_cover_url = user['stream']['previewImageURL']
        if is_check:
            return True

        if self.downloader == 'ffmpeg':
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
                s.bind(('localhost', 0))
                port = s.getsockname()[1]

            stream_shell = [
                "streamlink",
                "--player-external-http",  # 为外部程序提供流媒体数据
                "--player-external-http-port", str(port),  # 对外部输出流的端口
                "--player-external-http-interface", "localhost",
                # "--twitch-disable-ads",                     # 去广告，去掉、跳过嵌入的广告流
                # "--twitch-disable-hosting",               # 该参数从5.0起已被禁用
                "--twitch-disable-reruns",  # 如果该频道正在重放回放，不打开流
                self.url, "best"  # 流链接
            ]
            if self.twitch_disable_ads:  # 去广告，去掉、跳过嵌入的广告流
                stream_shell.insert(1, "--twitch-disable-ads")

            auth_token = TwitchUtils.get_auth_token()
            # 在设置且有效的情况下使用
            if auth_token:
                stream_shell.insert(1, f"--twitch-api-header=Authorization=OAuth {auth_token}")

            self.__proc = subprocess.Popen(stream_shell)
            self.raw_stream_url = f"http://localhost:{port}"
            i = 0
            while i < 5:
                if not (self.__proc.poll() is None):
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
    async def abatch_check(check_urls: List[str]) -> AsyncGenerator[str, None]:
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
        gql = await TwitchUtils.post_gql(ops)
        for index, data in enumerate(gql):
            user = data.get('data', {}).get('user')
            if not user:
                logger.warning(f"{Twitch.__name__}: {check_urls[index]}: 获取错误")
                continue
            elif not user['stream'] or user['stream']['type'] != 'live':
                continue
            yield check_urls[index]

    def danmaku_init(self):
        if self.twitch_danmaku:
            self.danmaku = DanmakuClient(self.url, self.gen_download_filename())

    def close(self):
        try:
            if self.__proc is not None:
                self.__proc.terminate()
                self.__proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.__proc.kill()
        except:
            logger.exception(f'terminate {self.fname} failed')
        finally:
            self.__proc = None


class TwitchUtils:
    # Twitch已失效的auth_token
    _invalid_auth_token = None

    @staticmethod
    def get_auth_token():
        auth_token = config.get('user', {}).get('twitch_cookie')
        if TwitchUtils._invalid_auth_token == auth_token:
            return None
        return auth_token

    @staticmethod
    def invalid_auth_token():
        TwitchUtils._invalid_auth_token = config.get('user', {}).get('twitch_cookie')
        logger.warning("Twitch Cookie已失效请及时更换，后续操作将忽略Twitch Cookie")

    @staticmethod
    async def post_gql(ops):
        headers = {
            'Content-Type': 'text/plain;charset=UTF-8',
            'Client-ID': _CLIENT_ID,
        }
        auth_token = TwitchUtils.get_auth_token()
        if auth_token:
            headers['Authorization'] = f'OAuth {auth_token}'

        gql = await biliup.common.util.client.post(
            'https://gql.twitch.tv/gql',
            json=ops,
            headers=headers,
            timeout=15)
        # gql.close()
        data = gql.json()

        if isinstance(data, dict) and data.get('error') == 'Unauthorized':
            TwitchUtils.invalid_auth_token()
            return await TwitchUtils.post_gql(ops)

        return data
