import inspect
import logging
import time
import json

from biliup.config import config
from .engine.decorators import Plugin

logger = logging.getLogger('biliup')

def merge_dict(*dicts):
    """
    合并多个字典，兼容空字典、None 或 JSON 字符串
    :param dicts: 任意数量的字典
    :return: 合并后的字典
    """
    result = {}
    for d in dicts:
        if isinstance(d, str):  # 如果是 JSON 字符串，尝试解析
            try:
                d = json.loads(d)
            except json.JSONDecodeError:
                continue
        if d:  # 检查字典是否为 None 或空
            result.update(d)
    return result

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
        platform = context.get("uploader") if context.get("uploader") else "biliup-rs"
        cls = Plugin.upload_plugins.get(platform)
        if cls is None:
            return logger.error(f"No such uploader: {platform}")
        data, context = fmt_title_and_desc(data)
        data['dolby'] = config.get('dolby', 0)
        data['hires'] = config.get('hires', 0)
        data['no_reprint'] = config.get('no_reprint', 0)
        data['extra_fields'] = json.dumps(merge_dict(context.get('extra_fields', {}), {"is_only_self": context.get('is_only_self', 0)}))

        data['open_elec'] = config.get('open_elec', 0)
        sig = inspect.signature(cls)
        kwargs = {}
        for k in sig.parameters:
            v = context.get(k)
            if v:
                kwargs[k] = v
        return cls(index, data, **kwargs).start()
    except:
        logger.exception("Uncaught exception:")


def biliup_uploader(filelist, data):
    try:
        index = data['name']
        context = {**data}
        platform = context.get("uploader") if context.get("uploader") else "biliup-rs"
        cls = Plugin.upload_plugins.get(platform)
        if cls is None:
            return logger.error(f"No such uploader: {platform}")
        data, context = fmt_title_and_desc_m(data)
        data['dolby'] = data.get('dolby', 0)
        data['hires'] = data.get('hires', 0)
        data['no_reprint'] = data.get('no_reprint', 0)
        data['extra_fields'] = json.dumps(merge_dict(data.get('extra_fields', ''), {"is_only_self": data.get('is_only_self', 0)}))

        data['open_elec'] = data.get('open_elec', 0)
        sig = inspect.signature(cls)
        kwargs = {}
        for k in sig.parameters:
            v = context.get(k)
            if v:
                kwargs[k] = v
        logger.info("start biliup")
        return cls(index, data, **kwargs).upload(filelist)
    except:
        logger.exception("Uncaught exception:")
    else:
        logger.info("stop biliup")


def fmt_title_and_desc_m(data):
    index = data['name']
    context = {**data}
    streamer = data.get('streamer', index)
    date = data.get("date", time.localtime())
    title = data.get('title', index)
    url = data.get('url')
    data["format_title"] = custom_fmtstr(context.get('title') or f'%Y.%m.%d{index}', date, title, streamer, url)
    if context.get('description'):
        context['description'] = custom_fmtstr(context.get('description'), date, title, streamer, url)
    return data, context


# 将格式化标题和简介拆分出来方便复用
def fmt_title_and_desc(data):
    """
    格式化标题和简介
    :param data: {name,url,{
        title,
        row_id,
        name,
        live_cover_path,
        url,
        date: time.struct_time()
    }}
    :return: {name,url,date,format_title}
    """
    index = data['name']
    context = {**config, **config['streamers'][index]}
    streamer = data.get('streamer', index)
    date = data.get("date", time.localtime())
    title = data.get('title', index)
    url = data.get('url')
    data["format_title"] = custom_fmtstr(context.get('title') or f'%Y.%m.%d{index}', date, title, streamer, url)
    if context.get('description'):
        context['description'] = custom_fmtstr(context.get('description'), date, title, streamer, url)
    return data, context


def custom_fmtstr(string, date, title, streamer, url):
    return time.strftime(string.encode('unicode-escape').decode(), date).encode().decode("unicode-escape").format(
        title=title, streamer=streamer, url=url)
