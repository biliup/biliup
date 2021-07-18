import logging
import re
import time
from urllib.error import HTTPError

from .engine.decorators import Plugin
from .plugins import general, BatchCheckBase

logger = logging.getLogger('biliup')


def download(fname, url, **kwargs):
    for plugin in Plugin.download_plugins:
        if re.match(plugin.VALID_URL_BASE, url):
            pg = plugin(fname, url)
            for k in pg.__dict__:
                if kwargs.get(k):
                    pg.__dict__[k] = kwargs.get(k)
            pg.start()
            return
    general.__plugin__(fname, url).start()


def check_url(plugin, secs=15):
    try:
        if isinstance(plugin, BatchCheckBase):
            return (yield from plugin.check())
        for url in plugin.url_list:
            if plugin(f'检测{url}', url).check_stream():
                yield url
            if url != plugin.url_list[-1]:
                logger.debug('歇息会')
                time.sleep(secs)
    except HTTPError as e:
        logger.error(f'{plugin.__module__} {e.url} => {e}')
    except IOError:
        logger.exception("IOError")
    except:
        logger.exception("Uncaught exception:")
