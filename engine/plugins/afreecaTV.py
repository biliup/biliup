import requests

from common import logger
from engine.plugins import FFmpegdl, match1

VALID_URL_BASE = r"https?://play\.afreecatv\.com/(?P<username>\w+)(?:/\d+)?"

STREAM_INFO_URLS = "{rmd}/broad_stream_assign.html"
CHANNEL_API_URL = "http://live.afreecatv.com:8057/afreeca/player_live_api.php"

QUALITIES = ["original", "hd", "sd"]


class AfreecaTV(FFmpegdl):
    def __init__(self, fname, url, suffix='mp4'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        logger.debug(self.fname)
        username = match1(self.url, VALID_URL_BASE)
        res_bno = requests.post(CHANNEL_API_URL, data={"bid": username, "mode": "landing", "player_type": "html5"},
                                timeout=5)
        res_bno.close()

        if res_bno.json()["CHANNEL"]["RESULT"] == 0:
            return
        bno = res_bno.json()["CHANNEL"]["BNO"]
        cdn = res_bno.json()["CHANNEL"]["CDN"]
        rmd = res_bno.json()["CHANNEL"]["RMD"]
        res_aid = requests.post(CHANNEL_API_URL, data={
            "bid": username,
            "bno": bno,
            "pwd": "",
            "quality": QUALITIES[0],
            "type": "pwd"
        }, timeout=5)
        res_aid.close()
        aid = res_aid.json()["CHANNEL"]["AID"]
        params = {
            "return_type": cdn,
            "broad_key": "{broadcast}-flash-{quality}-hls".format(broadcast=bno, quality=QUALITIES[0])
        }
        res = requests.get(STREAM_INFO_URLS.format(rmd=rmd), params=params, timeout=5)
        res.close()
        self.ydl_opts['absurl'] = res.json()["view_url"] + "?aid=" + aid
        return True


__plugin__ = AfreecaTV
