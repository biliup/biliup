import logging
import re

from .engine.decorators import Plugin
from .plugins import general

logger = logging.getLogger('biliup')


def download(fname, url, **kwargs):
    pg = None
    for plugin in Plugin.download_plugins:
        if re.match(plugin.VALID_URL_BASE, url):
            pg = plugin(fname, url)
            for k in pg.__dict__:
                if kwargs.get(k):
                    pg.__dict__[k] = kwargs.get(k)
            break
    if not pg:
        pg = general.__plugin__(fname, url)
        logger.warning(f'Not found plugin for {fname} -> {url} This may cause problems')
    return pg.start()


def biliup_download(name, url, kwargs: dict):
    kwargs.pop('url')
    suffix = kwargs.get('format')
    if suffix:
        kwargs['suffix'] = suffix
    return download(name, url, **kwargs)