import requests
import json
from . import logger
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase

@Plugin.download(regexp=r'(?:https?://)?(?:egame\.)?qq\.com')
class egame(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
    
    def check_stream(self):
        if len(self.url.split("egame.qq.com/")) < 2:
            logger.debug("直播间格式错误")
            return False
        rid = self.url.split("egame.qq.com/")[1]
        url = 'https://share.egame.qq.com/cgi-bin/pgg_async_fcgi'
        data = {'param':'''{"0":{"module":"pgg_live_read_svr","method":"get_live_and_profile_info","param":{"anchor_id":''' + str(rid) + ''',"layout_id":"hot","index":1,"other_uid":0}}}'''}
        r = requests.post(url=url,data=data).json()
        if (r['ecode'] != 0):
            logger.debug("直播间地址错误")
            return False
        pid = r['data']["0"]["retBody"]["data"]["video_info"]["pid"]
        if pid == "":
            logger.debug("直播间不存在")
            return False
        is_live = r['data']["0"]["retBody"]["data"]["profile_info"]["is_live"]
        if is_live != 1:
            logger.debug("主播未开播")
            return False
        self.raw_stream_url = r['data']["0"]["retBody"]["data"]["video_info"]["stream_infos"][0]["play_url"]
        return True