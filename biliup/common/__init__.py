import logging
import sys

# logging.SafeRotatingFileHandler = SafeRotatingFileHandler
# log_file_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'configlog.ini')
# logging.config.fileConfig(log_file_path)


def new_hook(t, v, tb):
    if issubclass(t, KeyboardInterrupt):
        sys.__excepthook__(t, v, tb)
        return
    logging.getLogger('biliup').error("Uncaught exception:", exc_info=(t, v, tb))


sys.excepthook = new_hook

