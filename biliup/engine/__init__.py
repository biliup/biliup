import asyncio

import yaml

from ..common.decorators import Plugin
from ..common.event import Event, event_manager
from ..common.reload import AutoReload
from ..common.timer import Timer

with open(r'config.yaml', encoding='utf-8') as stream:
    config = yaml.load(stream, Loader=yaml.FullLoader)


def invert_dict(d: dict):
    inverse_dict = {}
    for k, v in d.items():
        for item in v:
            inverse_dict[item] = k
    return inverse_dict


async def main():
    from . import plugins
    streamers = config['streamers']
    streamer_url = {k: v['url'] for k, v in streamers.items()}
    inverted_index = invert_dict(streamer_url)
    urls = list(inverted_index.keys())
    url_status = dict.fromkeys(inverted_index, 0)
    checker = Plugin(plugins).sorted_checker(urls)
    # 初始化事件管理器
    event_manager.context = {**config, 'urls': urls, 'url_status': url_status,
                             'checker': checker, 'inverted_index': inverted_index, 'streamer_url': streamer_url}
    from ..engine.handler import CHECK_UPLOAD, CHECK
    event_manager.start()

    async def check_timer():
        event_manager.send_event(Event(CHECK_UPLOAD))
        for k in checker.keys():
            event_manager.send_event(Event(CHECK, (k,)))

    # 初始化定时器
    timer = Timer(func=check_timer, interval=40)

    # 模块更新自动重启
    detector = AutoReload(event_manager, timer, interval=15)
    await asyncio.gather(detector.astart(), timer.astart(), return_exceptions=True)


__all__ = ['downloader', 'uploader', 'plugins', 'main']
