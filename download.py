from upload import *
import os
import requests
import json
import time
import youtube_dl

# logger = logging.getLogger('log01')

# links = ['www.twitch.tv/innovation_s2','www.panda.tv/1160340','www.twitch.tv/sc2soo','www.panda.tv/1150595','www.twitch.tv/kimdaeyeob3']

root_url = ['https://www.twitch.tv/', 'https://www.panda.tv/', 'https://api.twitch.tv/kraken/streams/']

links_id = {
    '星际2Innovation吕布卫星人族天梯第一视角': ['innovation_s2', '1160340'],
    '星际2soO输本虫族天梯第一视角': ['sc2soo', '1150595'],
    '星际2sOs狗哥神族天梯第一视角': ['', '1160930'],
    '星际2Stats拔本神族天梯第一视角': ['kimdaeyeob3'],
    '星际2Dark暗本虫族天梯第一视角': ['qkrfuddn0'],
    '星际2Scarlett噶姐虫族天梯第一视角': ['scarlettm'],
    '星际2GuMiho砸本人族天梯第一视角': ['gumiho'],
    '星际2Maru人族天梯第一视角': ['maru072'],
    '星际2ByuN武圣人族天梯第一视角': ['byunprime'],
    '星际2小herO神族天梯第一视角': ['dmadkr0818'],
    '星际2Zest神族天梯第一视角': ['sc2_zest'],
    '星际2PartinG跳跳胖丁神族天梯第一视角':['partingthebigboy']
    # 'test':['maru072','37229'],
    # 'test1':['byunprime','10003']
}

headers = {
    'client-id': 'jzkbprff40iqj646a697cyrvl0zt2m6'
}


def get_twitch_stream(url, key):
    try:
        res = requests.get(url, headers=headers)
        res.close()
    except requests.exceptions.ConnectionError:
        logger.exception('During handling of the above exception, another exception occurred:')
        return None
    except requests.exceptions.SSLError:
        logger.error('获取流信息发生错误')
        logger.error(requests.exceptions.SSLError, exc_info = True)
        return None
    try:
        s = json.loads(res.text)
        # s = res.json()
    except json.decoder.JSONDecodeError:
        logger.exception('Expecting value')
        return None
    print(key)
    try:
        stream = s['stream']
    except KeyError :
        logger.error(KeyError, exc_info = True)
        return None
    return stream


def download_twitch_stream(dict, q, confirm_url, key_, url_, tfile_name_, value_=None):
    status_ = get_twitch_stream(confirm_url,key_)
    # print(status_)
    if str(status_) != 'None':
        if value_ is None:
            value_ = dict[key_]
            dict.pop(key_)
        else:
            logger.info('准备递归下载'+key_)
        print('开始下载twitch', key_)
        logger.info('开始下载twitch：' + key_)
        ydl_opts = {
            'outtmpl': tfile_name_,
            'format': '720p60'
            # 'keep_fragments':True
            # 'postprocessors': [{
            #     'key': 'FFmpegFixupM3u8',
            # 'preferredcodec': 'mp3',
            # 'preferredquality': '1900',
            # }],
        }
        try:
            with youtube_dl.YoutubeDL(ydl_opts) as ydl:

                pid = os.getpid()
                q.put([pid, tfile_name_])

                ydl.download([url_])

            print('下载完成')
            logger.info('下载完成' + key_)

            uploads(tfile_name_, url_)

        except youtube_dl.utils.DownloadError:

            # print('分段下载')
            try:
                os.rename(tfile_name_ + '.part', tfile_name_[:-4] + str(time.time())[:10] + tfile_name_[-4:])
                logger.info('更名{0}为{1}+时间'.format(tfile_name_ + '.part', tfile_name_))
            except FileExistsError:
                os.rename(tfile_name_ + '.part', tfile_name_[:-4] + str(time.time())[:10] + tfile_name_[-4:])

            download_twitch_stream(dict, q, confirm_url, key_, url_, tfile_name_, value_)

        finally:
            dict[key_] = value_
            logger.info('退出下载')

    elif len(dict[key_]) == 1:
        value_ = dict[key_]
        dict.pop(key_)

        supplemental_upload(dict, tfile_name_, key_, url_, value_)


