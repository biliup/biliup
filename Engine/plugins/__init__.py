import os
import re
import subprocess
import sys
import time
from threading import Thread
import psutil
import streamlink
import youtube_dl
from common import logger


class DownloadBase:
    url_list = None

    def __init__(self, fname, url, suffix=None):
        self.fname = fname
        self.url = url
        self.suffix = suffix
        self.flag = True
        self.ydl_opts = {}

    def check_stream(self):
        logger.debug(self.fname)

    def download(self):
        pass

    def run(self):
        file_name = self.file_name
        self.ydl_opts = {'outtmpl': file_name}
        if self.check_stream():
            try:
                logger.info('开始下载%s：%s' % (self.__class__.__name__, self.fname))
                pid = os.getpid()
                t = Thread(target=self.kill_child_processes, args=(pid, file_name))
                t.start()
                retval = self.download()
                self.rename(file_name)
                if retval != 0:
                    logger.info('准备递归下载')
                    self.run()
                else:
                    logger.info('下载完成' + self.fname)

            # except youtube_dl.utils.DownloadError:
            #     self.rename(file_name)
            #     logger.info('准备递归下载')
            #     self.run()

            finally:
                logger.info('退出下载')
        return

    def kill_child_processes(self, parent_pid, file_name):
        file_name_ = file_name + '.part'
        last_file_size = 0.0
        logger.info('获取到{0}，{1}'.format(parent_pid, file_name_))
        while True:
            time.sleep(15)
            if os.path.isfile(file_name_):
                file_size = os.path.getsize(file_name_) / 1024 / 1024 / 1024
                file_sizes = os.path.getsize(file_name_)
                if float(file_sizes) == last_file_size:
                    try:
                        parent = psutil.Process(parent_pid)
                    except psutil.NoSuchProcess:
                        return
                    children = parent.children(recursive=True)
                    if len(children) == 0:
                        # parent.send_signal(sig)
                        # self.flag = False
                        logger.info('下载卡死' + self.__class__.__name__ + file_name_)
                        # parent.terminate()
                    else:
                        for process in children:
                            # print(process)
                            # process.send_signal(sig)
                            process.terminate()
                        logger.info('下载卡死' + self.__class__.__name__ + file_name_)
                    # time.sleep(1)
                    if os.path.isfile(file_name_):
                        logger.info('卡死下载进程可能未成功退出')
                        continue
                    else:
                        logger.info('卡死下载进程成功退出')
                        break

                last_file_size = file_sizes

                if float(file_size) >= 2.5:
                    try:
                        parent = psutil.Process(parent_pid)
                    except psutil.NoSuchProcess:
                        return
                    children = parent.children(recursive=True)
                    if len(children) == 0:
                        # parent.send_signal(sig)
                        # parent.terminate()
                        # logger.info('分段下载pandatv' + file_name_)
                        self.flag = False
                    else:
                        for process in children:
                            # print(process)
                            # process.send_signal(sig)
                            process.terminate()
                        # print('分段下载')
                        # logger.info('分段下载' + file_name_)
                    logger.info('分段下载' + self.__class__.__name__ + file_name_)
                    break
            else:
                logger.info('监控<%s>线程退出' % file_name_)
                return
                # os._exit(0)
        logger.info('退出监控<%s>线程' % file_name_)

    @staticmethod
    def rename(file_name):
        try:
            os.rename(file_name + '.part', file_name)
            logger.info('更名{0}为{1}'.format(file_name + '.part', file_name))
        except FileNotFoundError:
            logger.info('FileNotFoundError')
        except FileExistsError:
            os.rename(file_name + '.part', file_name)
            logger.info('FileExistsError:更名{0}为{1}'.format(file_name + '.part', file_name))

    @property
    def file_name(self):
        file_name = '%s%s.%s' % (self.fname, str(time.time())[:10], self.suffix)
        return file_name


