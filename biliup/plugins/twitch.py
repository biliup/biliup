import os
import re
import yt_dlp
import requests
import ffmpeg  # 新增导入ffmpeg库  pip install ffmpeg-python

from . import logger
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase
from biliup.config import config

VALID_URL_VIDEOS = r'https?://(?:(?:www|go|m)\.)?twitch\.tv/(?P<id>[^/]+)/(?:videos|profile|clips)'
_CLIENT_ID = 'kimne78kx3ncx6brgo4mv6wki5h1ko'
AUTH_EXPIRE_STATUS = False

@Plugin.download(regexp=VALID_URL_VIDEOS)
class TwitchVideos(DownloadBase):
    def __init__(self, fname, url, suffix='mp4'):
        super().__init__(fname, url, suffix=suffix)
        self.is_download = True

    def check_stream(self, is_check=False):
        if self._is_live():
            logger.warning(f"{self.url}：主播正在直播，停止下载回放")
            return False

        with yt_dlp.YoutubeDL({'download_archive': 'archive.txt'}) as ydl:
            try:
                info = ydl.extract_info(self.url, download=False, process=False)
                for entry in info['entries']:
                    if ydl.in_download_archive(entry):
                        continue
                    if not is_check:
                        download_info = ydl.extract_info(entry['url'], download=False)
                        self.room_title = download_info['title']
                        self.raw_stream_url = download_info['url']
                        thumbnails = download_info.get('thumbnails')
                        if thumbnails and len(thumbnails) > 0:
                            self.live_cover_url = thumbnails[-1].get('url')
                        ydl.record_download_archive(entry)
                    return True
            except Exception as e:
                logger.warning(f"{self.url}：获取错误 - {e}")
                return False
        return False

    def download(self, filename):
        download_dir = './downloads'
        ydl_opts = {
            'outtmpl': os.path.join(download_dir, f'{filename}.%(ext)s'),
            'format': 'bestvideo+bestaudio/best',
        }

        if not os.path.exists(download_dir):
            os.makedirs(download_dir)

        with yt_dlp.YoutubeDL(ydl_opts) as ydl:
            result = ydl.extract_info(self.raw_stream_url, download=True)
            if 'entries' in result:
                video = result['entries'][0]
            else:
                video = result

            if video:
                downloaded_file_path = ydl.prepare_filename(video)
                if os.path.exists(downloaded_file_path):
                    duration = self._get_video_duration(downloaded_file_path)
                    if duration > 36000:  # 10小时
                        self._split_video(downloaded_file_path, 9 * 3600 + 55 * 60)  # 9小时55分钟
                    else:
                        self._move_file(downloaded_file_path, './')

    def _get_video_duration(self, filepath):
        """获取视频时长（秒）"""
        try:
            probe = ffmpeg.probe(filepath)
            duration = float(probe['format']['duration'])
            return duration
        except ffmpeg.Error as e:
            logger.warning(f'{self.url}：获取视频时长失败: {e}')
            return 0

    def _split_video(self, filepath, segment_duration):
        """使用ffmpeg分割视频"""
        filename, ext = os.path.splitext(filepath)
        segment_filename = f'{filename}_%03d{ext}'
        
        # 保存原始文件名的基础部分
        self.base_filename = os.path.basename(filename)
        self.suffix = ext

        ffmpeg.input(filepath).output(segment_filename, f='segment', segment_time=segment_duration, reset_timestamps=1, c='copy').run()
        logger.warning(f"{self.url}：分段完成: {filepath}")

        os.remove(filepath)  # 删除原文件
        logger.warning(f"{self.url}：原文件已删除: {filepath}")

        # 获取分段后的文件列表并移动它们
        segment_files = self._get_segment_files()
        for seg_file in segment_files:
            self._move_file(seg_file, './')

    def _get_segment_files(self):
        """获取分段后的文件列表"""
        segment_files = []
        search_dir = './downloads'
        logger.warning(f"{self.url}：正在搜索目录: {search_dir}")

        for file in os.listdir(search_dir):
            full_path = os.path.join(search_dir, file)
            if file.startswith(self.base_filename) and file.endswith(self.suffix):
                segment_files.append(full_path)
                logger.warning(f"{self.url}：找到分段文件: {full_path}")

        if not segment_files:
            logger.warning("{self.url}：未找到分段文件，请确认分段文件的命名格式和存储位置。")

        return segment_files

    def _move_file(self, src, dst):
        """移动文件"""
        try:
            # 构建完整的目标文件路径
            dst_path = os.path.join(dst, os.path.basename(src))
            logger.warning(f"{self.url}：正在移动文件: 从 {src} 到 {dst_path}")
            os.rename(src, dst_path)
            logger.warning(f"{self.url}：文件移动成功: {src} 到 {dst_path}")
        except Exception as e:
            logger.warning(f"{self.url}：文件移动失败: {e}")

    def _is_live(self):
        channel_name = re.match(VALID_URL_VIDEOS, self.url).group('id').lower()
        response = post_gql({
            "query": '''
                query query($channel_name:String!) {
                    user(login: $channel_name){
                        stream {
                            type
                        }
                    }
                }
            ''',
            'variables': {'channel_name': channel_name}
        })
        user = response.get('data',{}).get('user')
        return user and user['stream'] and user['stream']['type'] == 'live'

def post_gql(ops):
    global AUTH_EXPIRE_STATUS
    headers = {
        'Content-Type': 'text/plain;charset=UTF-8',
        'Client-ID': _CLIENT_ID,
    }
    twitch_cookie = config.get('user', {}).get('twitch_cookie')
    if not AUTH_EXPIRE_STATUS and twitch_cookie:
        headers['Authorization'] = f'OAuth {twitch_cookie}'

    gql = requests.post(
        'https://gql.twitch.tv/gql',
        json=ops,
        headers=headers,
        timeout=15)
    gql.close()
    data = gql.json()

    if isinstance(data, dict) and data.get('error') == 'Unauthorized':
        AUTH_EXPIRE_STATUS = True
        logger.warning("Twitch Cookie已失效请及时更换，之后操作将忽略Twitch Cookie")
        return post_gql(ops)

    return data

