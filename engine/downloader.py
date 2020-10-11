import re
import time
from functools import reduce

from common import logger
from common.decorators import Plugin
from engine.plugins import general

batches = []
onebyone = []


def suit_url(pattern, urls):
    sorted_url = []
    for i in range(len(urls) - 1, -1, -1):
        if re.match(pattern, urls[i]):
            sorted_url.append(urls[i])
            urls.remove(urls[i])
    return sorted_url


def sorted_checker(urls):
    curls = urls.copy()
    if batches or onebyone:
        return batches, onebyone
    for plugin in Plugin.download_plugins:
        plugin.url_list = suit_url(plugin.VALID_URL_BASE, curls)
        if hasattr(plugin, "BatchCheck"):
            batches.append(plugin.BatchCheck(plugin.url_list))
        else:
            onebyone.append(plugin)
    general.__plugin__.url_list = curls
    onebyone.append(general.__plugin__)
    # onebyone.append(__import__('engine.plugins.general', fromlist=['general',]))
    return batches, onebyone


def download(fname, url):
    for plugin in Plugin.download_plugins:
        if re.match(plugin.VALID_URL_BASE, url):
            plugin(fname, url).run()
            return
    general.__plugin__(fname, url).run()


def batch_check(live, plugin):
    try:
        res = plugin.check()
        if res:
            live.extend(res)
    except IOError:
        logger.exception("IOError")
    finally:
        return live


def single_check(live, plugin):
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


def check(url_list, mode="ALL"):
    batch, single = sorted_checker(url_list)
    if mode == "batch":
        live = reduce(batch_check, batch, [])
    elif mode == "single":
        live = reduce(single_check, single, [])
    else:
        live = reduce(batch_check, batch, [])
        live = reduce(single_check, single, live)
    return live
