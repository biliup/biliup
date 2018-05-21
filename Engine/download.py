import os
import requests
import json
import time
import youtube_dl
from Engine import Enginebase, logger, links_id

# logger = logging.getLogger('log01')

headers = {
    'client-id': 'jzkbprff40iqj646a697cyrvl0zt2m6'
}


class Downloadbase(Enginebase):

    def __init__(self, dictionary, key, suffix, queue):
        Enginebase.__init__(self, dictionary, key, suffix)
        self.queue = queue

    def check_stream(self):
        try:
            self.get_sinfo()
            return True
        except youtube_dl.utils.DownloadError:
            # logger.debug('%s未开播或读取下载信息失败' % self.key)
            print('%s未开播或读取下载信息失败' % self.key)
            # logger.debug('准备补充上传'+key_)
            # supplemental_upload(self.dic, pfile_name_, key_, url_, value_)
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

    def is_recursion(self, value_):
        if value_ is None:

            value_ = self.dic[self.key]
            self.dic.pop(self.key)

            value_ = value_
        else:
            logger.info('准备递归下载' + self.key)

        return value_

    def download(self, ydl_opts, event):
        logger.info('开始下载%s：%s' % (self.__class__.__name__, self.key))
        self.dl(ydl_opts)

    def dl(self, ydl_opts):
        with youtube_dl.YoutubeDL(ydl_opts) as ydl:
            pid = os.getpid()
            self.queue.put([pid, self.file_name])
            ydl.download([self.url[self.__class__.__name__]])

        print('下载完成')
        logger.info('下载完成' + self.key)

    def rename(self):
        file_name = self.file_name
        fname = os.path.splitext(file_name)[0]
        suffix = os.path.splitext(file_name)[1]
        try:
            # logger.info('更名{0}'.format(pfile_name_+ '.part'))
            os.rename(file_name + '.part', fname + str(time.time())[:10] + suffix)
            logger.info('更名{0}为{1}+时间'.format(file_name + '.part', file_name))
        except FileExistsError:
            os.rename(file_name + '.part', fname + str(time.time())[:10] + suffix)
            logger.info('FileExistsError:更名{0}为{1}'.format(file_name + '.part', file_name))

    def run(self, event, value=None):
        file_name = self.file_name
        event.dict_['url'] = self.url[self.__class__.__name__]
        if event.dict_.get('file_name'):
            event.dict_['file_name'] += [file_name]
        else:
            event.dict_['file_name'] = [file_name]
        if self.check_stream():
            value = self.is_recursion(value)
            ydl_opts = {
                'outtmpl': file_name,
                # 'format': '720p'
                # 'external_downloader_args':['-timeout', '5']
                # 'keep_fragments':True
            }
            try:
                self.download(ydl_opts, event)
            except youtube_dl.utils.DownloadError or KeyboardInterrupt:
                self.rename()
                self.run(event, value)
            finally:
                self.dic[self.key] = value
                logger.info('退出下载')


class Twitch(Downloadbase):
    def __init__(self, dictionary, key, queue, suffix='mp4'):
        Downloadbase.__init__(self, dictionary=dictionary, key=key, suffix=suffix, queue=queue)

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
        logger.info('开始下载twitch：' + self.key)
        info_list = self.get_sinfo()

        if self.key in ['星际2ByuN武圣人族天梯第一视角', '星际2Innovation吕布卫星人族天梯第一视角', '星际2Maru人族天梯第一视角']:
            pass
        elif '720p' in info_list:
            ydl_opts['format'] = '720p'
        elif '720p60' in info_list:
            ydl_opts['format'] = '720p60'

        self.dl(ydl_opts)


class Panda(Downloadbase):
    def __init__(self, dictionary, key, queue, suffix='flv'):
        Downloadbase.__init__(self, dictionary=dictionary, key=key, suffix=suffix, queue=queue)

    def download(self, ydl_opts, event):
        file_name = self.file_name
        fname = os.path.splitext(file_name)[0]
        suffix = os.path.splitext(file_name)[1]

        print('开始下载panda', self.key)
        logger.info('开始下载panda：' + self.key)
        self.dl(ydl_opts)

        if self.check_stream():
            logger.info('实际未下载完成' + self.key)
            if os.path.isfile(file_name):
                os.rename(file_name, fname + str(time.time())[:10] + suffix)
                logger.info(
                    '存在{0}更名为{1}'.format(file_name, fname + '时间' + suffix))
            self.run(event)


if __name__ == '__main__':
    # get_twitch_stream('https://api.twitch.tv/kraken/streams/1160340','233')
    for k in links_id:
        pd = Panda(dictionary=links_id, key=k, queue=1)
        pd.check_stream()
