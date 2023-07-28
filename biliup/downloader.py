import logging
import re
import time
from urllib.error import HTTPError

from .config import config
from .engine.decorators import Plugin

from .engine.download import DownloadBase
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


def check_url(plugin, url_status, url_upload_count, secs=15):
    try:
        # 待检测url
        check_urls = []
        # 过滤url
        for url in plugin.url_list:
            if url_status[url] == 1:
                logger.debug(f'{url}正在下载中，跳过检测')
                continue
            if url_upload_count[url] > 0 and not config.get('uploading_record', False):
                logger.debug(f'{url}正在上传中，跳过检测')
                continue
            check_urls.append(url)

        if DownloadBase.batch_check != getattr(plugin.static_class, DownloadBase.batch_check.__name__):
            # 如果支持批量检测
            yield from plugin.static_class.batch_check(check_urls)
        else:
            # 不支持批量检测
            for url in check_urls:
                from .handler import event_manager
                if plugin(event_manager.context['inverted_index'][url], url).check_stream(True):
                    yield url
                if url != check_urls[-1]:
                    logger.debug('歇息会')
                    time.sleep(secs)

    except HTTPError as e:
        logger.error(f'{plugin.__module__} {e.url} => {e}')
    except IOError:
        logger.exception("IOError")
    except:
        logger.exception("Uncaught exception:")
