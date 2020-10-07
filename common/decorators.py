import functools


class Plugin:
    download_plugins = []
    upload_plugins = {}

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
