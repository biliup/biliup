import logging
import re
import time
from urllib.error import HTTPError

from .engine.decorators import Plugin

from .engine.download import DownloadBase
from .engine.event import Event
from .engine.upload import UploadBase
from .plugins import general
from biliup.config import config

logger = logging.getLogger('biliup')


def download(fname, url, **kwargs):
    pg = general.__plugin__(fname, url)
    for plugin in Plugin.download_plugins:
        if re.match(plugin.VALID_URL_BASE, url):
            pg = plugin(fname, url)
            for k in pg.__dict__:
                if kwargs.get(k):
                    pg.__dict__[k] = kwargs.get(k)
            break
    return pg.start()


def check_url(checker):
    from .handler import event_manager, TO_MODIFY, UPLOAD
    # 单主播检测延迟
    checker_sleep = config.get('checker_sleep', 15)
    # 平台检测延迟
    event_loop_interval = config.get('event_loop_interval', 40)
    context = event_manager.context
    class_reference = type(checker('', ''))
    while True:
        try:
            # 待检测url
            check_urls = []
            # 过滤url
            for url in checker.url_list:
                if context['url_status'][url] == 1:
                    logger.debug(f'{url}正在下载中，跳过检测')
                    continue

                # 检测之前可能未上传的视频
                if len(UploadBase.file_list(context['inverted_index'][url])) > 0:
                    event_manager.send_event(
                        Event(UPLOAD, args=({'name': context['inverted_index'][url], 'url': url},)))
                    # 这里不能直接修改url_upload_count 因为UPLOAD Event中有url_upload_count的修改
                    # 所以单独判断
                    if config.get('uploading_record', False):
                        logger.debug(f'{url}正在上传中，跳过检测')
                        continue
                else:
                    # 没有等待上传的视频
                    if context['url_upload_count'][url] > 0 and not config.get('uploading_record', False):
                        logger.debug(f'{url}正在上传中，跳过检测')
                        continue
                check_urls.append(url)

            if DownloadBase.batch_check != getattr(class_reference, DownloadBase.batch_check.__name__):
                # 如果支持批量检测
                # 发送下载的事件
                for url in class_reference.batch_check(check_urls):
                    event_manager.send_event(Event(TO_MODIFY, args=(url,)))
            else:
                # 不支持批量检测
                for (index, url) in enumerate(check_urls):
                    # 某个检测异常略过不应影响其他检测
                    try:
                        if index > 0:
                            logger.debug('歇息会')
                            time.sleep(checker_sleep)

                        if checker(context['inverted_index'][url], url).check_stream(True):
                            event_manager.send_event(Event(TO_MODIFY, args=(url,)))
                    except HTTPError as e:
                        logger.error(f'{checker.__module__} {e.url} => {e}')
                    except IOError:
                        logger.exception("IOError")
                    except:
                        logger.exception("Uncaught exception:")



        except:
            # 除了单个检测异常 其他异常会影响整体 直接略过本次 等待下次整体检测
            logger.exception("Uncaught exception:")

        time.sleep(event_loop_interval)
