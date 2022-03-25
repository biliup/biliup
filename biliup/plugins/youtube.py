import yt_dlp
from yt_dlp.utils import MaxDownloadsReached

from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from . import logger

VALID_URL_BASE = r'https?://(?:(?:www|m)\.)?youtube\.com/(?P<id>.*?)\??(.*?)'

@Plugin.download(regexp=VALID_URL_BASE)
class Youtube(DownloadBase):
    def __init__(self, fname, url, suffix='webm'):
        DownloadBase.__init__(self, fname, url, suffix=suffix)

    def check_stream(self):
        with yt_dlp.YoutubeDL({'download_archive': 'archive.txt', 'ignoreerrors': True, 'extract_flat': True}) as ydl:
            info = ydl.extract_info(self.url, download=False, process=False)
            if info is None:
                logger.warning(self.url)
                return False
            if info.get('entries') is None:
                if ydl.in_download_archive(info):
                    return False
                return True
            for entry in info['entries']:
                if ydl.in_download_archive(entry):
                    continue
                # ydl.record_download_archive()
                return True

    def download(self, filename):
        try:
            self.ydl_opts = {
                'outtmpl': filename,
                'ignoreerrors': True,
                'max_downloads': 1,
                'download_archive': 'archive.txt'
            }
            with yt_dlp.YoutubeDL(self.ydl_opts) as ydl:
                ydl.download([self.url])
        except MaxDownloadsReached:
            return False
        except yt_dlp.utils.DownloadError:
            logger.exception(self.fname)
            return 1
        return 0
