import os
import requests
import json
import time
import youtube_dl
from Engine import Enginebase, logger, links_id

# logger = logging.getLogger('log01')

# links = ['www.twitch.tv/innovation_s2','www.panda.tv/1160340',
# 'www.twitch.tv/sc2soo','www.panda.tv/1150595','www.twitch.tv/kimdaeyeob3']

# root_url = ['https://www.twitch.tv/', 'https://www.panda.tv/', 'https://api.twitch.tv/kraken/streams/']
#
# links_id = {
#     '星际2Innovation吕布卫星人族天梯第一视角': ['innovation_s2', '1160340'],
#     '星际2soO输本虫族天梯第一视角': ['sc2soo', '1150595'],
#     '星际2sOs狗哥神族天梯第一视角': ['', '1160930'],
#     '星际2Stats拔本神族天梯第一视角': ['kimdaeyeob3'],
#     '星际2Dark暗本虫族天梯第一视角': ['qkrfuddn0'],
#     '星际2Scarlett噶姐虫族天梯第一视角': ['scarlettm'],
#     '星际2GuMiho砸本人族天梯第一视角': ['gumiho'],
#     '星际2Maru人族天梯第一视角': ['maru072'],
#     '星际2TY全教主全太阳人族天梯第一视角': ['sc2tyty'],
#     '星际2ByuN武圣人族天梯第一视角': ['byunprime'],
#     '星际2小herO神族天梯第一视角': ['dmadkr0818'],
#     '星际2Zest神族天梯第一视角': ['sc2_zest'],
#     '星际2PartinG跳跳胖丁神族天梯第一视角':['partingthebigboy'],
# '星际ForGG火车王人族天梯第一视角': ['forgg'],
# '星际2NoRegreT莲弟虫族天梯第一视角': ['noregret_']
# 'test':['expertmma','37229'],
# 'test1':['byunprime','10003']
# }

headers = {
    'client-id': 'jzkbprff40iqj646a697cyrvl0zt2m6'
}


class Downloadbase(Enginebase):

    def __init__(self, dictionary, key, suffix, queue):
        Enginebase.__init__(self, dictionary, key, suffix)
        self.value_ = None
        self.queue = queue
        self.info_list = []

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
        with youtube_dl.YoutubeDL() as ydl:
            cu = self.url.get(self.__class__.__name__)
            if cu:
                info = ydl.extract_info(cu, download=False)
            else:
                print('%s不存在' % self.__class__.__name__)
                return
            for i in info['formats']:
                self.info_list.append(i['format_id'])
            print(self.info_list)

    def is_recursion(self):
        if self.value_ is None:

            value_ = self.dic[self.key]
            self.dic.pop(self.key)

            self.value_ = value_
        else:
            logger.info('准备递归下载' + self.key)
        return

    def download(self, ydl_opts, event):
        pass

    def dl(self, ydl_opts):
        with youtube_dl.YoutubeDL(ydl_opts) as ydl:
            pid = os.getpid()
            self.queue.put([pid, self.file_name])
            ydl.download([self.url[self.__class__.__name__]])

        print('下载完成')
        logger.info('下载完成' + self.key)

    def rename(self):
        file_name = self.file_name
        try:
            # logger.info('更名{0}'.format(pfile_name_+ '.part'))
            os.rename(file_name + '.part', file_name[:-4] + str(time.time())[:10] + file_name[-4:])
            logger.info('更名{0}为{1}+时间'.format(file_name + '.part', file_name))
        except FileExistsError:
            os.rename(file_name + '.part', file_name[:-4] + str(time.time())[:10] + file_name[-4:])
            logger.info('FileExistsError:更名{0}为{1}'.format(file_name + '.part', file_name))

    def run(self, event):
        file_name = self.file_name
        event.dict_['url'] = self.url[self.__class__.__name__]
        if event.dict_.get('file_name'):
            event.dict_['file_name'] += [file_name]
        else:
            event.dict_['file_name'] = [file_name]
        if self.check_stream():
            self.is_recursion()
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
                self.run(event)
            finally:
                self.dic[self.key] = self.value_
                logger.info('退出下载')


# def get_twitch_stream(url, key):
#     try:
#         res = requests.get(url, headers=headers)
#         res.close()
#     except requests.exceptions.SSLError:
#         logger.error('获取流信息发生错误')
#         logger.error(requests.exceptions.SSLError, exc_info = True)
#         return None
#     except requests.exceptions.ConnectionError:
#         logger.exception('During handling of the above exception, another exception occurred:')
#         return None
#
#     try:
#         s = json.loads(res.text)
#         # s = res.json()
#     except json.decoder.JSONDecodeError:
#         logger.exception('Expecting value')
#         return None
#     print(key)
#     try:
#         stream = s['stream']
#     except KeyError :
#         logger.error(KeyError, exc_info = True)
#         return None
#     return stream


