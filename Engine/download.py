import os
import signal
from threading import Thread

import requests
import json
import youtube_dl
from Engine import Enginebase, logger, links_id, work

# logger = logging.getLogger('log01')
from Engine.work import kill_child_processes

headers = {
    'client-id': 'jzkbprff40iqj646a697cyrvl0zt2m6'
}


class Downloadbase(Enginebase):

    def __init__(self, items, suffix='flv'):
        Enginebase.__init__(self, items, suffix)
        # self.queue = queue

    def check_stream(self):
        try:
            self.get_sinfo()
            return True
        except youtube_dl.utils.DownloadError:
            # logger.debug('%s未开播或读取下载信息失败' % self.key)
            print('%s未开播或读取下载信息失败' % self.key)
            return False

    def get_sinfo(self):
        info_list = []
        with youtube_dl.YoutubeDL() as ydl:
            cu = self.url.get(self.__class__.__name__)
            if cu:
                info = ydl.extract_info(cu, download=False)
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
            pid = os.getpid()
            fname = ydl_opts['outtmpl']
            # self.queue.put([pid, fname])
            t = Thread(target=kill_child_processes, args=(pid, fname))
            t.start()
            ydl.download([self.url[self.__class__.__name__]])

        print('下载完成')
        logger.info('下载完成' + self.key)

    @staticmethod
    def rename(file_name):
        try:
            os.rename(file_name + '.part', file_name)
            logger.info('更名{0}为{1}'.format(file_name + '.part', file_name))
        except FileExistsError:
            os.rename(file_name + '.part', file_name)
            logger.info('FileExistsError:更名{0}为{1}'.format(file_name + '.part', file_name))

    def run(self, event):
        file_name = self.file_name
        event.dict_['url'] = self.url[self.__class__.__name__]
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
                logger.info('开始下载%s：%s' % (self.__class__.__name__, self.key))
                self.download(ydl_opts, event)
            except youtube_dl.utils.DownloadError:
                self.rename(file_name)
                logger.info('准备递归下载')
                self.run(event)
            finally:
                logger.info('退出下载')


class Twitch(Downloadbase):
    def __init__(self, items, suffix='mp4'):
        Downloadbase.__init__(self, items, suffix=suffix )

    def check_stream(self):
        try:
            res = requests.get(self.url['Twitch_check'], headers=headers)
            res.close()
        except requests.exceptions.SSLError:
            logger.error('获取流信息发生错误')
            logger.error(requests.exceptions.SSLError, exc_info=True)
            return None
        except requests.exceptions.ConnectionError:
            logger.exception('During handling of the above exception, another exception occurred:')
            return None

        try:
            s = json.loads(res.text)
            # s = res.json()
        except json.decoder.JSONDecodeError:
            logger.exception('Expecting value')
            return None
        print(self.key)
        try:
            stream = s['stream']
        except KeyError:
            logger.error(KeyError, exc_info=True)
            return None
        return stream

    def download(self, ydl_opts, event):
        print('开始下载twitch', self.key)
        info_list = self.get_sinfo()

        if self.key in ['星际2ByuN武圣人族天梯第一视角', '星际2Innovation吕布卫星人族天梯第一视角', '星际2Maru人族天梯第一视角']:
            pass
        elif '720p' in info_list:
            ydl_opts['format'] = '720p'
        elif '720p60' in info_list:
            ydl_opts['format'] = '720p60'

        self.dl(ydl_opts)


class Panda(Downloadbase):
    def __init__(self, items, suffix='flv'):
        Downloadbase.__init__(self, items, suffix=suffix)

    def download(self, ydl_opts, event):

        print('开始下载panda', self.key)
        signal.signal(signal.SIGTERM, work.signal_handler)
        # info_list = self.get_sinfo()

        # if 'SD-m3u8' in info_list:
        #     ydl_opts['format'] = 'SD-m3u8'
        # elif 'HD-m3u8' in info_list:
        #     ydl_opts['format'] = 'HD-m3u8'

        self.dl(ydl_opts)

        if self.check_stream():
            logger.info('实际未下载完成' + self.key)
            logger.info('准备递归下载')
            self.run(event)


if __name__ == '__main__':
    # get_twitch_stream('https://api.twitch.tv/kraken/streams/1160340','233')
    for k in links_id:
        pd = Panda(k, queue=1)
        pd.check_stream()
