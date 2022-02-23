import time

import youtube_dl
from youtube_dl import MaxDownloadsReached

from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from ..plugins import BatchCheckBase
from . import logger

VALID_URL_BASE = r'https?://(?:(?:www|m)\.)?youtube\.com/(?P<id>.*?)\??(.*?)'

@Plugin.download(regexp=VALID_URL_BASE)
class Youtube(DownloadBase):
    def __init__(self, fname, url, suffix='webm'):
        DownloadBase.__init__(self, fname, url, suffix=suffix)

    def check_stream(self):
        with youtube_dl.YoutubeDL({'download_archive': 'archive.txt', 'ignoreerrors': True, 'extract_flat': True}) as ydl:
            info = ydl.extract_info(self.url, download=False, process=False)
            if info is None:
                logger.warning(self.url)
                return False
            print(info)
            for entry in info['entries']:
                if ydl.in_download_archive(entry):
                    continue
                # ydl.record_download_archive()
                return True
            # for entry in info['entries']:
            #     print(entry)
            #     nest = ydl.extract_info(entry['url'], download=False)
            #     if nest['_type'] == 'playlist':
            #         print('no')
            #         nonono = False
            #         for nest_entry in nest['entries']:
            #             print(nest_entry)
            #             if nest_entry['_type'] == 'url':
            #                 print("ohhhh no")
            #                 nonono = True
            #                 continue
            #         if nonono:
            #             continue
            #     if ydl.in_download_archive(entry):
            #         continue
            #     # self.raw_stream_url = entry['formats'][-1]['url']
            #     # ydl.record_download_archive(entry)
            #     return True

    def download(self, filename):
        try:
            self.ydl_opts = {
                'outtmpl': filename,
                'ignoreerrors': True,
                'max_downloads': 1,
                'download_archive': 'archive.txt'
            }
            with youtube_dl.YoutubeDL(self.ydl_opts) as ydl:
                ydl.download([self.url])
        except MaxDownloadsReached:
            logger.info(f'退出下载: {self.fname}')
            return False
        except youtube_dl.utils.DownloadError:
            logger.exception(self.fname)
            return 1
        return 0

    # class BatchCheck(BatchCheckBase):
    #     def __init__(self, urls):
    #         BatchCheckBase.__init__(self, pattern_id=VALID_URL_BASE, urls=urls)
    #         self.urls = urls
    #
    #     def check(self):
    #         with youtube_dl.YoutubeDL({'download_archive': 'archive.txt', 'ignoreerrors': True, 'extract_flat': True}) as ydl:
    #             for url in self.urls:
    #                 try:
    #                     info = ydl.extract_info(url, download=False, process=False)
    #                 except:
    #                     logger.exception("youtube_dl.utils")
    #                     continue
    #                 for entry in info['entries']:
    #                     if ydl.in_download_archive(entry):
    #                         continue
    #                     yield url
    #                 time.sleep(10)
