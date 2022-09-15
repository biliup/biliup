import json
import urllib.request

import requests
import re
from . import logger
from biliup.config import config
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m|live)\.)?douyin\.com')
class Douyin(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        headers = {
            "user-agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) "
                          "Chrome/94.0.4606.71 Safari/537.36 Edg/94.0.992.38",
            "referer": "https://live.douyin.com/",
            "cookie": config.get('douyin_cookie')
        }
        if len(self.url.split("live.douyin.com/")) < 2:
            if len(self.url.split("douyin.com/user/")) < 2:
                logger.debug("直播间地址错误")
                return False 
            else:
                mainPage=requests.get(self.url, headers=headers).text\
                .split('<script id="RENDER_DATA" type="application/json">')[1].split('</script>')[0]
                txt = urllib.request.unquote(mainPage)
                rex = re.compile(r'(?<=\"web_rid\":\")[0-9]*(?=\")')
                rid = rex.findall(txt)[0]    
        else:
            rid = self.url.split("live.douyin.com/")[1]
        r1 = requests.get('https://live.douyin.com/' + rid, headers=headers).text \
            .split('<script id="RENDER_DATA" type="application/json">')[1].split('</script>')[0]
        r2 = urllib.request.unquote(r1)
        room_info = json.loads(r2)['app']['initialState']['roomStore']['roomInfo']['room']
        if room_info.get('status') != 2:
            logger.debug("主播未开播")
            return False
        if room_info.get('stream_url'):
            r5 = room_info['stream_url']['live_core_sdk_data']['pull_data']['stream_data']
            self.raw_stream_url = json.loads(r5)['data']['origin']['main']['flv']
            self.room_title = room_info['title']
            return True
