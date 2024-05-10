from threading import Event
from ykdl.common import url_to_module
import yt_dlp

from ..engine.download import DownloadBase
from . import logger


class YDownload(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.ydl_opts = {}

    async def acheck_stream(self, is_check=False):
        try:
            self.get_sinfo()
            return True
        except yt_dlp.utils.DownloadError:
            logger.debug('%s未开播或读取下载信息失败' % self.fname)
            return False

    def get_sinfo(self):
        info_list = []
        with yt_dlp.YoutubeDL() as ydl:
            if self.url:
                info = ydl.extract_info(self.url, download=False)
            else:
                logger.debug('%s不存在' % self.__class__.__name__)
                return
            for i in info['formats']:
                info_list.append(i['format_id'])
            logger.debug(info_list)
        return info_list

    def download(self):
        try:
            filename = self.gen_download_filename(is_fmt=True)
            self.ydl_opts = {'outtmpl': filename}
            with yt_dlp.YoutubeDL(self.ydl_opts) as ydl:
                ydl.download([self.url])
        except yt_dlp.utils.DownloadError:
            return 1
        return 0


class SDownload(DownloadBase):
    def __init__(self, fname, url, suffix='mp4'):
        super().__init__(fname, url, suffix)
        self.stream = None
        self.flag = Event()

    async def acheck_stream(self, is_check=False):
        logger.debug(self.fname)
        import streamlink
        try:
            streams = streamlink.streams(self.url)
            if streams:
                self.stream = streams["best"]
                fd = self.stream.open()
                fd.close()
                # streams.close()
                return True
        except streamlink.StreamlinkError:
            return

    def download(self):
        filename = self.gen_download_filename(is_fmt=True)
        # fd = stream.open()
        try:
            with self.stream.open() as fd:
                with open(filename + '.part', 'wb') as file:
                    for f in fd:
                        file.write(f)
                        if self.flag.is_set():
                            # self.flag.clear()
                            return 1
                    return 0
        except OSError:
            self.download_file_rename(filename + '.part', filename)
            raise


class Generic(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.handler = self

    async def acheck_stream(self, is_check=False):
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

    def download(self):
        if self.handler == self:
            return super(Generic, self).download()
        return self.handler.download()


__plugin__ = Generic
