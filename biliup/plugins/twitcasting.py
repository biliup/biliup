import requests
import time
import streamlink
import re

from biliup.config import config
from ..engine.decorators import Plugin
from . import logger
from ..engine.download import DownloadBase

VALID_URL_BASE = r"https?://twitcasting\.tv/(?P<channel>[^/]+)"
VALID_URL_VIDEOS = r'https?://twitcasting\.tv/(?P<id>[0-9_:a-zA-Z]+)/(?:archive)'

# @Plugin.download(regexp=VALID_URL_VIDEOS)
# class TwitcastingVideos(DownloadBase):
#     def __init__(self, fname, url, suffix='mp4'):
#         pass

#     def check_stream(self, is_check=False):
#         pass

@Plugin.download(regexp=VALID_URL_BASE)
class Twitcasting(DownloadBase):

    def __init__(self, fname, url, suffix='flv'):
        DownloadBase.__init__(self, fname, url, suffix=suffix)
        # self.twitcasting_danmaku = config.get('twitcasting_danmaku', False)
        self.fake_headers = {
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:122.0) Gecko/20100101 Firefox/122.0",
            "Accept-Encoding": "gzip, deflate, br",
            "Accept": "*/*",
            "Connection": "keep-alive",
            # "Referer": "https://twitcasting.tv/",
            # "Origin": "https://twitcasting.tv",
        }

    def check_stream(self, is_check=False):
        # with requests.Session() as s:
        #     s.headers = self.fake_headers
        #     self.fake_headers['Cookie'] = s.get(self.url).headers['Set-Cookie'].split(';')[0]

        #     channel = re.match(VALID_URL_BASE, self.url).group('channel')
        #     params = {"__n": int(time.time() * 1000)}
        #     is_on_live = s.get(f"https://frontendapi.twitcasting.tv/users/{channel}/latest-movie", params=params)
        #     print(is_on_live)
        #     webSessionId = 'credentials'
        streams = streamlink.streams(self.url)
        if len(streams) > 0:
            return True
        return False

    def download(self, filename):
        filename = self.get_filename()
        fmtname = time.strftime(filename.encode("unicode-escape").decode()).encode().decode("unicode-escape")

        # self.danmaku_download_start(fmtname)

