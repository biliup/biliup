from datetime import datetime, timedelta, timezone
from multiprocessing import Process, Manager, Queue
from download import links_id, root_url, get_twitch_stream, download_stream, download_twitch_stream, monitoring, download_panda_stream
import time

if __name__ == '__main__':
    manager = Manager()
    d = manager.dict(links_id)

    queue = Queue()
    p0 = Process(target=monitoring, args=(queue,))
    p0.start()
    while True:
        utc_dt = datetime.utcnow().replace(tzinfo=timezone.utc)
        bj_dt = utc_dt.astimezone(timezone(timedelta(hours=8)))
        now = bj_dt.strftime('%Y{0}%m{1}%d{2}').format(*'年月日')

        for key in d.copy():
            # print(key)
            file_name = '%s%s.mp4' % (key, now)
            if len(links_id[key]) == 2:
                twitch_url = root_url[0]+links_id[key][0]
                panda_url = root_url[1]+links_id[key][1]
                if links_id[key][0] != '':
                    confirm_url = root_url[2]+links_id[key][0]
                    status = get_twitch_stream(confirm_url,key)
                # print(status)
                # print('父进程id%s'%(os.getpid()))
                    p = Process(target=download_stream, args=(d, queue, status,key,twitch_url,panda_url,file_name))
                    p.start()
                else:
                    p = Process(target=download_panda_stream,args=(d, queue, key, panda_url, file_name))

            elif len(links_id[key]) ==1:
                twitch_url = root_url[0]+links_id[key][0]
                confirm_url = root_url[2] + links_id[key][0]
                status = get_twitch_stream(confirm_url, key)
                p = Process(target=download_twitch_stream, args=(d, queue, status, key,twitch_url,file_name))
                p.start()
            time.sleep(30)