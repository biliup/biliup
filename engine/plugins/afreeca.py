import requests
from common import logger
import re

from engine.plugins.base_adapter import FFmpegdl

QUALITYS = ["original", "hd", "sd"]
QUALITY_WEIGHTS = {
    "original": 1080,
    "hd": 720,
    "sd": 480
}

STREAM_INFO_URLS = "{rmd}/broad_stream_assign.html"

VALID_URL_BASE = r"https?://play\.afreecatv\.com/(?P<username>\w+)(?:/\d+)?"

CHANNEL_API_URL = "http://live.afreecatv.com:8057/afreeca/player_live_api.php"

class Afreeca(FFmpegdl):
    def __init__(self, fname, url, suffix='mp4'):
        super().__init__(fname, url, suffix)
        _url_re = re.compile(VALID_URL_BASE)
        self.username = _url_re.match(url).group("username")

    def check_stream(self):
        logger.debug(self.fname)
        resBNO = requests.post(CHANNEL_API_URL,
            data={"bid": self.username, "mode": "landing", "player_type": "html5"}, timeout=5)
        resBNO.close()
        # print("IS ONLINE = ",resBNO.json()["CHANNEL"]["RESULT"])

        if resBNO.json()["CHANNEL"]["RESULT"] == 0:
            return False
        BNO = resBNO.json()["CHANNEL"]["BNO"]
        CDN = resBNO.json()["CHANNEL"]["CDN"]
        RMD = resBNO.json()["CHANNEL"]["RMD"]
        data2 = {
            "bid": self.username,
            "bno": BNO,
            "pwd": "",
            "quality": QUALITYS[0],
            "type": "pwd"
        }
        resAID = requests.post(CHANNEL_API_URL,data=data2,timeout=5)
        resAID.close()
        AID = resAID.json()["CHANNEL"]["AID"]
        # print("BNO=",BNO,"CDN=",CDN,"RMD=",RMD,"AID=",AID)
        params = {
            "return_type": CDN,
            "broad_key": "{broadcast}-flash-{quality}-hls".format(broadcast=BNO,quality=QUALITYS[0])
        }
        res = requests.get(STREAM_INFO_URLS.format(rmd = RMD),params=params,timeout=5)
        res.close()
        # print("URL=",res.json()["view_url"]+"?aid="+AID)
        self.ydl_opts['absurl'] =res.json()["view_url"]+"?aid="+AID
        return True
__plugin__ = Afreeca
