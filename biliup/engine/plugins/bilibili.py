from ykdl.common import url_to_module

from ...common.decorators import Plugin
from .base_adapter import FFmpegdl


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m|live)\.)?bilibili\.com')
class Bilibili(FFmpegdl):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        site, url = url_to_module(self.url)
        try:
            info = site.parser(url)
        except AssertionError:
            return False
        stream_id = info.stream_types[0]
        urls = info.streams[stream_id]['src']
        self.raw_stream_url = urls[0]
        return True
