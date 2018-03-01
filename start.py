import logging.config
logging.config.fileConfig('configlog.ini')
if __name__ == '__main__':
    from datetime import datetime, timedelta, timezone
    from multiprocessing import Pool,Manager
    from download import *
    from work import *
    import sys
    # logging.basicConfig(filename='AUTO.log', level=logging.INFO)
    # logging.basicConfig(format='%(asctime)s %(filename)s[line:%(lineno)d] %(levelname)s %(message)s',\
    #                     datefmt='%a, %d %b %Y %H:%M',level=logging.INFO)

    sys.excepthook = new_hook

    manager = Manager()
    d = manager.dict(links_id)

    queue = manager.Queue()
    p0 = Process(target=monitoring, args=(queue,))
    p0.start()

    pool = Pool(processes=3)

    # signal.signal(signal.SIGCHLD, wait_child)

    while True:
        utc_dt = datetime.utcnow().replace(tzinfo=timezone.utc)
        bj_dt = utc_dt.astimezone(timezone(timedelta(hours=8)))
        now = bj_dt.strftime('%Y{0}%m{1}%d{2}').format(*'年月日')

        for key in d.copy():
            # print(key)
            tfile_name = '%s%s.mp4' % (key, now)
            pfile_name = '%s%s.flv' % (key, now)
            if len(links_id[key]) == 2:
                twitch_url = root_url[0] + links_id[key][0]
                panda_url = root_url[1] + links_id[key][1]
                if links_id[key][0] != '':
                    confirm_url = root_url[2] + links_id[key][0]
                    # status = get_twitch_stream(confirm_url, key)
                    # print(status)
                    # print('父进程id%s'%(os.getpid()))
                    res = pool.apply_async(func=download_stream,
                                           args=(d, queue, confirm_url, key, twitch_url, panda_url, tfile_name, pfile_name))
                    # p = Process(target=download_stream, args=(d, queue, status,key,twitch_url,panda_url,tfile_name, pfile_name))
                    # p.start()
                else:
                    res = pool.apply_async(func=download_panda_stream,
                                           args=(d, queue, key, panda_url, pfile_name))
                    # p = Process(target=download_panda_stream,args=(d, queue, key, panda_url, pfile_name))
                    # p.start()

            elif len(links_id[key]) == 1:
                twitch_url = root_url[0] + links_id[key][0]
                confirm_url = root_url[2] + links_id[key][0]
                # status = get_twitch_stream(confirm_url, key)
                res = pool.apply_async(func=download_twitch_stream,
                                       args=(d, queue, confirm_url, key, twitch_url, tfile_name))
                # p = Process(target=download_twitch_stream, args=(d, queue, status, key,twitch_url,tfile_name))
                # p.start()
            # print(res.get())
            time.sleep(50)

    # pool.close()
    # pool.join()
