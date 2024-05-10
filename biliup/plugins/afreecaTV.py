import time
from typing import Optional, Dict

import requests

import biliup.common.util
from biliup.config import config
from ..common import tools
from ..common.tools import NamedLock
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import match1, logger

# VALID_URL_BASE = r"https?://(.*?)\.afreecatv\.com/(?P<username>\w+)(?:/\d+)?"
VALID_URL_BASE = r"https?://play\.afreecatv\.com/(?P<username>\w+)(?:/\d+)?"
CHANNEL_API_URL = "https://live.afreecatv.com/afreeca/player_live_api.php"

QUALITIES = ["original", "hd4k", "hd", "sd"]


@Plugin.download(regexp=r"https?://(.*?)\.afreecatv\.com/(?P<username>\w+)(?:/\d+)?")
class AfreecaTV(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        if AfreecaTVUtils.get_cookie():
            self.fake_headers['cookie'] = ';'.join(
                [f"{name}={value}" for name, value in AfreecaTVUtils.get_cookie().items()])

    async def acheck_stream(self, is_check=False):
        try:
            username = match1(self.url, VALID_URL_BASE)
            if not username:
                logger.warning(f"{AfreecaTV.__name__}: {self.url}: 直播间地址错误")
                return False

            channel_info = (await biliup.common.util.client.post(CHANNEL_API_URL, data={
                "bid": username,
                "bno": "",
                "type": "live",
                "pwd": "",
                "player_type": "html5",
                "stream_type": "common",
                "quality": QUALITIES[0],
                "mode": "landing",
                "from_api": 0,
            }, headers=self.fake_headers, timeout=5)).json()

            if channel_info["CHANNEL"]["RESULT"] == -6:
                logger.warning(f"{AfreecaTV.__name__}: {self.url}: 检测失败,请检查账号密码设置")
                return False

            if channel_info["CHANNEL"]["RESULT"] != 1:
                return False

            self.room_title = channel_info["CHANNEL"]["TITLE"]

            if is_check:
                return True

            aid_info = (await biliup.common.util.client.post(CHANNEL_API_URL, data={
                "bid": username,
                "bno": channel_info["CHANNEL"]["BNO"],
                "type": "aid",
                "pwd": "",
                "player_type": "html5",
                "stream_type": "common",
                "quality": QUALITIES[0],
                "mode": "landing",
                "from_api": 0,
            }, headers=self.fake_headers, timeout=5)).json()

            view_info = (await biliup.common.util.client.get(f'{channel_info["CHANNEL"]["RMD"]}/broad_stream_assign.html', params={
                "return_type": channel_info["CHANNEL"]["CDN"],
                "broad_key": f'{channel_info["CHANNEL"]["BNO"]}-common-{QUALITIES[0]}-hls'
            }, headers=self.fake_headers, timeout=5)).json()

            self.raw_stream_url = view_info["view_url"] + "?aid=" + aid_info["CHANNEL"]["AID"]
        except:
            logger.warning(f"{AfreecaTV.__name__}: {self.url}: 获取错误，本次跳过")
            return False

        return True


class AfreecaTVUtils:
    _cookie: Optional[Dict[str, str]] = None
    _cookie_expires = None

    @staticmethod
    def get_cookie() -> Optional[Dict[str, str]]:
        with NamedLock("AfreecaTV_cookie_get"):
            if not AfreecaTVUtils._cookie or AfreecaTVUtils._cookie_expires <= time.time():
                username = config.get('user', {}).get('afreecatv_username', '')
                password = config.get('user', {}).get('afreecatv_password', '')
                if not username or not password:
                    return None
                response = requests.post("https://login.afreecatv.com/app/LoginAction.php", data={
                    "szUid": username,
                    "szPassword": password,
                    "szWork": "login",
                    "szType": "json",
                    "isSaveId": "true",
                    "isSavePw": "true",
                    "isSaveJoin": "true",
                    "isLoginRetain": "Y",
                })
                if response.json()["RESULT"] != 1:
                    return None

                cookie_dict = response.cookies.get_dict()
                AfreecaTVUtils._cookie = {
                    "RDB": cookie_dict["RDB"],
                    "PdboxBbs": cookie_dict["PdboxBbs"],
                    "PdboxTicket": cookie_dict["PdboxTicket"],
                    "PdboxSaveTicket": cookie_dict["PdboxSaveTicket"],
                }
                AfreecaTVUtils._cookie_expires = time.time() + (7 * 24 * 60 * 60)

            return AfreecaTVUtils._cookie
