import inspect
import logging
import time

from biliup.config import config
from .engine.decorators import Plugin

logger = logging.getLogger('biliup')


def upload(data):
    """
    上传入口
    :param platform:
    :param index:
    :param data: 现在需包含内容{url,date} 完整包含内容{url,date,format_title}
    :return:
    """
    try:
        index = data['name']
        context = {**config, **config['streamers'][index]}
        platform = context.get("uploader", "biliup-rs")
        cls = Plugin.upload_plugins.get(platform)
        if cls is None:
            return logger.error(f"No such uploader: {platform}")

        date = data.get("date", time.localtime())
        room_title = data.get('title', index)
        data["format_title"] = custom_fmtstr(context.get('title', f'%Y.%m.%d{index}'), date, room_title)

        if context.get('description'):
            context['description'] = custom_fmtstr(context.get('description'), date, room_title)
        threshold = config.get('filtering_threshold')
        if threshold:
            data['threshold'] = threshold

        sig = inspect.signature(cls)
        kwargs = {}
        for k in sig.parameters:
            v = context.get(k)
            if v:
                kwargs[k] = v
        return cls(index, data, **kwargs).start()
    except:
        logger.exception("Uncaught exception:")


def custom_fmtstr(string, date, title):
    return time.strftime(string.encode('unicode-escape').decode(), date).encode().decode("unicode-escape").format(title=title)
