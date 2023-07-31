import asyncio
import random
import shutil
import subprocess
import threading

import yaml
import yt_dlp
import streamlink
import os
import time

from yt_dlp.utils import DateRange, UserNotLive
from biliup.config import config
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase, get_valid_filename
from . import logger

VALID_URL_BASE = r'https?://(?:(?:www|m)\.)?youtube\.com/(?P<id>.*?)\??(.*?)'


@Plugin.download(regexp=VALID_URL_BASE)
class Youtube(DownloadBase):
    def __init__(self, fname, url, suffix='webm'):
        DownloadBase.__init__(self, fname, url, suffix=suffix)
        self.use_new_ytb_downloader = config.get('use_new_ytb_downloader', False)
        self.ytb_danmaku = config.get('ytb_danmaku', False)
        self.cookiejarFile = config.get('user', {}).get('youtube_cookie')
        self.vcodec = config.get('youtube_prefer_vcodec', 'av01|vp9|avc')
        self.acodec = config.get('youtube_prefer_acodec', 'opus|mp4a')
        self.resolution = config.get('youtube_max_resolution', '4320')
        self.filesize = config.get('youtube_max_videosize', '100G')
        self.beforedate = config.get('youtube_before_date', '20770707')
        self.afterdate = config.get('youtube_after_date', '19700101')
        # 需要下载的url
        self.download_url = None

    def check_stream(self, is_check=False):
        self.download_url = None
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
                    return False
                time.sleep(1)
                i += 1
            return True
        else:
            with yt_dlp.YoutubeDL({'download_archive': 'archive.txt', 'cookiefile': self.cookiejarFile,
                                   'format': f"bestvideo[vcodec~='^({self.vcodec})'][height<={self.resolution}][filesize<{self.filesize}]+bestaudio[acodec~='^({self.acodec})']/best[height<={self.resolution}]/best",
                                   }) as ydl:
                try:
                    info = ydl.extract_info(self.url, download=False, process=False)
                    if type(info) is not dict:
                        logger.warning(f"{self.url}：获取错误")
                        return False
                except UserNotLive:
                    logger.warning(f"{self.url}：地址填写错误")
                    return False
                except:
                    logger.warning(f"{self.url}：获取错误")
                    return False

                if info.get('entries') is None:
                    if ydl.in_download_archive(info):
                        return False
                    # 视频取标题
                    self.room_title = info.get('title')
                    self.live_cover_url = info.get('thumbnail')
                    self.download_url = self.url
                    return True

                # 时间范围缓存避免每次都要读取
                cache_save_dir = './cache'
                cache_filename = f'{cache_save_dir}/yt_dlp_cache.yaml'
                if not os.path.exists(cache_save_dir):
                    os.makedirs(cache_save_dir)
                if not os.path.exists(cache_filename):
                    with open(cache_filename, 'w') as f:
                        f.close()

                cache = {}
                try:
                    with open(cache_filename, 'r', encoding='utf-8') as file:
                        cache = yaml.load(file, Loader=yaml.FullLoader)
                        if type(cache) is not dict:
                            cache = {}
                except:
                    logger.exception(f'读取{Youtube.__class__}缓存失败，将重置缓存')

                self.is_download = True

                for entry in info['entries']:
                    # 检测是否已下载
                    if ydl.in_download_archive(entry):
                        continue

                    try:
                        # 检测时间范围
                        if entry['id'] not in cache:
                            cache[entry['id']] = {}
                        upload_date = cache.get(entry['id']).get('upload_date')
                        if upload_date is None:
                            upload_date = ydl.extract_info(entry['url'], download=False).get('upload_date')

                        cache[entry['id']]['upload_date'] = upload_date
                        if upload_date not in DateRange(self.afterdate, self.beforedate):
                            continue
                    except:
                        continue

                    # 取 Playlist 内视频标题
                    self.room_title = entry['title']
                    thumbnails = entry.get('thumbnails', [])
                    if type(thumbnails) is list and len(thumbnails) > 0:
                        self.live_cover_url = thumbnails[len(thumbnails) - 1].get('url')
                    self.download_url = entry['url']
                    break

                with open(cache_filename, "w", encoding='utf-8') as f:
                    yaml.dump(cache, f, encoding='utf-8', allow_unicode=True)

                return self.download_url is not None

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
            # ydl下载的文件在下载失败时不可控
            # 临时存储在其他地方
            if self.is_download:
                download_dir = f'./cache/youtube/{filename}'
            else:
                download_dir = '.'
            try:
                ydl_opts = {
                    'outtmpl': f'{download_dir}/{filename}.%(ext)s',
                    'format': f"bestvideo[vcodec~='^({self.vcodec})'][height<={self.resolution}][filesize<{self.filesize}]+bestaudio[acodec~='^({self.acodec})']/best[height<={self.resolution}]/best",
                    'cookiefile': self.cookiejarFile,
                    # 'proxy': proxyUrl,
                    'break_on_reject': True,
                }
                if self.is_download:
                    # 只有下载时记录已下载
                    ydl_opts['download_archive'] = 'archive.txt'

                with yt_dlp.YoutubeDL(ydl_opts) as ydl:
                    ydl.download([self.download_url])

                if self.is_download:
                    for file in os.listdir(download_dir):
                        shutil.move(f'{download_dir}/{file}', './')
            except:
                logger.exception(self.fname)
                return False
            finally:
                # 删除引用 让gc回收ydl释放文件句柄
                del ydl
                if self.is_download:
                    # 清理意外退出可能产生的多余文件
                    shutil.rmtree(download_dir)
            return True

    def get_stream_info(self, url):
        session = streamlink.Streamlink()
        # Streamlink 插件系统的内部运行机制，通过 URL 匹配到 YouTube 插件
        plugin = session.resolve_url(url)[1]
        streams = plugin(session=session, url=url)
        streams._get_streams()
        author = streams.get_author()
        title = streams.get_title()
        return author, title
