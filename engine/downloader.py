import importlib
import logging
import pkgutil
import re
import time
import engine.plugins

from common.decorators import Plugin
from engine.plugins import general
logger = logging.getLogger('log01')


def load_plugins(pkg=engine.plugins):
    """Attempt to load plugins from the path specified.
    engine.plugins.__path__[0]: full path to a directory where to look for plugins
    """

    plugins = []

    for loader, name, ispkg in pkgutil.iter_modules([pkg.__path__[0]]):
        # set the full plugin module name
        module_name = f"{pkg.__name__}.{name}"
        module = importlib.import_module(module_name)
        if ispkg:
            load_plugins(module)
            continue
        if module in plugins:
            continue
        plugins.append(module)
        # self.load_plugin(module_name)
    # print(self.plugins)
    return plugins


load_plugins()


def suit_url(pattern, urls):
    sorted_url = []
    for i in range(len(urls) - 1, -1, -1):
        if re.match(pattern, urls[i]):
            sorted_url.append(urls[i])
            urls.remove(urls[i])
    return sorted_url


def sorted_checker(urls):
    curls = urls.copy()
    checker_plugins = {}
    for plugin in Plugin.download_plugins:
        url_list = suit_url(plugin.VALID_URL_BASE, curls)
        if not url_list:
            continue
        elif hasattr(plugin, "BatchCheck"):
            checker_plugins[plugin.__name__] = plugin.BatchCheck(url_list)
        else:
            plugin.url_list = url_list
            checker_plugins[plugin.__name__] = plugin
        if not curls:
            return checker_plugins
    general.__plugin__.url_list = curls
    checker_plugins[general.__plugin__.__name__] = general.__plugin__
    # onebyone.append(__import__('engine.plugins.general', fromlist=['general',]))
    return checker_plugins


def download(fname, url):
    for plugin in Plugin.download_plugins:
        if re.match(plugin.VALID_URL_BASE, url):
            plugin(fname, url).run()
            return
    general.__plugin__(fname, url).run()


def batch_check(plugin):
    live = []
    try:
        res = plugin.check()
        if res:
            live.extend(res)
    except IOError:
        logger.exception("IOError")
    finally:
        return live


def singleton_check(plugin):
    live = []
    try:
        for url in plugin.url_list:
            if plugin(f'检测{url}', url).check_stream():
                live.append(url)
            if url != plugin.url_list[-1]:
                logger.debug('歇息会')
                time.sleep(15)
    except IOError:
        logger.exception("IOError")
    finally:
        return live
