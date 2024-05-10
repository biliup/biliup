import copy
import os
import shutil
from typing import Optional

import yt_dlp

from yt_dlp import DownloadError
from yt_dlp.utils import DateRange
from biliup.config import config
from ..engine.decorators import Plugin
from . import logger
from ..engine.download import DownloadBase

VALID_URL_BASE = r'https?://(?:(?:www|m)\.)?youtube\.com/(?P<id>.*?)\??(.*?)'


@Plugin.download(regexp=VALID_URL_BASE)
class Youtube(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.ytb_danmaku = config.get('ytb_danmaku', False)
        self.youtube_cookie = config.get('user', {}).get('youtube_cookie')
        self.youtube_prefer_vcodec = config.get('youtube_prefer_vcodec')
        self.youtube_prefer_acodec = config.get('youtube_prefer_acodec')
        self.youtube_max_resolution = config.get('youtube_max_resolution')
        self.youtube_max_videosize = config.get('youtube_max_videosize')
        self.youtube_before_date = config.get('youtube_before_date')
        self.youtube_after_date = config.get('youtube_after_date')
        self.youtube_enable_download_live = config.get('youtube_enable_download_live', True)
        self.youtube_enable_download_playback = config.get('youtube_enable_download_playback', True)
        # 需要下载的 url
        self.download_url = None

    async def acheck_stream(self, is_check=False):
        with yt_dlp.YoutubeDL({
            'download_archive': 'archive.txt',
            'cookiefile': self.youtube_cookie,
            'ignoreerrors': True,
            'extractor_retries': 0,
        }) as ydl:
            # 获取信息的时候不要过滤
            ydl_archive = copy.deepcopy(ydl.archive)
            ydl.archive = set()
            if self.download_url is not None:
                # 直播在重试的时候特别处理
                info = ydl.extract_info(self.download_url, download=False)
            else:
                info = ydl.extract_info(self.url, download=False, process=False)
            if type(info) is not dict:
                logger.warning(f"{Youtube.__name__}: {self.url}: 获取错误")
                return False

            cache = KVFileStore(f"./cache/youtube/{self.fname}.txt")

            def loop_entries(entrie):
                if type(entrie) is not dict:
                    return None
                elif entrie.get('_type') == 'playlist':
                    # 播放列表递归
                    for e in entrie.get('entries'):
                        le = loop_entries(e)
                        if type(le) is dict:
                            return le
                        elif le == "stop":
                            return None
                elif type(entrie) is dict:
                    # is_upcoming 等待开播 is_live 直播中 was_live结束直播(回放)
                    if entrie.get('live_status') == 'is_upcoming':
                        return None
                    elif entrie.get('live_status') == 'is_live':
                        # 未开启直播下载忽略
                        if not self.youtube_enable_download_live:
                            return None
                    elif entrie.get('live_status') == 'was_live':
                        # 未开启回放下载忽略
                        if not self.youtube_enable_download_playback:
                            return None

                    # 检测是否已下载
                    if ydl._make_archive_id(entrie) in ydl_archive:
                        # 如果已下载但是还在直播则不算下载
                        if entrie.get('live_status') != 'is_live':
                            return None

                    upload_date = cache.query(entrie.get('id'))
                    if upload_date is None:
                        if entrie.get('upload_date') is not None:
                            upload_date = entrie['upload_date']
                        else:
                            entrie = ydl.extract_info(entrie.get('url'), download=False, process=False)
                            if type(entrie) is dict and entrie.get('upload_date') is not None:
                                upload_date = entrie['upload_date']

                        # 时间是必然存在的如果不存在说明出了问题 暂时跳过
                        if upload_date is None:
                            return None
                        else:
                            cache.add(entrie.get('id'), upload_date)

                    if self.youtube_after_date is not None and upload_date < self.youtube_after_date:
                        return 'stop'

                    # 检测时间范围
                    if upload_date not in DateRange(self.youtube_after_date, self.youtube_before_date):
                        return None

                    return entrie
                return None

            download_entry: Optional[dict] = loop_entries(info)
            if type(download_entry) is dict:
                if download_entry.get('live_status') == 'is_live':
                    self.is_download = False
                else:
                    self.is_download = True
                if not is_check:
                    if download_entry.get('_type') == 'url':
                        download_entry = ydl.extract_info(download_entry.get('url'), download=False, process=False)
                    self.room_title = download_entry.get('title')
                    self.live_cover_url = download_entry.get('thumbnail')
                    self.download_url = download_entry.get('webpage_url')
                return True
            else:
                return False

    def download(self):
        filename = self.gen_download_filename(is_fmt=True)
        # ydl下载的文件在下载失败时不可控
        # 临时存储在其他地方
        download_dir = f'./cache/temp/youtube/{filename}'
        try:
            ydl_opts = {
                'outtmpl': f'{download_dir}/{filename}.%(ext)s',
                'cookiefile': self.youtube_cookie,
                'break_on_reject': True,
                'download_archive': 'archive.txt',
                'format': 'bestvideo',
                # 'proxy': proxyUrl,
            }

            if self.youtube_prefer_vcodec is not None:
                ydl_opts['format'] += f"[vcodec~='^({self.youtube_prefer_vcodec})']"
            if self.youtube_max_videosize is not None and self.is_download:
                # 直播时无需限制文件大小
                ydl_opts['format'] += f"[filesize<{self.youtube_max_videosize}]"
            if self.youtube_max_resolution is not None:
                ydl_opts['format'] += f"[height<={self.youtube_max_resolution}]"
            ydl_opts['format'] += "+bestaudio"
            if self.youtube_prefer_acodec is not None:
                ydl_opts['format'] += f"[acodec~='^({self.youtube_prefer_acodec})']"
            # 不能由yt_dlp创建会占用文件夹
            if not os.path.exists(download_dir):
                os.makedirs(download_dir)
            with yt_dlp.YoutubeDL(ydl_opts) as ydl:
                if not self.is_download:
                    # 直播模式不过滤但是能写入过滤
                    ydl.archive = set()
                ydl.download([self.download_url])
            # 下载成功的情况下移动到运行目录
            for file in os.listdir(download_dir):
                shutil.move(f'{download_dir}/{file}', '.')
        except DownloadError as e:
            if 'Requested format is not available' in e.msg:
                logger.error(f"{Youtube.__name__}: {self.url}: 无法获取到流，请检查vcodec,acodec,height,filesize设置")
            elif 'ffmpeg is not installed' in e.msg:
                logger.error(f"{Youtube.__name__}: {self.url}: ffmpeg未安装，无法下载")
            else:
                logger.error(f"{Youtube.__name__}: {self.url}: {e.msg}")
            return False
        finally:
            # 清理意外退出可能产生的多余文件
            try:
                del ydl
                shutil.rmtree(download_dir)
            except:
                logger.error(f"{Youtube.__name__}: {self.url}: 清理残留文件失败，请手动删除{download_dir}")
        return True


class KVFileStore:
    def __init__(self, file_path):
        self.file_path = file_path
        self.cache = {}
        self._preload_data()

    def _ensure_file_and_folder_exists(self):
        folder_path = os.path.dirname(self.file_path)
        # 如果文件夹不存在，则创建文件夹
        if not os.path.exists(folder_path):
            os.makedirs(folder_path)
        # 如果文件不存在，则创建空文件
        if not os.path.exists(self.file_path):
            open(self.file_path, "w").close()

    def _preload_data(self):
        self._ensure_file_and_folder_exists()
        with open(self.file_path, "r", encoding="utf-8") as f:
            for line in f:
                k, v = line.strip().split("=")
                self.cache[k] = v

    def add(self, key, value):
        with open(self.file_path, "a", encoding="utf-8") as f:
            f.write(f"{key}={value}\n")
        # 更新缓存
        self.cache[key] = value

    def query(self, key, default=None):
        if key in self.cache:
            return self.cache[key]
        return default
