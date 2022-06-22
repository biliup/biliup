import inspect
import logging
from biliup.config import config
from biliup import common
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
        # platform = context.get("uploader") if context.get("uploader") else "bili_web"
        platform = context.get("uploader") if context.get("uploader") else "biliup-rs"
        if context.get('user_cookie'):
            platform = 'biliup-rs'
        cls = Plugin.upload_plugins.get(platform)
        if cls is None:
            return logger.error(f"No such uploader: {platform}")

        date = data.get("date") if data.get("date") else common.time.now()
        room_title = data.get('title') if data.get('title') else index
        if context.get('title'):
            data["format_title"] = custom_fmtstr(context.get('title'), date, room_title)
        else:
            data["format_title"] = f"{common.time.format_time(date)}{index}"
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


def custom_fmtstr(string, time, title):
    return common.time.format_time(time, string).format(title=title)
