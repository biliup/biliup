import logging
import os
import subprocess
import sys
import time

from biliup import config
from ..plugins import fake_headers

logger = logging.getLogger('biliup')


class DownloadBase:
    def __init__(self, fname, url, suffix=None, opt_args=None):
        if opt_args is None:
            opt_args = []
        self.fname = fname
        self.url = url
        self.suffix = suffix
        # ffmpeg.exe -i  http://vfile1.grtn.cn/2018/1542/0254/3368/154202543368.ssm/154202543368.m3u8
        # -c copy -bsf:a aac_adtstoasc -movflags +faststart output.mp4
        self.raw_stream_url = None
        self.opt_args = opt_args

        self.default_output_args = [
            '-bsf:a', 'aac_adtstoasc',
        ]
        if config.get('segment_time'):
            self.default_output_args += \
                ['-segment_time', f"{config.get('segment_time') if config.get('segment_time') else '00:50:00'}"]
        else:
            self.default_output_args += \
                ['-fs', f"{config.get('file_size') if config.get('file_size') else '2621440000'}"]
        self.default_input_args = ['-headers', ''.join('%s: %s\r\n' % x for x in fake_headers.items()),
                                   '-reconnect_streamed', '1', '-reconnect_delay_max', '20', '-rw_timeout', '20000000']

    def check_stream(self):
        logger.debug(self.fname)
        raise NotImplementedError()

    def download(self, filename):
        args = ['ffmpeg', '-y', *self.default_input_args,
                '-i', self.raw_stream_url, *self.default_output_args, *self.opt_args,
                '-c', 'copy', '-f', self.suffix]
        if config.get('segment_time'):
            args += ['-f', 'segment', f'{self.fname} {time.strftime("%Y-%m-%d %H_%M_%S", time.localtime())} part-%03d.{self.suffix}']
        else:
            args += [f'{filename}.part']

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
                try:
                    ret = self.run()
                    if ret is False:
                        return
                    elif ret == 1:
                        time.sleep(45)
                except:
                    logger.exception("Uncaught exception:")
                    continue
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
