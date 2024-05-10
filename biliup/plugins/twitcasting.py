import hashlib

import biliup.common.util
from biliup.config import config
from .Danmaku import DanmakuClient
from ..common import tools
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import logger, match1

VALID_URL_BASE = r"https?://twitcasting\.tv/([^/]+)"


@Plugin.download(regexp=VALID_URL_BASE)
class Twitcasting(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.twitcasting_danmaku = config.get('twitcasting_danmaku', True)
        self.twitcasting_password = config.get('twitcasting_password', '')
        self.fake_headers['referer'] = "https://twitcasting.tv/"
        if self.twitcasting_password:
            self.fake_headers[
                'cookie'] = f"wpass={hashlib.md5(self.twitcasting_password.encode(encoding='UTF-8')).hexdigest()}"
        # TODO 传递过于繁琐
        self.movie_id = None

    async def acheck_stream(self, is_check=False):
        # with requests.Session() as s:
        biliup.common.util.client.headers = self.fake_headers

        uploader_id = match1(self.url, r'twitcasting.tv/([^/?]+)')
        response = await biliup.common.util.client.get(f'https://twitcasting.tv/streamserver.php?target={uploader_id}&mode=client&player=pc_web',
                                                       timeout=5)
        if response.status_code != 200:
            logger.warning(f"{Twitcasting.__name__}: {self.url}: 获取错误，本次跳过")
            return False
        room_info = response.json()
        if not room_info:
            logger.warning(f"{Twitcasting.__name__}: {self.url}: 直播间地址错误")
            return False
        if not room_info['movie']['live']:
            logger.debug(f"{Twitcasting.__name__}: {self.url}: 未开播")
            return False

        self.movie_id = room_info['movie']['id']

        room_html = (await biliup.common.util.client.get(f'https://twitcasting.tv/{uploader_id}', timeout=5)).text
        if 'Enter the secret word to access' in room_html:
            logger.warning(f"{Twitcasting.__name__}: {self.url}: 直播间需要密码")
            return False
        self.room_title = match1(room_html, r'<meta name="twitter:title" content="([^"]*)"')
        # 尺寸不合适
        # self.live_cover_url = match1(room_html, r'<meta property="og:image" content="([^"]*)"')
        self.raw_stream_url = f"https://twitcasting.tv/{uploader_id}/metastream.m3u8?mode=source"
        return True

    def danmaku_init(self):
        if self.twitcasting_danmaku:
            self.danmaku = DanmakuClient(self.url, self.gen_download_filename(), {
                'movie_id': self.movie_id,
                'password': self.twitcasting_password,
            })

#
# class TwitcastingUtils:
#     import hashlib
#
#     def _getBroadcaster(html_text: str) -> dict:
#         _info = {}
#         _info['ID'] = match1(html_text, VALID_URL_BASE)
#         _info['Title'] = match1(
#             html_text,
#             r'<meta name="twitter:title" content="([^"]*)"'
#         )
#         _info['MovieID'] = match1(
#             match1(
#                 html_text,
#                 r'<meta name="twitter:image" content="([^"]*)"'
#             ),
#             r'/(\d+)'
#         )
#         _info['web-authorize-session-id'] = json.loads(
#             match1(
#                 html_text,
#                 r'<meta name="tc-page-variables" content="([^"]+)"'
#             ).replace(
#                 '&quot;',
#                 '"'
#             )
#         ).get('web-authorize-session-id')
#         return _info
#
#     def _generate_authorizekey(salt: str, timestamp: str, method: str, pathname: str, search: str,
#                                sessionid: str) -> str:
#         _hash_str = salt + timestamp + method + pathname + search + sessionid
#         return str(timestamp + "." + TwitcastingUtils.hashlib.sha256(_hash_str.encode()).hexdigest())
# '''
# X-Web-Authorizekey 可在 PlayerPage2.js 文件中
# 通过 return ""[u(413)](m, ".")[u(413)](f) 所在的方法计算而出
# 由 salt + 10位 timestamp + 接口Method大写 + 接口pathname + 接口search + web-authorize-session-id 拼接后
# 再经过 SHA-256 处理，最后在字符串前面拼接上 10位 timestamp 和 dot 得到
# '''
# __n = int(time.time() * 1000)
# _salt = "d6g97jormun44naq"
# _time = str(__n)[:10]
# _method = "GET"
# _pathname = f"/users/{boardcasterInfo['ID']}/latest-movie"
# _search = "?__n=" + str(__n)
#
# s.headers.update({
#     "X-Web-Authorizekey": TwitcastingUtils._generate_authorizekey(
#         _salt,
#         _time,
#         _method,
#         _pathname,
#         _search,
#         boardcasterInfo['web-authorize-session-id']
#     ),
#     "X-Web-Sessionid": boardcasterInfo['web-authorize-session-id'],
# })
# params = {"__n": __n}
# r = s.get(f"https://frontendapi.twitcasting.tv{_pathname}", params=params, timeout=5).json()
# if not r['movie']['is_on_live']:
#     return False
#
# if boardcasterInfo['ID']:
#     params = {
#         "mode": "client",
#         "target": boardcasterInfo['ID']
#     }
#     _stream_info = s.get("https://twitcasting.tv/streamserver.php", params=params, timeout=5).json()
#     if not _stream_info['movie']['live']:
#         return False
#     if is_check:
#         return True
