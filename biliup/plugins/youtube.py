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
    def __init__(self, fname, url):
        super().__init__(fname, url)
        self.ytb_danmaku = config.get('ytb_danmaku', False)
        self.cookiejarFile = config.get('user', {}).get('youtube_cookie')
        self.vcodec = config.get('youtube_prefer_vcodec')
        self.acodec = config.get('youtube_prefer_acodec')
        self.resolution = config.get('youtube_max_resolution')
        self.filesize = config.get('youtube_max_videosize')
        self.beforedate = config.get('youtube_before_date')
        self.afterdate = config.get('youtube_after_date')
        self.enable_download_live = config.get('youtube_enable_download_live', True)
        self.enable_download_playback = config.get('youtube_enable_download_playback', True)
        # 需要下载的 url
        self.download_url = None

    def check_stream(self, is_check=False):
        with yt_dlp.YoutubeDL({
            'download_archive': 'archive.txt',
            'cookiefile': self.cookiejarFile,
            'ignoreerrors': True,
            'extractor_retries': 0,
        }) as ydl:
            # 获取信息的时候不要过滤
            ydl_archive = copy.deepcopy(ydl.archive)
            ydl.archive = None
            if self.download_url is not None:
                # 直播在重试的时候特别处理
                info = ydl.extract_info(self.download_url, download=False)
            else:
                info = ydl.extract_info(self.url, download=False, process=False)

            if type(info) is not dict:
                logger.warning(f"{Youtube.__name__}: {self.url}: 获取错误")
                return False

            def loop_entries(entrie):
                if type(entrie) is not dict:
                    return None
                elif entrie.get('_type') == 'url':
                    # is_upcoming 等待开播 is_live 直播中 was_live结束直播(回放)
                    if entrie.get('live_status') == 'is_upcoming':
                        return None
                    elif entrie.get('live_status') == 'is_live':
                        # 未开启直播下载忽略
                        if not self.enable_download_live:
                            return None
                    elif entrie.get('live_status') == 'was_live':
                        # 未开启回放下载忽略
                        if not self.enable_download_playback:
                            return None

                    return loop_entries(ydl.extract_info(entrie.get('url'), download=False, process=False))
                elif entrie.get('_type') == 'playlist':
                    # 播放列表递归
                    for e in entrie.get('entries'):
                        le = loop_entries(e)
                        if type(le) is dict:
                            return le
                elif type(entrie) is dict:
                    # 检测是否已下载
                    if ydl._make_archive_id(entrie) in ydl_archive:
                        # 如果已下载但是还在直播则不算下载
                        if entrie.get('live_status') != 'is_live':
                            return None

                    # 检测时间范围
                    if entrie.get('upload_date') not in DateRange(self.afterdate, self.beforedate):
                        return None

                    return entrie
                return None

            download_entry: Optional[dict] = loop_entries(info)
            if type(download_entry) is dict:
                if download_entry.get('live_status') == 'is_live':
                    self.is_download = False
                else:
                    self.is_download = True
                self.room_title = download_entry.get('title')
                self.live_cover_url = download_entry.get('thumbnail')
                self.download_url = download_entry.get('webpage_url')
                return True
            else:
                return False

    def download(self, filename):
        # ydl下载的文件在下载失败时不可控
        # 临时存储在其他地方
        download_dir = f'./cache/temp/youtube/{filename}'
        try:
            ydl_opts = {
                'outtmpl': f'{download_dir}/{filename}.%(ext)s',
                'cookiefile': self.cookiejarFile,
                'break_on_reject': True,
                'download_archive': 'archive.txt',
                'format': 'bestvideo',
                # 'proxy': proxyUrl,
            }

            if self.vcodec is not None:
                ydl_opts['format'] += f"[vcodec~='^({self.vcodec})']"
            if self.filesize is not None and self.is_download:
                # 直播时无需限制文件大小
                ydl_opts['format'] += f"[filesize<{self.filesize}]"
            if self.resolution is not None:
                ydl_opts['format'] += f"[height<={self.resolution}]"
            ydl_opts['format'] += "+bestaudio"
            if self.acodec is not None:
                ydl_opts['format'] += f"[acodec~='^({self.acodec})']"
            # 不能由yt_dlp创建会占用文件夹
            if not os.path.exists(download_dir):
                os.makedirs(download_dir)
            with yt_dlp.YoutubeDL(ydl_opts) as ydl:
                if not self.is_download:
                    # 直播模式不过滤但是能写入过滤
                    ydl.archive = None
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
