import os
import re
import time
from threading import Thread
import youtube_dl
from Engine.work import kill_child_processes
from common import logger


class Download(object):
    url_list = None

    def __init__(self, fname, url, suffix='flv'):
        self.fname = fname
        self.url = url
        self.suffix = suffix

    @property
    def file_name(self):
        # now = Engine.work.time_now()
        # if self.suffix == 'mp4':
        file_name = '%s%s.%s' % (self.fname, str(time.time())[:10], self.suffix)
        # elif self.suffix == 'flv':
        #     file_name = '%s%s%s.flv' % (self.fname, now, str(time.time())[:10])
        # else:
        #     raise ValueError
        return file_name

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

    def download(self, ydl_opts, event):
        self.dl(ydl_opts)

    def dl(self, ydl_opts):
        with youtube_dl.YoutubeDL(ydl_opts) as ydl:
            # ydl.download([self.url[self.__class__.__name__]])
            ydl.download([self.url])

    @staticmethod
    def rename(file_name):
        try:
            os.rename(file_name + '.part', file_name)
            logger.info('更名{0}为{1}'.format(file_name + '.part', file_name))
        except FileExistsError:
            os.rename(file_name + '.part', file_name)
            logger.info('FileExistsError:更名{0}为{1}'.format(file_name + '.part', file_name))

    def run(self, event=None):
        file_name = self.file_name
        # event.dict_['url'] = self.url[self.__class__.__name__]
        # if event.dict_.get('file_name'):
        #     event.dict_['file_name'] += [file_name]
        # else:
        #     event.dict_['file_name'] = [file_name]
        if self.check_stream():
            ydl_opts = {
                'outtmpl': file_name,
                # 'format': '720p'
                # 'external_downloader_args':['-timeout', '5']
                # 'keep_fragments':True
            }
            try:
                logger.info('开始下载%s：%s' % (self.__class__.__name__, self.fname))
                pid = os.getpid()
                # fname = ydl_opts['outtmpl']
                # self.queue.put([pid, fname])
                t = Thread(target=kill_child_processes, args=(pid, file_name))
                t.start()
                self.download(ydl_opts, event)
                logger.info('下载完成' + self.fname)

            except youtube_dl.utils.DownloadError:
                self.rename(file_name)
                logger.info('准备递归下载')
                self.run(event)
            finally:
                logger.info('退出下载')
        return


class BatchCheckBase(object):
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
