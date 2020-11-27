from ykdl.common import url_to_module

from engine.plugins import logger
from engine.plugins.base_adapter import YDownload, SDownload, FFmpegdl


class Generic(FFmpegdl):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.handler = self

    def check_stream(self):
        logger.debug(self.fname)
        try:
            site, url = url_to_module(self.url)
            info = site.parser(url)
            stream_id = info.stream_types[0]
            urls = info.streams[stream_id]['src']
            self.raw_stream_url = urls[0]
        # print(info.title)
        except:
            handlers = [YDownload(self.fname, self.url, 'mp4'), SDownload(self.fname, self.url, 'flv')]
            for handler in handlers:
                if handler.check_stream():
                    self.handler = handler
                    self.suffix = handler.suffix
                    return True
            return False
        return True

    def download(self, filename):
        if self.handler == self:
            return super(Generic, self).download(filename)
        return self.handler.download(filename)


__plugin__ = Generic
