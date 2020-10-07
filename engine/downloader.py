import re
from common.decorators import Plugin
from engine.plugins import general


def suit_url(pattern, urls):
    sorted_url = []
    for i in range(len(urls) - 1, -1, -1):
        if re.match(pattern, urls[i]):
            sorted_url.append(urls[i])
            urls.remove(urls[i])
    return sorted_url


def sorted_checker(urls):
    curls = urls.copy()
    batches = []
    onebyone = []
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


# def load_download_plugin():
#     # Set the global http session for this plugin
#     module = importlib.import_module(name)
#     plugins = []
#     for module in PLUGINS:
#         if hasattr(module, "download_plugin"):
#             # module_name = getattr(module, "__name__")
#             # plugin_name = module_name.split(".")[-1]  # get the plugin part of the module name
#             if module in plugins:
#                 print('存在')
#                 continue
#             plugins.append(module)
#     return Plugin.download_plugins
