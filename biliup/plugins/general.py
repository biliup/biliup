import os
import subprocess
import sys
import time
from threading import Event

import streamlink
import youtube_dl
from ykdl.common import url_to_module

from ..engine import config
from . import logger, fake_headers


class DownloadBase:
    def __init__(self, fname, url, suffix=None):
        self.fname = fname
        self.url = url
        self.suffix = suffix
        # ffmpeg.exe -i  http://vfile1.grtn.cn/2018/1542/0254/3368/154202543368.ssm/154202543368.m3u8
        # -c copy -bsf:a aac_adtstoasc -movflags +faststart output.mp4
        self.raw_stream_url = None
        self.opt_args = []
        self.default_output_args = [
            '-bsf:a', 'aac_adtstoasc',
            '-fs', f"{config.get('file_size') if config.get('file_size') else '2621440000'}"
        ]
        self.default_input_args = ['-headers', ''.join('%s: %s\r\n' % x for x in fake_headers.items()),
                                   '-reconnect_streamed', '1', '-reconnect_delay_max', '20', '-rw_timeout', '20000000']

    def check_stream(self):
        logger.debug(self.fname)
        raise NotImplementedError()

    def download(self, filename):
        args = ['ffmpeg', '-y', *self.default_input_args,
                '-i', self.raw_stream_url, *self.default_output_args, *self.opt_args,
                '-c', 'copy', '-f', self.suffix, f'{filename}.part']
        proc = subprocess.Popen(args, stdin=subprocess.PIPE)
        try:
            retval = proc.wait()
        except KeyboardInterrupt:
            if sys.platform != 'win32':
                proc.communicate(b'q')
            raise
        return retval

    def run(self):
        if not self.check_stream():
            return False
        file_name = f'{self.file_name}.{self.suffix}'
        retval = self.download(file_name)
        logger.info(f'{retval}part: {file_name}')
        self.rename(file_name)
        return retval

    def start(self):
        i = 0
        try:
            logger.info('开始下载%s：%s' % (self.__class__.__name__, self.fname))
            while i < 30:
                ret = self.run()
                if ret is False:
                    return
                elif ret == 1:
                    time.sleep(45)
                i += 1
        except:
            logger.exception("Uncaught exception:")
        finally:
            logger.info(f'退出下载{i}: {self.fname}')

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
        file_name = '%s%s' % (self.fname, str(time.time())[:10])
        return file_name


class YDownload(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.ydl_opts = {}

    def check_stream(self):
        try:
            self.get_sinfo()
            return True
        except youtube_dl.utils.DownloadError:
            logger.debug('%s未开播或读取下载信息失败' % self.fname)
            return False

    def get_sinfo(self):
        info_list = []
        with youtube_dl.YoutubeDL() as ydl:
            if self.url:
                info = ydl.extract_info(self.url, download=False)
            else:
                logger.debug('%s不存在' % self.__class__.__name__)
                return
            for i in info['formats']:
                info_list.append(i['format_id'])
            logger.debug(info_list)
        return info_list

    def download(self, filename):
        try:
            self.ydl_opts = {'outtmpl': filename}
            self.dl()
        except youtube_dl.utils.DownloadError:
            return 1
        return 0

    def dl(self):
        with youtube_dl.YoutubeDL(self.ydl_opts) as ydl:
            ydl.download([self.url])


class SDownload(DownloadBase):
    def __init__(self, fname, url, suffix='mp4'):
        super().__init__(fname, url, suffix)
        self.stream = None
        self.flag = Event()

    def check_stream(self):
        logger.debug(self.fname)
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

    def download(self, filename):

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
            self.rename(filename)
            raise


class Generic(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.handler = self

    def check_stream(self):
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

    def download(self, filename):
        if self.handler == self:
            return super(Generic, self).download(filename)
        return self.handler.download(filename)


__plugin__ = Generic
