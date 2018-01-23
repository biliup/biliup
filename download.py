import signal
from upload import upload, get_file
from datetime import datetime, timedelta, timezone
from multiprocessing import Process,Manager,Queue
import os
import asyncio
import psutil
import requests
import json
import time
import youtube_dl


# links = ['www.twitch.tv/innovation_s2','www.panda.tv/1160340','www.twitch.tv/sc2soo','www.panda.tv/1150595','www.twitch.tv/kimdaeyeob3']

root_url = ['www.twitch.tv/','www.panda.tv/','https://api.twitch.tv/kraken/streams/']

links_id = {
    '星际2innovation吕布卫星人族天梯第一视角': ['innovation_s2','1160340'],
    '星际2soo输本虫族天梯第一视角': ['sc2soo','1150595'],
    '星际2stats拔本神族天梯第一视角': ['kimdaeyeob3'],
    '星际2sos狗哥神族天梯第一视角':['', '1160930']
    # 'test':['maru072','10027'],
    # 'test1':['mcanning','70000']
         }

headers = {
    'client-id':'jzkbprff40iqj646a697cyrvl0zt2m6'
           }


# class SizeError(Exception):
#     def __init__(self, info):
#         Exception.__init__(self)
#         self.errorinfo = info
#
#     def __str__(self):
#         return "SizeError:%s" % (self.errorinfo)


def kill_child_processes(parent_pid, file_name_, sig=signal.SIGINT):
    file_name_ = file_name_+'.part'
    while True:
        time.sleep(10)
        if os.path.isfile(file_name_):
            file_size = os.path.getsize(file_name_)/1024/1024/1024
            if float(file_size) >= 3.9:
                try:
                    parent = psutil.Process(parent_pid)
                except psutil.NoSuchProcess:
                    return
                children = parent.children(recursive=True)
                for process in children:
                    # print(process)
                    process.send_signal(sig)
                break
            # print(file_name_, '文件大小', file_size) # 单位GB
        # try:
        #     parent = psutil.Process(parent_pid)
        # except psutil.NoSuchProcess:
        #     return
        # children = parent.children(recursive=True)
        # for process in children:
        #     print(process)
        #     # process.send_signal(sig)


def monitoring(q):
    while True:
        # print('开始监测')
        pid,file_name = q.get()
        time.sleep(1)
        # print('获取到{0}，{1}'.format(pid,file_name))
        p = Process(target=kill_child_processes, args=(pid, file_name))
        p.start()

# def download_part():
#     try:
#         with youtube_dl.YoutubeDL(ydl_opts) as ydl:
#             ydl.download([url_])
#     except SizeError:
#
# async def download(file_name_,url_, ydl_):
#     if os.path.isfile(file_name_):
#         file_size = os.path.getsize(file_name_)
#         print('文件大小',file_size)
#     print(os.path.isfile(file_name_))
#     await asyncio.sleep(3)#ydl_.download([url_])
#
#
# async def file_size(file_name_):
#     file_size_ = os.path.getsize(file_name_)
#     print('文件大小', file_size_)


# def is_err(file_name_):
#     if os.path.getsize(file_name_) / 1024 / 1024 >= 5:
#         raise SizeError('文件大小超出限制')


def get_twitch_stream (url,value):
    res = requests.get(url, headers=headers)
    s = json.loads(res.text)
    print(value,s['stream'])
    return s['stream']


def download_twitch_stream(dict, q, status_, key_,url_, file_name_):
    # print(status_)
    if str(status_) != 'None':
        value_ = dict[key_]
        dict.pop(key_)
        print('开始下载',key_)
        ydl_opts = {
            'outtmpl':file_name_,
            'format': '720p'
            # 'keep_fragments':True
            # 'postprocessors': [{
            #     'key': 'FFmpegFixupM3u8',
                # 'preferredcodec': 'mp3',
                # 'preferredquality': '1900',
            # }],
        }
        try:
            with youtube_dl.YoutubeDL(ydl_opts) as ydl:
                # loop = asyncio.get_event_loop()
                # tasks = [download(file_name_+'.part',url_,ydl),file_size(file_name_+'.part')]
                # loop.run_until_complete(download(file_name_+'.part',url_,ydl))
                # loop.run_until_complete(asyncio.wait(tasks))
                # loop.close()
                pid = os.getpid()
                q.put([pid, file_name_])

                ydl.download([url_])
        except youtube_dl.utils.DownloadError:

            print('分段下载')
            try:
                os.rename(file_name_ + '.part', file_name_[:-4] + str(time.time())[:10] + '.mp4')
            except FileExistsError:
                os.rename(file_name_ + '.part', file_name_[:-4] + str(time.time())[:10] + '.mp4')
        finally:
            dict[key_] = value_
        print('下载完成')
        os.rename(file_name_, file_name_[:-4] + str(time.time())[:10] + '.mp4')
        file_list = get_file(file_name_)
        videopath = ''
        root = os.getcwd()
        for i in range(len(file_list)):
            file = file_list[i]
            videopath += root + '/' + file + '\n'
        videopath = videopath.rstrip()
        upload(video_path=videopath,link=url_, title_=file_name_[:-4])

        for r in file_list:
            os.remove(r)
        # print(links_id)


