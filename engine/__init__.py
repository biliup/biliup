import yaml
from common.event import Event
from common.reload import autoreload
from common.timer import Timer
from engine.downloader import sorted_checker

with open(r'config.yaml', encoding='utf-8') as stream:
    config = yaml.load(stream, Loader=yaml.FullLoader)

streamers = config['streamers']
chromedriver_path = config.get('chromedriver_path')


def invert_dict(d: dict):
    inverse_dict = {}
    for k, v in d.items():
        for item in v:
            inverse_dict[item] = k
    return inverse_dict


streamer_url = {k: v['url'] for k, v in streamers.items()}
inverted_index = invert_dict(streamer_url)
urls = list(inverted_index.keys())
url_status = dict.fromkeys(inverted_index, 0)
checker = sorted_checker(urls)
platforms = checker.keys()

context = {**config, "urls": urls, "url_status": url_status}


def check_timer(event_manager):
    event_manager.send_event(Event(CHECK_UPLOAD))
    for k in platforms:
        event_manager.send_event(Event(CHECK, (k,)))


def main(event_manager):
    # 初始化定时器
    timer = Timer(func=check_timer, args=(event_manager,), interval=40)

    # 模块更新自动重启
    autoreload(event_manager, timer, interval=15)

    event_manager.start()
    timer.start()


CHECK = 'check'
CHECK_UPLOAD = 'check_upload'
TO_MODIFY = 'to_modify'
DOWNLOAD = 'download'
BE_MODIFIED = 'be_modified'
UPLOAD = 'upload'
__all__ = ['downloader', 'uploader', 'plugins', 'main',
           'context', 'inverted_index', 'checker',
           'CHECK', 'BE_MODIFIED', 'DOWNLOAD', 'TO_MODIFY', 'UPLOAD', 'CHECK_UPLOAD']
