import requests
from urllib.parse import urlparse, parse_qs
import json
import time

from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import logger, match1, random_user_agent
from biliup.config import config
from .Danmaku import DanmakuClient

VALID_URL_BASE = r"https?://twitcasting\.tv/([^/]+)"

@Plugin.download(regexp=VALID_URL_BASE)
class Twitcasting(DownloadBase):
    def __init__(self, fname, url, suffix='ts'):
        super().__init__(fname, url, suffix)
        # 指定 streamlink 下载器，因为提供的链接是其他下载器不支持的 wss://
        self.downloader = "streamlink"
        self.twitcasting_danmaku = config.get('twitcasting_danmaku', True)
        self.fake_headers = {
            "Accept": "*/*",
            "Accept-Encoding": "gzip, deflate, br",
            "Cache-Control": "no-cache",
            "Pragma": "no-cache",
            "Referer": "https://twitcasting.tv/",
            "User-Agent": random_user_agent()
        }

    def check_stream(self, is_check=False):
        with requests.Session() as s:
            s.headers.update(self.fake_headers.copy())
            response = s.get(self.url, timeout=5)
            if response.status_code != 200:
                logger.warning(f"{Twitcasting.__name__}: {self.url}: 获取错误，本次跳过")
                return False
            boardcasterInfo = TwitcastingUtils._getBroadcaster(response.text)

            '''
            X-Web-Authorizekey 可在 PlayerPage2.js 文件中
            通过 return ""[u(413)](m, ".")[u(413)](f) 所在的方法计算而出
            由 salt + 10位 timestamp + 接口Method大写 + 接口pathname + 接口search + web-authorize-session-id 拼接后
            再经过 SHA-256 处理，最后在字符串前面拼接上 10位 timestamp 和 dot 得到
            '''
            __n = int(time.time() * 1000)
            _salt = "d6g97jormun44naq"
            _time = str(__n)[:10]
            _method = "GET"
            _pathname = f"/users/{boardcasterInfo['ID']}/latest-movie"
            _search = "?__n=" + str(__n)

            s.headers.update({
                "X-Web-Authorizekey": TwitcastingUtils._generate_authorizekey(
                    _salt,
                    _time,
                    _method,
                    _pathname,
                    _search,
                    boardcasterInfo['web-authorize-session-id']
                ),
                "X-Web-Sessionid": boardcasterInfo['web-authorize-session-id'],
            })
            params = {"__n": __n}
            r = s.get(f"https://frontendapi.twitcasting.tv{_pathname}", params=params, timeout=5).json()
            if not r['movie']['is_on_live']:
                return False

            if boardcasterInfo['ID']:
                params = {
                    "mode": "client",
                    "target": boardcasterInfo['ID']
                }
                _stream_info = s.get("https://twitcasting.tv/streamserver.php", params=params, timeout=5).json()
                if not _stream_info['movie']['live']:
                    return False
                if is_check:
                    return True
        self.room_title = boardcasterInfo['Title']
        self.raw_stream_url = self.url
        return True

    def danmaku_download_start(self, filename):
        if self.twitcasting_danmaku:
            self.danmaku = DanmakuClient(self.url, filename + "." + self.suffix)
            self.danmaku.start()

    def close(self):
        if self.danmaku:
            self.danmaku.stop()

class TwitcastingUtils:
    import hashlib

    def _getBroadcaster(html_text: str) -> dict:
        _info = {}
        _info['ID'] = match1(html_text, VALID_URL_BASE)
        _info['Title'] = match1(
            html_text,
            r'<meta name="twitter:title" content="([^"]*)"'
        )
        _info['MovieID'] = match1(
            match1(
                html_text,
                r'<meta name="twitter:image" content="([^"]*)"'
            ),
            r'/(\d+)'
        )
        _info['web-authorize-session-id'] = json.loads(
            match1(
                html_text,
                r'<meta name="tc-page-variables" content="([^"]+)"'
            ).replace(
                '&quot;',
                '"'
            )
        ).get('web-authorize-session-id')
        return _info

    def _generate_authorizekey(salt: str, timestamp: str, method: str, pathname: str, search: str, sessionid: str) -> str:
        _hash_str = salt + timestamp + method + pathname + search + sessionid
        return str(timestamp + "." + TwitcastingUtils.hashlib.sha256(_hash_str.encode()).hexdigest())