class YDownload(DownloadBase):
    # url_list = None

    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        # self.fname = fname
        # self.url = url
        # self.suffix = suffix

    # @property
    # def file_name(self):
    #     # now = Engine.work.time_now()
    #     # if self.suffix == 'mp4':
    #     file_name = '%s%s.%s' % (self.fname, str(time.time())[:10], self.suffix)
    #     # elif self.suffix == 'flv':
    #     #     file_name = '%s%s%s.flv' % (self.fname, now, str(time.time())[:10])
    #     # else:
    #     #     raise ValueError
    #     return file_name

    def check_stream(self):
        try:
            self.get_sinfo()
            return True
        except youtube_dl.utils.DownloadError:
            # logger.debug('%s未开播或读取下载信息失败' % self.key)
            print('%s未开播或读取下载信息失败' % self.fname)
            return False

    def get_sinfo(self):
        info_list = []
        with youtube_dl.YoutubeDL() as ydl:
            # cu = self.url.get(self.__class__.__name__)
            if self.url:
                info = ydl.extract_info(self.url, download=False)
            else:
                print('%s不存在' % self.__class__.__name__)
                return
            for i in info['formats']:
                info_list.append(i['format_id'])
            print(info_list)
        return info_list

    def download(self):
        try:
            self.dl()
        except youtube_dl.utils.DownloadError:
            return 1
        return 0

    def dl(self):
        with youtube_dl.YoutubeDL(self.ydl_opts) as ydl:
            # ydl.download([self.url[self.__class__.__name__]])
            ydl.download([self.url])

    # @staticmethod
    # def rename(file_name):
    #     try:
    #         os.rename(file_name + '.part', file_name)
    #         logger.info('更名{0}为{1}'.format(file_name + '.part', file_name))
    #     except FileExistsError:
    #         os.rename(file_name + '.part', file_name)
    #         logger.info('FileExistsError:更名{0}为{1}'.format(file_name + '.part', file_name))

    # def run(self):
    #     file_name = self.file_name
    #     # event.dict_['url'] = self.url[self.__class__.__name__]
    #     # if event.dict_.get('file_name'):
    #     #     event.dict_['file_name'] += [file_name]
    #     # else:
    #     #     event.dict_['file_name'] = [file_name]
    #     if self.check_stream():
    #         ydl_opts = {
    #             'outtmpl': file_name,
    #             # 'format': '720p'
    #             # 'external_downloader_args':['-timeout', '5']
    #             # 'keep_fragments':True
    #         }
    #         try:
    #             logger.info('开始下载%s：%s' % (self.__class__.__name__, self.fname))
    #             pid = os.getpid()
    #             # fname = ydl_opts['outtmpl']
    #             # self.queue.put([pid, fname])
    #             t = Thread(target=kill_child_processes, args=(pid, file_name))
    #             t.start()
    #             self.download(ydl_opts)
    #             logger.info('下载完成' + self.fname)
    #
    #         except youtube_dl.utils.DownloadError:
    #             self.rename(file_name)
    #             logger.info('准备递归下载')
    #             self.run()
    #         finally:
    #             logger.info('退出下载')
    #     return


class SDownload(DownloadBase):
    def __init__(self, fname, url, suffix='mp4'):
        super().__init__(fname, url, suffix)
        self.stream = None

    def check_stream(self):
        streams = streamlink.streams(self.url)
        try:
            if streams:
                self.stream = streams["best"]
                fd = self.stream.open()
                fd.close()
                return True
        except streamlink.StreamlinkError:
            return

    def download(self):

        # fd = stream.open()
        try:
            with self.stream.open() as fd:
                with open(self.ydl_opts['outtmpl'] + '.part', 'wb') as file:
                    for f in fd:
                        file.write(f)
                        if not self.flag:
                            self.flag = True
                            return 1
                    return 0
        finally:
            self.rename(self.ydl_opts['outtmpl'])


class FFmpegdl(DownloadBase):
    def download(self):
        args = ['ffmpeg', '-y', '-i', self.ydl_opts['absurl'], '-c', 'copy', '-f', 'flv',
                self.ydl_opts['outtmpl'] + '.part']
        proc = subprocess.Popen(args, stdin=subprocess.PIPE)
        try:
            retval = proc.wait()
        except KeyboardInterrupt:
            if sys.platform != 'win32':
                proc.communicate(b'q')
            raise
        return retval


class BatchCheckBase:
    def __init__(self, pattern_id, urls):
        self.usr_dict = {}
        self.usr_list = []
        self.pattern_id = pattern_id
        for url in urls:
            self.get_id(url)

    def get_id(self, url):
        m = re.match(self.pattern_id, url)
        if m:
            usr_id = m.group('id')
            self.usr_dict[usr_id.lower()] = url
            self.usr_list.append(usr_id)

    def check(self):
        pass
