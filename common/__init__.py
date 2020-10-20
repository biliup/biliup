import importlib
import os
import pkgutil
from datetime import datetime, timezone, timedelta
import logging.config


def time_now():
    utc_dt = datetime.utcnow().replace(tzinfo=timezone.utc)
    bj_dt = utc_dt.astimezone(timezone(timedelta(hours=8)))
    # now = bj_dt.strftime('%Y{0}%m{1}%d{2}').format(*'...')
    now = bj_dt.strftime('%Y.%m.%d')
    return now


def new_hook(t, v, tb):
    logger.error("Uncaught exception:", exc_info=(t, v, tb))


# @singleton
def load_plugins():
    """Attempt to load plugins from the path specified.
    engine.plugins.__path__[0]: full path to a directory where to look for plugins
    """
    import engine.plugins

    plugins = []

    for loader, name, ispkg in pkgutil.iter_modules([engine.plugins.__path__[0]]):
        # set the full plugin module name
        module_name = "engine.plugins.{0}".format(name)
        # print(module_name)
        module = importlib.import_module(module_name)
        if module in plugins:
            continue
        plugins.append(module)
        # self.load_plugin(module_name)
    # print(self.plugins)
    return plugins


load_plugins()


log_file_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'configlog.ini')
logging.config.fileConfig(log_file_path)
logger = logging.getLogger('log01')
