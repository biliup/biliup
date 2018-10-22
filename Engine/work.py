import re
from datetime import datetime, timezone, timedelta
import yaml
from common import logger


def time_now():
    utc_dt = datetime.utcnow().replace(tzinfo=timezone.utc)
    bj_dt = utc_dt.astimezone(timezone(timedelta(hours=8)))
    # now = bj_dt.strftime('%Y{0}%m{1}%d{2}').format(*'...')
    now = bj_dt.strftime('%Y{0}%m{1}%d').format(*'..')
    return now


def match1(text, *patterns):
    if len(patterns) == 1:
        pattern = patterns[0]
        match = re.search(pattern, text)
        if match:
            return match.group(1)
        else:
            return None
    else:
        ret = []
        for pattern in patterns:
            match = re.search(pattern, text)
            if match:
                ret.append(match.group(1))
        return ret


def new_hook(t, v, tb):
    logger.error("Uncaught exception：", exc_info=(t, v, tb))


# 用来包装需要多进程的函数（多进程执行避免主进程阻塞）
class Service:
    def __init__(self, pool, func, callback):
        # 事件处理进程池
        self.__pool = pool
        self.__func = func
        self.callback = callback

    def __run(self, args):
        self.__pool.apply_async(func=self.__func, args=args, callback=self.callback)
        # self.__pool.apply(func=self.__func, args=args)

    def start(self, event):
        args = event.args
        self.__run(args)

    def stop(self):
        self.__pool.close()
        self.__pool.join()


def find_name(url):
    for name in links_id:
        if url in links_id[name]:
            return name


def getmany():
    urls = []
    urlstatus = {}
    for k, v in links_id.items():
        urls += v
        for url in v:
            urlstatus[url] = 0
    return urls, urlstatus, urlstatus.copy()


with open(r'config.yaml', encoding='utf-8') as stream:
    config = yaml.load(stream)
    links_id = config['links_id']
    user_name = config['user_name']
    pass_word = config['pass_word']
    chromedrive_path = config['chromedrive_path']

Urls, url_status, url_status_base = getmany()
