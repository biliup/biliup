from common.decorators import Plugin
from engine.plugins import logger
from engine.plugins.base_adapter import FFmpegdl
from ykdl.common import url_to_module


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m)\.)?douyu\.com')
class Douyu(FFmpegdl):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        logger.debug(self.fname)
        site, url = url_to_module(self.url)
        try:
            info = site.parser(url)
        except AssertionError:
            return
        stream_id = info.stream_types[0]
        urls = info.streams[stream_id]['src']
        self.raw_stream_url = urls[0]
        # print(info.title)
        return True
