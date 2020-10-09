from engine.plugins import logger
from engine.plugins.base_adapter import YDownload, SDownload, FFmpegdl
from ykdl.common import url_to_module


class Handler:
    def __init__(self, successor=None):
        self.successor = successor

    def handle(self):
        """
        Handle request and stop.
        If can't - call next handler in chain.
        As an alternative you might even in case of success
        call the next handler.
        """
        if not self.check_stream() and self.successor:
            return self.successor.handle()
        return self

    def check_stream(self):
        pass


class FallbackHandler(Handler):
    def check_stream(self):
        return False

    def __eq__(self, other):
        return self.__class__.__name__ == other.__class__.__name__


class StreamLink(SDownload, Handler):
    def __init__(self, fname, url, successor=None):
        super().__init__(fname, url, suffix='flv')
        self.successor = successor


class YoutubeDl(YDownload, Handler):
    def __init__(self, fname, url, successor=None):
        super().__init__(fname, url, suffix='mp4')
        self.successor = successor


class Generic(FFmpegdl):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.handler = self

    def check_stream(self):
        logger.debug(self.fname)
        streamlink = StreamLink(self.fname, self.url, FallbackHandler())
        youtube_dl = YoutubeDl(self.fname, self.url, streamlink)
        try:
            site, url = url_to_module(self.url)
            info = site.parser(url)
        # except AssertionError:
        #     self.handler = youtube_dl.handle()
        #     return self.handler != FallbackHandler()

            stream_id = info.stream_types[0]
            urls = info.streams[stream_id]['src']
            self.raw_stream_url = urls[0]
        # print(info.title)
        except:
            self.handler = youtube_dl.handle()
            self.suffix = self.handler.suffix
            return self.handler != FallbackHandler()
        return True

    def download(self, filename):
        if self.handler == self:
            return super(Generic, self).download(filename)
        return self.handler.download(filename)


__plugin__ = Generic