# def download_twitch_stream(dict, q, confirm_url, key_, url_, tfile_name_, value_=None):
#     status_ = get_twitch_stream(confirm_url,key_)
#     # print(status_)
#     if str(status_) != 'None':
#         if value_ is None:
#             value_ = dict[key_]
#             dict.pop(key_)
#         else:
#             logger.info('准备递归下载'+key_)
#         print('开始下载twitch', key_)
#         logger.info('开始下载twitch：' + key_)
#         ydl_opts = {
#             'outtmpl': tfile_name_,
#             # 'format': '720p'
#             # 'external_downloader_args':['-timeout', '5']
#             # 'keep_fragments':True
#         }
#         list_info = []
#
#         with youtube_dl.YoutubeDL() as ydl:
#             info = ydl.extract_info(url_, download=False)
#             for i in info['formats']:
#                 list_info.append(i['format_id'])
#
#         if key_ in ['星际2ByuN武圣人族天梯第一视角', '星际2Innovation吕布卫星人族天梯第一视角', '星际2Maru人族天梯第一视角']:
#             pass
#         elif '720p' in list_info:
#             ydl_opts['format'] = '720p'
#         elif '720p60' in list_info:
#             ydl_opts['format'] = '720p60'
#
#
#         try:
#             with youtube_dl.YoutubeDL(ydl_opts) as ydl:
#
#                 pid = os.getpid()
#                 q.put([pid, tfile_name_])
#
#                 ydl.download([url_])
#
#             print('下载完成')
#             logger.info('下载完成' + key_)
#
#             uploads(tfile_name_, url_)
#
#         except youtube_dl.utils.DownloadError:
#
#             # print('分段下载')
#             try:
#                 os.rename(tfile_name_ + '.part', tfile_name_[:-4] + str(time.time())[:10] + tfile_name_[-4:])
#                 logger.info('更名{0}为{1}+时间'.format(tfile_name_ + '.part', tfile_name_))
#             except FileExistsError:
#                 os.rename(tfile_name_ + '.part', tfile_name_[:-4] + str(time.time())[:10] + tfile_name_[-4:])
#                 logger.info('FileExistsError更名{0}为{1}'.format(tfile_name_ + '.part', tfile_name_))
#
#             download_twitch_stream(dict, q, confirm_url, key_, url_, tfile_name_, value_)
#
#         finally:
#             dict[key_] = value_
#             logger.info('退出下载')
#
#     elif len(dict[key_]) == 1:
#         value_ = dict[key_]
#         dict.pop(key_)
#
#         supplemental_upload(dict, tfile_name_, key_, url_, value_)


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
        self.get_sinfo()

        if self.key in ['星际2ByuN武圣人族天梯第一视角', '星际2Innovation吕布卫星人族天梯第一视角', '星际2Maru人族天梯第一视角']:
            pass
        elif '720p' in self.info_list:
            ydl_opts['format'] = '720p'
        elif '720p60' in self.info_list:
            ydl_opts['format'] = '720p60'

        self.dl(ydl_opts)

        # elif len(dict[key_]) == 1:
        #     value_ = dict[key_]
        #     dict.pop(key_)
        #
        #     supplemental_upload(dict, tfile_name_, key_, url_, value_)


class Panda(Downloadbase):
    def __init__(self, dictionary, key, queue, suffix='flv'):
        Downloadbase.__init__(self, dictionary=dictionary, key=key, suffix=suffix, queue=queue)

    def download(self, ydl_opts, event):
        file_name = self.file_name
        # if value_ is None:
        #     value_ = dict[key_]
        #     dict.pop(key_)
        # else:
        #     logger.info('准备递归下载'+key_)
        # list_info = []
        # with youtube_dl.YoutubeDL() as ydl:
        #     try:
        #         info = ydl.extract_info(url_, download=False)
        #         for i in info['formats']:
        #             list_info.append(i['format_id'])
        #     except youtube_dl.utils.DownloadError:
        #         logger.debug('%s未开播或读取下载信息失败' % key_)
        #
        #         # logger.debug('准备补充上传'+key_)
        #         supplemental_upload(dict, pfile_name_, key_, url_, value_)
        #         return
        # file_name = self.join_filename('mp4')

        # if 'HD-flv' in list_info:
        #     ydl_opts = {
        #         'outtmpl': pfile_name_,
        #         'format': 'HD-flv',
        #     }
        # else:
        #     ydl_opts = {
        #         'outtmpl': pfile_name_,
        #         'format': 'SD-flv',
        #         # 'keep_fragments':True
        #         # 'postprocessors': [{
        #         #     'key': 'FFmpegFixupM3u8',
        #         # 'preferredcodec': 'mp3',
        #         # 'preferredquality': '1900',
        #         # }],
        #     }
        print('开始下载panda', self.key)
        logger.info('开始下载panda：' + self.key)
        self.dl(ydl_opts)

        if self.check_stream():
            logger.info('实际未下载完成' + self.key)
            if os.path.isfile(file_name):
                os.rename(file_name, file_name[:-4] + str(time.time())[:10] + file_name[-4:])
                logger.info(
                    '存在{0}更名为{1}'.format(file_name, file_name[:-4] + '时间' + file_name[-4:]))
            self.run(event)


# def download_stream(dict, q, confirm_url, key_, twitch_url_, panda_url_, tfile_name_, pfile_name_):
# print('子进程id%s'%(os.getpid()))
#     download_twitch_stream(dict, q, confirm_url, key_, twitch_url_, tfile_name_)
#     download_panda_stream(dict, q, key_, panda_url_, pfile_name_)


if __name__ == '__main__':
    # get_twitch_stream('https://api.twitch.tv/kraken/streams/1160340','233')
    for k in links_id:
        pd = Panda(dictionary=links_id, key=k, queue=1)
        pd.check_stream()
