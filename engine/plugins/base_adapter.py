import os
import subprocess
import sys
import time

import streamlink
import youtube_dl

from engine.plugins import logger, Monitoring

fake_headers = {
    'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
    'Accept-Encoding': 'gzip, deflate',
    'Accept-Language': 'zh-CN,zh;q=0.8,en-US;q=0.5,en;q=0.3',
    'User-Agent': 'Mozilla/5.0 (X11; Linux x86_64; rv:38.0) Gecko/20100101 Firefox/38.0 Iceweasel/38.2.1'
}


class DownloadBase:
    url_list = None

    def __init__(self, fname, url, suffix=None):
        self.fname = fname
        self.url = url
        self.suffix = suffix
        self.flag = None

    def check_stream(self):
        logger.debug(self.fname)
        raise NotImplementedError()

    def download(self, filename):
        raise NotImplementedError()

    def run(self):
        try:
            logger.info('开始下载%s：%s' % (self.__class__.__name__, self.fname))
            self.start()
        except:
            logger.exception("Uncaught exception:")
        finally:
            logger.info('退出下载')

    def start(self):
        file_name = self.file_name
        if self.check_stream():
            file_name += "." + self.suffix
            pid = os.getpid()
            monitor = Monitoring(pid, file_name)
            self.flag = monitor.flag
            monitor.start()
            retval = self.download(file_name)
            self.rename(file_name)
            monitor.stop()
            if retval != 0:
                logger.debug('准备递归下载')
                self.start()
            else:
                logger.info('下载完成' + self.fname)

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
    # url_list = None

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
            # cu = self.url.get(self.__class__.__name__)
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
            # ydl.download([self.url[self.__class__.__name__]])
            ydl.download([self.url])


class SDownload(DownloadBase):
    def __init__(self, fname, url, suffix='mp4'):
        super().__init__(fname, url, suffix)
        self.stream = None

    def check_stream(self):
        logger.debug(self.fname)
        try:
            streams = streamlink.streams(self.url)
            if streams:
                self.stream = streams["best"]
                fd = self.stream.open()
                fd.close()
                streams.close()
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


# ffmpeg.exe -i  http://vfile1.grtn.cn/2018/1542/0254/3368/154202543368.ssm/154202543368.m3u8
# -c copy -bsf:a aac_adtstoasc -movflags +faststart output.mp4
class FFmpegdl(DownloadBase):
    def __init__(self, fname, url, suffix=None):
        super().__init__(fname, url, suffix)
        self.raw_stream_url = None

    def download(self, filename):
        args = ['ffmpeg', '-headers', ''.join('%s: %s\r\n' % x for x in fake_headers.items()),
                '-y', '-i', self.raw_stream_url, '-bsf:a', 'aac_adtstoasc', '-c', 'copy', '-f', self.suffix,
                filename + '.part']
        proc = subprocess.Popen(args, stdin=subprocess.PIPE)
        try:
            retval = proc.wait()
        except KeyboardInterrupt:
            if sys.platform != 'win32':
                proc.communicate(b'q')
            raise
        return retval


class UploadBase:
    def __init__(self, principal, data, persistence_path=None):
        self.principal = principal
        self.persistence_path = persistence_path
        self.data = data

    # @property
    @staticmethod
    def file_list(index):
        file_list = []
        for file_name in os.listdir('.'):
            if index in file_name:
                file_list.append(file_name)
        file_list = sorted(file_list)
        return file_list

    @staticmethod
    def remove_filelist(file_list):
        for r in file_list:
            os.remove(r)
            logger.info('删除-' + r)

    @staticmethod
    def filter_file(index):
        file_list = UploadBase.file_list(index)
        if len(file_list) == 0:
            return False
        for r in file_list:
            file_size = os.path.getsize(r) / 1024 / 1024 / 1024
            if file_size <= 0.02:
                os.remove(r)
                logger.info('过滤删除-' + r)
        file_list = UploadBase.file_list(index)
        if len(file_list) == 0:
            logger.info('视频过滤后无文件可传')
            return False
        for f in file_list:
            if f.endswith('.part'):
                os.rename(f, os.path.splitext(f)[0])
                logger.info('%s存在已更名' % f)
        return True

    def upload(self, file_list):
        raise NotImplementedError()

    def start(self):
        if self.filter_file(self.principal):
            logger.info('准备上传' + self.data["format_title"])
            self.upload(UploadBase.file_list(self.principal))
