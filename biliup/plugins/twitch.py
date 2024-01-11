import os
import re
import shutil
import yt_dlp
import requests

from . import logger
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from biliup.config import config

VALID_URL_VIDEOS = r'https?://(?:(?:www|go|m)\.)?twitch\.tv/(?P<id>[^/]+)/(?:videos|profile|clips)'
_CLIENT_ID = 'kimne78kx3ncx6brgo4mv6wki5h1ko'
AUTH_EXPIRE_STATUS = False

@Plugin.download(regexp=VALID_URL_VIDEOS)
class TwitchVideos(DownloadBase):
    def __init__(self, fname, url, suffix='mp4'):
        DownloadBase.__init__(self, fname, url, suffix=suffix)
        self.is_download = True

    def check_stream(self, is_check=False):
        if self._is_live():
            logger.warning(f"{self.url}：主播正在直播，停止下载回放")
            return False

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
            except Exception as e:
                logger.warning(f"{self.url}：获取错误 - {e}")
                return False
        return False

    def download(self, filename):
        download_dir = './downloads'
        ydl_opts = {
            'outtmpl': os.path.join(download_dir, f'{filename}.%(ext)s'),
            'format': 'bestvideo+bestaudio/best',
        }

        if not os.path.exists(download_dir):
            os.makedirs(download_dir)

        with yt_dlp.YoutubeDL(ydl_opts) as ydl:
            ydl.download([self.raw_stream_url])

        for file in os.listdir(download_dir):
            shutil.move(os.path.join(download_dir, file), './')

    def _is_live(self):
        channel_name = re.match(VALID_URL_VIDEOS, self.url).group('id').lower()
        response = post_gql({
            "query": '''
                query query($channel_name:String!) {
                    user(login: $channel_name){
                        stream {
                            type
                        }
                    }
                }
            ''',
            'variables': {'channel_name': channel_name}
        })
        user = response.get('data', {}).get('user')
        return user and user['stream'] and user['stream']['type'] == 'live'

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
