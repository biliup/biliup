import asyncio

from .common.reload import AutoReload
from .common.timer import Timer

from .engine.event import EventManager, Event
from .engine import config, invert_dict, Plugin
from . import plugins


def create_event_manager():
    streamer_url = {k: v['url'] for k, v in config['streamers'].items()}
    inverted_index = invert_dict(streamer_url)
    urls = list(inverted_index.keys())
    # 初始化事件管理器
    app = EventManager(config)
    app.context['urls'] = urls
    app.context['url_status'] = dict.fromkeys(inverted_index, 0)
    app.context['checker'] = Plugin(plugins).sorted_checker(urls)
    app.context['inverted_index'] = inverted_index
    app.context['streamer_url'] = streamer_url
    return app


event_manager = create_event_manager()


async def main():
    from .handler import CHECK_UPLOAD, CHECK

    event_manager.start()

    async def check_timer():
        event_manager.send_event(Event(CHECK_UPLOAD))
        for k in event_manager.context['checker'].keys():
            event_manager.send_event(Event(CHECK, (k,)))

    # 初始化定时器
    timer = Timer(func=check_timer, interval=40)

    # 模块更新自动重启
    detector = AutoReload(event_manager, timer, interval=15)
    await asyncio.gather(detector.astart(), timer.astart(), return_exceptions=True)