def download_panda_stream(dict, q, key_, url_, pfile_name_, value_=None):
    if value_ is None:
        value_ = dict[key_]
        dict.pop(key_)
    else:
        logger.info('准备递归下载'+key_)
    list_info = []
    with youtube_dl.YoutubeDL() as ydl:
        try:
            info = ydl.extract_info(url_, download=False)
            for i in info['formats']:
                list_info.append(i['format_id'])
        except youtube_dl.utils.DownloadError:
            logger.debug('%s未开播或读取下载信息失败' % key_)

            # logger.debug('准备补充上传'+key_)
            supplemental_upload(dict, pfile_name_, key_, url_, value_)
            return

    if 'HD-flv' in list_info:
        ydl_opts = {
            'outtmpl': pfile_name_,
            'format': 'HD-flv',
        }
    else:
        ydl_opts = {
            'outtmpl': pfile_name_,
            'format': 'SD-flv',
            # 'keep_fragments':True
            # 'postprocessors': [{
            #     'key': 'FFmpegFixupM3u8',
            # 'preferredcodec': 'mp3',
            # 'preferredquality': '1900',
            # }],
        }
    try:
        with youtube_dl.YoutubeDL(ydl_opts) as ydl:
            pid = os.getpid()
            # print(pid)
            q.put([pid, pfile_name_])

            # if os.path.isfile(pfile_name_):
            #     os.rename(pfile_name_, pfile_name_ + '.part')
            #     logger.info('存在{0}更名为{1}'.format(pfile_name_, pfile_name_ + '.part'))

            logger.info('开始下载pandaTV' + key_)
            ydl.download([url_])

        print('下载完成')
        logger.info('下载完成' + key_)

        with youtube_dl.YoutubeDL() as ydl:
            try:
                info = ydl.extract_info(url_, download=False)
            except youtube_dl.utils.DownloadError:

                uploads(pfile_name_, url_)

                return

            logger.info('实际未下载完成' + key_)

            if os.path.isfile(pfile_name_):
                os.rename(pfile_name_, pfile_name_[:-4] + str(time.time())[:10] + pfile_name_[-4:])
                logger.info('存在{0}更名为{1}'.format(pfile_name_, pfile_name_[:-4] + '时间' + pfile_name_[-4:]))

            download_panda_stream(dict, q, key_, url_, pfile_name_, value_)
    except KeyboardInterrupt:
    # except youtube_dl.utils.DownloadError:
        try:
            # logger.info('更名{0}'.format(pfile_name_+ '.part'))
            os.rename(pfile_name_ + '.part', pfile_name_[:-4] + str(time.time())[:10] + pfile_name_[-4:])
            logger.info('下载进程发生KeyboardInterrupt更名{0}为{1}+时间'.format(pfile_name_ + '.part', pfile_name_))
        except FileExistsError:
            os.rename(pfile_name_ + '.part', pfile_name_[:-4] + str(time.time())[:10] + pfile_name_[-4:])
        download_panda_stream(dict, q, key_, url_, pfile_name_, value_)
    finally:
        dict[key_] = value_
        logger.info('退出下载')


def download_stream(dict, q, confirm_url, key_, twitch_url_, panda_url_, tfile_name_, pfile_name_):
    # print('子进程id%s'%(os.getpid()))
    download_twitch_stream(dict, q, confirm_url, key_, twitch_url_, tfile_name_)
    download_panda_stream(dict, q, key_, panda_url_, pfile_name_)


if __name__ == '__main__':
    get_twitch_stream('https://api.twitch.tv/kraken/streams/1160340','233')
