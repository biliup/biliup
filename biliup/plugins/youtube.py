import asyncio
import random
import subprocess
import threading
from urllib.parse import urlparse

import yt_dlp
import streamlink
import os
import time
from PIL import Image
from yt_dlp.utils import MaxDownloadsReached

from yt_dlp.utils import DateRange
from biliup.config import config
from biliup.plugins.Danmaku import DanmakuClient
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase, get_valid_filename, stream_gears_download
from . import logger

VALID_URL_BASE = r'https?://(?:(?:www|m)\.)?youtube\.com/(?P<id>.*?)\??(.*?)'

@Plugin.download(regexp=VALID_URL_BASE)
class Youtube(DownloadBase):
    def __init__(self, fname, url, suffix='webm'):
        DownloadBase.__init__(self, fname, url, suffix=suffix)
        self.use_new_ytb_downloader = config.get('use_new_ytb_downloader', False)
        self.ytb_danmaku = config.get('ytb_danmaku', False)
        self.cookiejarFile = config.get('user', {}).get('youtube_cookie')
        self.vcodec = config.get('youtube_prefer_vcodec','av01|vp9|avc')
        self.acodec = config.get('youtube_prefer_acodec','opus|mp4a')
        self.resolution = config.get('youtube_max_resolution','4320')
        self.filesize = config.get('youtube_max_videosize','100G')
        self.beforedate = config.get('youtube_before_date','20770707')
        self.afterdate = config.get('youtube_after_date','19700101')
        self.use_youtube_cover = config.get('use_youtube_cover', False)

    def check_stream(self):
        if self.use_new_ytb_downloader and self.downloader == 'ffmpeg':
            _fname, self.room_title = self.get_stream_info(self.url)
            port = random.randint(1025, 65535)
            stream_shell = [
                "streamlink",
                "--player-external-http",  # 为外部程序提供流媒体数据
                "--player-external-http-port", str(port),  # 对外部输出流的端口
                self.url, "best"  # 流链接
            ]
            self.proc = subprocess.Popen(stream_shell)
            self.raw_stream_url = f"http://localhost:{port}"
            i = 0
            while i < 5:
                if not (self.proc.poll() is None):
                    return
                time.sleep(1)
                i += 1
            return True
        else:
            with yt_dlp.YoutubeDL({'download_archive': 'archive.txt', 'ignoreerrors': True, 'extract_flat': True,
                                   'cookiefile': self.cookiejarFile}) as ydl:
                info = ydl.extract_info(self.url, download=False, process=False)
                if info is None:
                    logger.warning(self.cookiejarFile)
                    logger.warning(self.url)
                    return False
                if '_type' in info:
                    # /live 形式链接取标题
                    if info['_type'] in 'url' and info['webpage_url_basename'] in 'live':
                        live = ydl.extract_info(info['url'], download=False, process=False)
                        self.room_title = live['title']
                    # Playlist 暂时返回列表名
                    if info['_type'] in 'playlist':
                        self.room_title = info['title']
                else:
                    # 视频取标题
                    self.room_title = info['title']
                if info.get('entries') is None:
                    if ydl.in_download_archive(info):
                        return False
                    return True
                for entry in info['entries']:
                    # 取 Playlist 内视频标题
                    self.room_title = entry['title']
                    if ydl.in_download_archive(entry):
                        continue
                    # ydl.record_download_archive()
                    return True

    def download(self, filename):
        if self.use_new_ytb_downloader and self.downloader == 'ffmpeg':
            if self.filename_prefix:  # 判断是否存在自定义录播命名设置
                filename = (self.filename_prefix.format(streamer=self.fname, title=self.room_title).encode(
                    'unicode-escape').decode()).encode().decode("unicode-escape")
            else:
                filename = f'{self.fname}%Y-%m-%dT%H_%M_%S'
            filename = get_valid_filename(filename)
            fmtname = time.strftime(filename.encode("unicode-escape").decode()).encode().decode("unicode-escape")
            threading.Thread(target=asyncio.run, args=(self.danmaku_download_start(fmtname),)).start()
            self.ffmpeg_download(fmtname)
        else:
            try:
                ydl_opts = {
                    'outtmpl': filename+ '.%(ext)s',
                    'format': f"bestvideo[vcodec~='^({self.vcodec})'][height<={self.resolution}][filesize<{self.filesize}]+bestaudio[acodec~='^({self.acodec})']/best[height<={self.resolution}]/best",
                    'cookiefile': self.cookiejarFile,
                    # 'proxy': proxyUrl,
                    'ignoreerrors': True,
                    'max_downloads': 1,
                    'daterange' : DateRange(self.afterdate, self.beforedate),
                    'break_on_reject': True,
                    'download_archive': 'archive.txt',
                    'lazy_playlist': config.get('youtube_lazy_playlist', False)
                }
                if self.use_youtube_cover is True:
                    ydl_opts['writethumbnail'] = True
                with yt_dlp.YoutubeDL(ydl_opts) as ydl:
                    ydl.download([self.url])
            except MaxDownloadsReached:
                save_dir = f'cover/youtube/'
                webp_cover_path = f'{filename}.webp'
                jpg_cover_path = f'{filename}.jpg'
                if os.path.exists(webp_cover_path):
                    with Image.open(webp_cover_path) as img:
                        img = img.convert('RGB')
                        if not os.path.exists(save_dir):
                            os.makedirs(save_dir)
                        img.save(f'{save_dir}{filename}.jpg', format='JPEG')
                    os.remove(webp_cover_path)
                    self.live_cover_path = f'{save_dir}{filename}.jpg'
                elif os.path.exists(jpg_cover_path):
                    os.rename(jpg_cover_path, f'{save_dir}{filename}.jpg')
                    self.live_cover_path = f'{save_dir}{filename}.jpg'
                return False
            except yt_dlp.utils.RejectedVideoReached:
                logger.info("已下载完毕指定日期内的视频，结束本次任务")
                return False
            except yt_dlp.utils.DownloadError:
                logger.exception(self.fname)
                return 1
            return 0

    def get_stream_info(self, url):
        session = streamlink.Streamlink()
        # Streamlink 插件系统的内部运行机制，通过 URL 匹配到 YouTube 插件
        plugin = session.resolve_url(url)[1]
        streams = plugin(session=session, url=url)
        streams._get_streams()
        author = streams.get_author()
        title = streams.get_title()
        return author, title