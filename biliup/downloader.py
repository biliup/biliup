import logging
import re

from .engine.decorators import Plugin
from .plugins import general

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


def biliup_download(name, url, kwargs: dict):
    kwargs.pop('url')
    suffix = kwargs.get('format')
    if suffix:
        kwargs['suffix'] = suffix
    return download(name, url, **kwargs)