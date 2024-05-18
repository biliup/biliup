import asyncio
import logging
from concurrent.futures import ThreadPoolExecutor

from . import plugins
from biliup.config import config
from biliup.engine import Plugin, invert_dict
from biliup.engine.event import EventManager, Event
from .common.timer import Timer
from .common.tools import NamedLock

logger = logging.getLogger('biliup')


def create_event_manager():
    pool1_size = config.get('pool1_size', 5)
    pool2_size = config.get('pool2_size', 3)
    pool = {
        'Asynchronous1': ThreadPoolExecutor(pool1_size, thread_name_prefix='Asynchronous1'),
        'Asynchronous2': ThreadPoolExecutor(pool2_size, thread_name_prefix='Asynchronous2'),
        # 'Asynchronous3': ThreadPoolExecutor(2, thread_name_prefix='Asynchronous3'),
    }
    # 初始化事件管理器
    app = EventManager(config, pool)
    app.context['url_upload_count'] = {}
    # 正在上传的文件 用于同时上传一个url的时候过滤掉正在上传的
    app.context['upload_filename'] = []
    return app


event_manager = create_event_manager()
context = event_manager.context


async def singleton_check(platform, name, url):
    from biliup.handler import PRE_DOWNLOAD, UPLOAD
    context['url_upload_count'].setdefault(url, 0)
    if context['PluginInfo'].url_status[url] == 1:
        logger.debug(f'{url} 正在下载中，跳过检测')
        return
    # 可能对同一个url同时发送两次上传事件
    with NamedLock(f"upload_count_{url}"):
        if context['url_upload_count'][url] > 0:
            logger.debug(f'{url} 正在上传中，跳过')
        else:
            # from .handler import event_manager, UPLOAD
            # += 不是原子操作
            context['url_upload_count'][url] += 1
            event_manager.send_event(Event(UPLOAD, ({'name': name, 'url': url},)))
    if await platform(name, url).acheck_stream(True):
        # 需要等待上传文件列表检索完成后才可以开始下次下载
        with NamedLock(f'upload_file_list_{name}'):
            event_manager.send_event(Event(PRE_DOWNLOAD, args=(name, url,)))


async def shot(event):
    index = 0
    while True:
        if not len(event.url_list):
            logger.info(f"{event}没有任务，退出")
            return
        if index >= len(event.url_list):
            index = 0
            continue
        cur = event.url_list[index]
        try:
            await singleton_check(event, context['PluginInfo'].inverted_index[cur], cur)
            index += 1
            skip = context['PluginInfo'].url_status[cur] == 1 and index < len(event.url_list)
            if skip:  # 在一次 url_list 内，如果 url 正在下载，则跳过本次等待以加快下一个检测
                continue
        except Exception:
            logger.exception('shot')
        await asyncio.sleep(config.get('event_loop_interval', 30))


@event_manager.server()
class PluginInfo:
    def __init__(self, streamers):
        streamer_url = {k: v['url'] for k, v in streamers.items()}
        self.inverted_index = invert_dict(streamer_url)
        urls = list(self.inverted_index.keys())
        self.checker = Plugin(plugins).sorted_checker(urls)
        self.url_status = dict.fromkeys(self.inverted_index, 0)
        self.coroutines = dict.fromkeys(self.checker)
        self.init_tasks()

    def add(self, name, url):
        temp = Plugin(plugins).inspect_checker(url)
        key = temp.__name__
        if key in self.checker:
            self.checker[key].url_list.append(url)
        else:
            temp.url_list = [url]
            self.checker[key] = temp
            from .engine.download import BatchCheck
            if issubclass(temp, BatchCheck):
                # 如果支持批量检测
                self.batch_check_task(temp)
            else:
                self.coroutines[key] = asyncio.create_task(shot(temp))
        self.inverted_index[url] = name
        self.url_status[url] = 0

    def delete(self, url):
        if not url in self.inverted_index:
            return
        del self.inverted_index[url]
        exec_del = False
        for key, value in self.checker.items():
            if url in value.url_list:
                if len(value.url_list) == 1:
                    exec_del = key
                else:
                    value.url_list.remove(url)
        if exec_del:
            del self.checker[exec_del]
            self.coroutines[exec_del].cancel()
            del self.coroutines[exec_del]

    def init_tasks(self):
        from .engine.download import BatchCheck

        for key, plugin in self.checker.items():
            if issubclass(plugin, BatchCheck):
                # 如果支持批量检测
                self.batch_check_task(plugin)
                continue
            self.coroutines[key] = asyncio.create_task(shot(plugin))

    def batch_check_task(self, plugin):
        from biliup.handler import PRE_DOWNLOAD

        async def check_timer():
            name = None
            # 如果支持批量检测
            try:
                async for turl in plugin.abatch_check(plugin.url_list):
                    context['url_upload_count'].setdefault(turl, 0)
                    for k, v in config['streamers'].items():
                        if v.get("url", "") == turl:
                            name = k
                    event_manager.send_event(Event(PRE_DOWNLOAD, args=(name, turl,)))
            except Exception:
                logger.exception('batch_check_task')

        timer = Timer(func=check_timer, interval=30)
        self.coroutines[plugin.__name__] = asyncio.create_task(timer.astart())