def download_panda_stream(dict, q, key_, url_,file_name_):
    value_ = dict[key_]
    dict.pop(key_)
    print('开始下载', key_)
    list_info = []
    with youtube_dl.YoutubeDL() as ydl:
        try:
            info = ydl.extract_info(url_, download=False)
            for i in info['formats']:
                list_info.append(i['format_id'])
        except youtube_dl.utils.DownloadError:
            dict[key_] = value_
    if 'HD-m3u8' in list_info:
        ydl_opts = {
            'outtmpl': file_name_,
            'format': 'HD-m3u8',
        }
    else:
        ydl_opts = {
            'outtmpl': file_name_,
            'format': 'SD-m3u8',
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
            print(pid)
            q.put([pid, file_name_])

            ydl.download([url_])
    except youtube_dl.utils.DownloadError:
        print('分段下载')
        try:
            os.rename(file_name_ + '.part', file_name_[:-4] + str(time.time())[:10] + '.mp4')
        except FileExistsError:
            os.rename(file_name_ + '.part', file_name_[:-4] + str(time.time())[:10] + '.mp4')
    finally:
        dict[key_] = value_
    print('下载完成')
    os.rename(file_name_, file_name_[:-4] + str(time.time())[:10] + '.mp4')
    file_list = get_file(file_name_)
    videopath = ''
    root = os.getcwd()
    for i in range(len(file_list)):
        file = file_list[i]
        videopath += root + '/' + file + '\n'
    videopath = videopath.rstrip()
    upload(video_path=videopath, link=url_, title_=file_name_[:-4])

    for r in file_list:
        os.remove(r)
    # print(links_id)


def download_stream(dict, q, status_, key_,twitch_url_,panda_url_,file_name_):
    # print('子进程id%s'%(os.getpid()))
    download_twitch_stream(dict, q, status_, key_,twitch_url_, file_name_)
    download_panda_stream(dict, q, key_, panda_url_, file_name_)

if __name__ == '__main__':
    manager = Manager()
    # d = manager.dict(links_id)
    # while True:
    #     utc_dt = datetime.utcnow().replace(tzinfo=timezone.utc)
    #     bj_dt = utc_dt.astimezone(timezone(timedelta(hours=8)))
    #     now = bj_dt.strftime('%Y{0}%m{1}%d{2}').format(*'年月日')
    #
    #     for key in d.copy():
    #         # print(key)
    #         file_name = '%s%s.mp4' % (key, now)
    #         if len(links_id[key]) == 2:
    #             twitch_url = root_url[0]+links_id[key][0]
    #             panda_url = root_url[1]+links_id[key][1]
    #             confirm_url = root_url[2]+links_id[key][0]
    #             status = get_twitch_stream(confirm_url,key)
    #             # print(status)
    #             # print('父进程id%s'%(os.getpid()))
    #             p = Process(target=download_stream, args=(d,status,key,twitch_url,panda_url,file_name))
    #             p.start()
    #
    #         elif len(links_id[key]) ==1:
    #             twitch_url = root_url[0]+links_id[key][0]
    #             confirm_url = root_url[2] + links_id[key][0]
    #             status = get_twitch_stream(confirm_url, key)
    #             p = Process(target=download_twitch_stream, args=(d,status, key,twitch_url,file_name))
    #             p.start()
    #         time.sleep(5)
    # print('test2018年01月22日0473.mp4')
    # try:
    #     os.rename('1','2')
    # except FileExistsError:
    #     print('test2018年01月22日.mp4')
    # root = os.getcwd()
    # print(root)
    # print(os.listdir())
    # get_twitch_stream('https://api.twitch.tv/kraken/streams/',1)
    # a = 'test2018年01月17日.mp4'
    # print(a[-4:])
    # a = ['111','2']
    # if '11' in a:
    #     print('yes')
    # else:
    #     print('88')