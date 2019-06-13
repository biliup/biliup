import hashlib
import json
import re
import time
import requests
from engine.plugins import FFmpegdl, match1
from engine.plugins.twitch import headers
from common import logger
from ykdl.common import url_to_module
# 详细api见 https://github.com/zhangn1985/ykdl/blob/master/cykdl/__main__.py


class Douyu(FFmpegdl):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        m, u = url_to_module(self.url)
        parser = m.parser
        info = parser(u)
        stream_id = info.stream_types[0]
        urls = info.streams[stream_id]['src']
        self.ydl_opts['absurl'] = urls[0]
        return True


__plugin__ = Douyu