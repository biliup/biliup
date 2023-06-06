import random
import string
import json
import subprocess
import time

import requests
from . import logger
import requests
import json
from biliup.config import config
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from bs4 import BeautifulSoup
from ..plugins import logger


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m|live)\.)?nicovideo\.jp')
class Nico(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        try:
            response = requests.get(self.url)
            soup = BeautifulSoup(response.text, 'html.parser')
            script_tag = soup.find('script', {'type': 'application/ld+json'})
            if script_tag:
                data = script_tag.string
                json_sc = json.loads(data)
                self.room_title = json_sc['name']
        except:
            logger.info("获取标题失败")
        port = random.randint(1025, 65535)
        stream_shell = [
            "streamlink",
            "--player-external-http",  # 为外部程序提供流媒体数据
            "--player-external-http-port", str(port),  # 对外部输出流的端口
            self.url, "best"  # 流链接
        ]
        if config.get('user', {}).get('niconico-email') is not None:
            niconico_email = "--niconico-email " + config.get('user', {}).get('niconico-email')
            stream_shell.insert(1, niconico_email)
        if config.get('user', {}).get('niconico-password') is not None:
            niconico_password = "--niconico-password " + config.get('user', {}).get('niconico-password')
            stream_shell.insert(1, niconico_password)
        if config.get('user', {}).get('niconico-user-session') is not None:
            niconico_user_session = "--niconico-user-session " + config.get('user', {}).get('niconico-user-session')
            stream_shell.insert(1, niconico_user_session)
        if config.get('user', {}).get('niconico-purge-credentials') is not None:
            niconico_purge_credentials = "--niconico-purge-credentials " + config.get('user', {}).get('niconico-purge-credentials')
            stream_shell.insert(1, niconico_purge_credentials)
        self.proc = subprocess.Popen(stream_shell)
        self.raw_stream_url = f"http://localhost:{port}"
        i = 0
        while i < 5:
            if not (self.proc.poll() is None):
                return False
            time.sleep(1)
            i += 1
        return True

    def close(self):
        try:
            if self.proc is not None:
                self.proc.terminate()
        except:
            logger.exception(f'terminate {self.fname} failed')
