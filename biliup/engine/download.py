import asyncio
import logging
import os
import re
import subprocess
import sys
import threading
import time
from urllib.parse import urlparse

import stream_gears

from biliup.config import config

logger = logging.getLogger('biliup')


class DownloadBase:
    def __init__(self, fname, url, suffix=None, opt_args=None):
        self.danmaku = None
        self.room_title = None
        if opt_args is None:
            opt_args = []
            # 主播单独传参覆盖全局设置。例如新增了一个全局的filename_prefix参数，在下面添加self.filename_prefix = config.get('filename_prefix'),
            # 即可通过self.filename_prefix在下载或者上传时候传递主播单独的设置参数用于调用（如果该主播有设置单独参数，将会优先使用单独参数；如无，则会优先你用全局参数。）
        self.fname = fname
        self.url = url
        self.suffix = suffix
        self.title = None
        self.live_cover_path = None
        self.downloader = config.get('downloader', 'stream-gears')
        # ffmpeg.exe -i  http://vfile1.grtn.cn/2018/1542/0254/3368/154202543368.ssm/154202543368.m3u8
        # -c copy -bsf:a aac_adtstoasc -movflags +faststart output.mp4
        self.raw_stream_url = None
        self.filename_prefix = config.get('filename_prefix')
        self.use_live_cover = config.get('use_live_cover')
        self.opt_args = opt_args
        self.fake_headers = {
            'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
            'Accept-Encoding': 'gzip, deflate',
            'Accept-Language': 'zh-CN,zh;q=0.8,en-US;q=0.5,en;q=0.3',
            'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4515.159 Safari/537.36'
        }

        self.default_output_args = [
            '-bsf:a', 'aac_adtstoasc',
        ]
        if config.get('segment_time'):
            self.default_output_args += \
                ['-segment_time', f"{config.get('segment_time', '00:50:00')}"]
        else:
            self.default_output_args += \
                ['-fs', f"{config.get('file_size', '2621440000')}"]

    def check_stream(self):
        logger.debug(self.fname)
        raise NotImplementedError()

    def download(self, filename):
        if self.filename_prefix:  # 判断是否存在自定义录播命名设置
            filename = (self.filename_prefix.format(streamer=self.fname, title=self.room_title).encode(
                'unicode-escape').decode()).encode().decode("unicode-escape")
        else:
            filename = f'{self.fname}%Y-%m-%dT%H_%M_%S'
        filename = get_valid_filename(filename)
        fmtname = time.strftime(filename.encode("unicode-escape").decode()).encode().decode("unicode-escape")
        threading.Thread(target=asyncio.run, args=(self.danmaku_download_start(fmtname),)).start()
        if self.downloader == 'stream-gears':
            stream_gears_download(self.raw_stream_url, self.fake_headers, filename, config.get('segment_time'),
                                  config.get('file_size'))
        elif self.downloader == 'streamlink':
            parsed_url = urlparse(self.raw_stream_url)
            path = parsed_url.path
            if '.flv' in path: #streamlink无法处理flv,所以回退到ffmpeg
                self.ffmpeg_download(fmtname)
            else:
                self.streamlink_download(fmtname)
        else:
            self.ffmpeg_download(fmtname)

    def get_live_cover(self, uname, room_id, filename, timestamp, cover_url):
        import requests
        headers = self.fake_headers.copy()
        response = requests.get(cover_url, headers=headers, timeout=5)
        save_dir = f'cover/{uname}_{room_id}/'
        local_time = time.strftime('%Y-%m-%d_%H-%M-%S', time.localtime(timestamp))
        if not os.path.exists(save_dir):
            os.makedirs(save_dir)
        cover_file_name = get_valid_filename(f'{filename}_{local_time}.png')
        live_cover_path = f'{save_dir}{cover_file_name}'
        if os.path.exists(live_cover_path):
            return live_cover_path
        else:
            with open(live_cover_path, 'wb') as f:
                f.write(response.content)
                return live_cover_path

    def streamlink_download(self, filename): #streamlink+ffmpeg混合下载模式，适用于下载hls流
        streamlink_input_args = ['--stream-segment-threads', '6', '--hls-playlist-reload-attempts', '1']
        streamlink_cmd = ['streamlink', *streamlink_input_args, self.raw_stream_url, 'best', '-O']
        ffmpeg_input_args = ['-reconnect_streamed', '1', '-reconnect_delay_max', '20', '-rw_timeout', '20000000']
        ffmpeg_cmd = ['ffmpeg', '-re', '-i', 'pipe:0', '-y',*ffmpeg_input_args, *self.default_output_args, *self.opt_args, '-c', 'copy', '-f', self.suffix]
        if config.get('segment_time'):
            ffmpeg_cmd += ['-f', 'segment',
                     f'{filename} part-%03d.{self.suffix}']
        else:
            ffmpeg_cmd += [
                f'{filename}.{self.suffix}.part']
        streamlink_proc = subprocess.Popen(streamlink_cmd, stdout=subprocess.PIPE)
        ffmpeg_proc = subprocess.Popen(ffmpeg_cmd, stdin=streamlink_proc.stdout, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        try:
            with ffmpeg_proc.stdout as stdout:
                for line in iter(stdout.readline, b''):
                    decode_line = line.decode(errors='ignore')
                    print(decode_line, end='', file=sys.stderr)
                    logger.debug(decode_line.rstrip())
            retval = ffmpeg_proc.wait()
        except KeyboardInterrupt:
            if sys.platform != 'win32':
                ffmpeg_proc.communicate(b'q')
            raise
        return retval

    def ffmpeg_download(self, filename):
        default_input_args = ['-headers', ''.join('%s: %s\r\n' % x for x in self.fake_headers.items()),
                              '-reconnect_streamed', '1', '-reconnect_delay_max', '20', '-rw_timeout', '20000000']
        parsed_url = urlparse(self.raw_stream_url)
        path = parsed_url.path
        if '.m3u8' in path:
            default_input_args += ['-max_reload', '3']
        args = ['ffmpeg', '-y', *default_input_args,
                '-i', self.raw_stream_url, *self.default_output_args, *self.opt_args,
                '-c', 'copy', '-f', self.suffix]
        if config.get('segment_time'):
            args += ['-f', 'segment',
                     f'{filename} part-%03d.{self.suffix}']
        else:
            args += [
                f'{filename}.{self.suffix}.part']

        proc = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        try:
            with proc.stdout as stdout:
                for line in iter(stdout.readline, b''):  # b'\n'-separated lines
                    decode_line = line.decode(errors='ignore')
                    print(decode_line, end='', file=sys.stderr)
                    logger.debug(decode_line.rstrip())
            retval = proc.wait()
        except KeyboardInterrupt:
            if sys.platform != 'win32':
                proc.communicate(b'q')
            raise
        return retval

    async def danmaku_download_start(self, filename):
        pass

    def run(self):
        if not self.check_stream():
            return False
        file_name = self.file_name
        retval = self.download(file_name)
        logger.info(f'{retval}part: {file_name}.{self.suffix}')
        self.rename(f'{file_name}.{self.suffix}')
        return retval

    def start(self):
        i = 0
        logger.info('开始下载%s：%s' % (self.__class__.__name__, self.fname))
        date = time.localtime()
        while i < 30:
            try:
                ret = self.run()
            except:
                logger.exception("Uncaught exception:")
                continue
            finally:
                self.close()
            if ret is False:
                if config.get('delay'):
                    time.sleep(config.get('delay'))
                    logger.info(f"delay: {config.get('delay')}")
                    if self.check_stream():
                        time.sleep(5)
                        continue
                break
            elif ret == 1 or self.downloader == 'stream-gears':
                time.sleep(45)
            i += 1
        logger.info(f'退出下载{i}: {self.fname}')
        return {
            'name': self.fname,
            'url': self.url,
            'title': self.room_title,
            'date': date,
            'live_cover_path': self.live_cover_path,
        }

    @staticmethod
    def rename(file_name):
        try:
            os.rename(file_name + '.part', file_name)
            logger.debug('更名{0}为{1}'.format(file_name + '.part', file_name))
        except FileNotFoundError:
            logger.info('FileNotFoundError:' + file_name)
        except FileExistsError:
            os.rename(file_name + '.part', file_name)
            logger.info('FileExistsError:更名{0}为{1}'.format(file_name + '.part', file_name))

    @property
    def file_name(self):
        if self.filename_prefix:  # 判断是否存在自定义录播命名设置
            filename = (self.filename_prefix.format(streamer=self.fname, title=self.room_title).encode(
                'unicode-escape').decode()).encode().decode("unicode-escape")
        else:
            filename = f'{self.fname}%Y-%m-%dT%H_%M_%S'
        filename = get_valid_filename(filename)
        return time.strftime(filename.encode("unicode-escape").decode()).encode().decode("unicode-escape")

    def close(self):
        pass


def stream_gears_download(url, headers, file_name, segment_time=None, file_size=None):
    class Segment:
        pass

    segment = Segment()
    if segment_time:
        seg_time = segment_time.split(':')
        print(int(seg_time[0]) * 60 * 60 + int(seg_time[1]) * 60 + int(seg_time[2]))
        segment.time = int(seg_time[0]) * 60 * 60 + int(seg_time[1]) * 60 + int(seg_time[2])
    if file_size:
        segment.size = file_size
    if file_size is None and segment_time is None:
        segment.size = 8 * 1024 * 1024 * 1024
    stream_gears.download(
        url,
        headers,
        file_name,
        segment
    )


def get_valid_filename(name):
    """
    Return the given string converted to a string that can be used for a clean
    filename. Remove leading and trailing spaces; convert other spaces to
    underscores; and remove anything that is not an alphanumeric, dash,
    underscore, or dot.
    # >>> get_valid_filename("john's portrait in 2004.jpg")
    >>> get_valid_filename("{self.fname}%Y-%m-%dT%H_%M_%S")
    '{self.fname}%Y-%m-%dT%H_%M_%S'
    """
    # s = str(name).strip().replace(" ", "_") #因为有些人会在主播名中间加入空格，为了避免和录播完毕自动改名冲突，所以注释掉
    s = re.sub(r"(?u)[^-\w.%{}\[\]【】「」\s]", "", str(name))
    if s in {"", ".", ".."}:
        raise RuntimeError("Could not derive file name from '%s'" % name)
    return s
