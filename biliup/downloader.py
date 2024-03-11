import logging
import re
import threading
import time
from urllib.error import HTTPError

from biliup.config import config
from .app import context
from .common.tools import NamedLock
from .engine.decorators import Plugin
from .engine.download import DownloadBase
from .engine.event import Event
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
