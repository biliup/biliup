import time

import youtube_dl
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import BatchCheckBase
from . import logger

VALID_URL_BASE = r'https?://(?:(?:www|m)\.)?youtube\.com/(?P<id>.*?)\??(.*?)'

@Plugin.download(regexp=VALID_URL_BASE)
class Youtube(DownloadBase):
    def __init__(self, fname, url, suffix='mkv'):
        DownloadBase.__init__(self, fname, url, suffix=suffix)

    def check_stream(self):
        with youtube_dl.YoutubeDL({'download_archive': 'archive.txt'}) as ydl:
            info = ydl.extract_info(self.url, download=False)
            for entry in info['entries']:
                if ydl.in_download_archive(entry):
                    continue
                print(entry)
                self.raw_stream_url = entry['formats'][-1]['url']
                ydl.record_download_archive(entry)
                return True

    def download(self, filename):
        try:
            self.ydl_opts = {'outtmpl': filename}
            with youtube_dl.YoutubeDL(self.ydl_opts) as ydl:
                ydl.download([self.url])
        except youtube_dl.utils.DownloadError:
            return 1
        return 0

    class BatchCheck(BatchCheckBase):
        def __init__(self, urls):
            BatchCheckBase.__init__(self, pattern_id=VALID_URL_BASE, urls=urls)
            self.urls = urls

        def check(self):
            with youtube_dl.YoutubeDL({'download_archive': 'archive.txt'}) as ydl:
                for url in self.urls:
                    try:
                        info = ydl.extract_info(url, download=False)
                    except:
                        logger.exception("youtube_dl.utils")
                        continue
                    for entry in info['entries']:
                        if ydl.in_download_archive(entry):
                            continue
                        yield url
                    time.sleep(10)
