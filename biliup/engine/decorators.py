import functools
import importlib
import pkgutil


class Plugin:
    upload_plugins = {}

    def __init__(self, pkg):
        self.load_plugins(pkg)

    @staticmethod
    def upload(platform):
        def decorator(cls):
            @functools.wraps(cls)
            def wrapper(*args, **kw):
                print(f"args {args}")
                print(f"kw {kw}")
                return cls(*args, **kw)
            Plugin.upload_plugins[platform] = wrapper
            return wrapper
        return decorator

    def load_plugins(self, pkg):
        """Attempt to load plugins from the path specified.
        engine.plugins.__path__[0]: full path to a directory where to look for plugins
        """

        plugins = []

        for loader, name, ispkg in pkgutil.iter_modules([pkg.__path__[0]]):
            module_name = f"{pkg.__name__}.{name}"
            module = importlib.import_module(module_name)
            if ispkg:
                self.load_plugins(module)
                continue
            if module in plugins:
                continue
            plugins.append(module)
        return plugins
