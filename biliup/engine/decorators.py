import functools
import importlib
import pkgutil
import re


def suit_url(pattern, urls):
    sorted_url = []
    for i in range(len(urls) - 1, -1, -1):
        if re.match(pattern, urls[i]):
            sorted_url.append(urls[i])
            urls.remove(urls[i])
    return sorted_url


class Plugin:
    download_plugins = []
    upload_plugins = {}

    def __init__(self, pkg):
        self.load_plugins(pkg)

    @staticmethod
    def download(regexp):
        def decorator(cls):
            @functools.wraps(cls)
            def wrapper(*args, **kw):
                return cls(*args, **kw)
            wrapper.VALID_URL_BASE = regexp
            Plugin.download_plugins.append(wrapper)
            return wrapper
        return decorator

    @staticmethod
    def upload(platform):
        def decorator(cls):
            Plugin.upload_plugins[platform] = cls
            return cls
        return decorator

    @classmethod
    def sorted_checker(cls, urls):
        from ..plugins import general
        curls = urls.copy()
        checker_plugins = {}
        for plugin in cls.download_plugins:
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
        return checker_plugins

    def load_plugins(self, pkg):
        """Attempt to load plugins from the path specified.
        engine.plugins.__path__[0]: full path to a directory where to look for plugins
        """

        plugins = []

        for loader, name, ispkg in pkgutil.iter_modules([pkg.__path__[0]]):
            # set the full plugin module name
            module_name = f"{pkg.__name__}.{name}"
            module = importlib.import_module(module_name)
            if ispkg:
                self.load_plugins(module)
                continue
            if module in plugins:
                continue
            plugins.append(module)
        return plugins
