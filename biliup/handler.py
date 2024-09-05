import json
import logging
import os
import shutil
import subprocess
import time
from functools import reduce
from pathlib import Path
from typing import List

from biliup.config import config
from .app import event_manager, context
from .common.tools import NamedLock, processor
from .common.util import check_timerange
from .database.db import get_stream_info_by_filename, SessionLocal
from .downloader import biliup_download
from .engine.event import Event
from .engine.upload import UploadBase
from .uploader import upload, fmt_title_and_desc

PRE_DOWNLOAD = 'pre_download'
DOWNLOAD = 'download'
DOWNLOADED = 'downloaded'
UPLOAD = 'upload'
UPLOADED = 'uploaded'
logger = logging.getLogger('biliup')


# @event_manager.register(CHECK, block='Asynchronous3')


@event_manager.register(PRE_DOWNLOAD, block='Asynchronous1')
def pre_processor(name, url):
    url_status = context["PluginInfo"].url_status
    if url_status[url] == 1:
        logger.debug(f'{name} 正在下载中，跳过下载')
        return

    logger.info(f'{name} - {url} 开播了准备下载')
    preprocessor = config['streamers'].get(name, {}).get('preprocessor')
    if preprocessor:
        processor(preprocessor, json.dumps({
            "name": name,
            "url": url,
            "start_time": int(time.time())
        }, ensure_ascii=False))
    yield Event(DOWNLOAD, (name, url))


@event_manager.register(DOWNLOAD, block='Asynchronous1')
def process(name, url):
    url_status = context['PluginInfo'].url_status
    # 下载开始
    try:
        url_status[url] = 1
        stream_info = biliup_download(name, url, config['streamers'][name].copy())
        yield Event(DOWNLOADED, (stream_info,))
    except Exception as e:
        logger.exception(f"下载错误: {name} - {e}")
    finally:
        # 下载结束
        url_status[url] = 0 if check_timerange(name) else 2


@event_manager.register(DOWNLOADED, block='Asynchronous1')
def processed(stream_info):
    name = stream_info['name']
    url = stream_info['url']
    # 下载后处理 上传前处理
    downloaded_processor = config['streamers'].get(name, {}).get('downloaded_processor')
    if downloaded_processor:
        default_date = time.localtime()
        file_list = UploadBase.file_list(name)
        processor(downloaded_processor, json.dumps({
            "name": name,
            "url": url,
            "room_title": stream_info.get('title', name),
            "start_time": int(time.mktime(stream_info.get('date', default_date))),
            "end_time": int(time.mktime(stream_info.get('end_time', default_date))),
            "file_list": [file.video for file in file_list]
        }, ensure_ascii=False))
        # 后处理完成后重新扫描文件列表
    yield Event(UPLOAD, (stream_info,))


@event_manager.register(UPLOAD, block='Asynchronous2')
def process_upload(stream_info):
    url = stream_info['url']
    name = stream_info['name']
    url_upload_count = context['url_upload_count']
    # 永远不可能有两个同url的下载线程
    # 可能对同一个url同时发送两次上传事件
    with NamedLock(f"upload_count_{url}"):
        if context['url_upload_count'][url] > 0:
            return logger.debug(f'{url} 正在上传中，跳过')
        # from .handler import event_manager, UPLOAD
        # += 不是原子操作
        context['url_upload_count'][url] += 1
    # 上传开始
    try:
        file_list = UploadBase.file_list(name)
        file_count = len(file_list)
        if file_count <= 0:
            logger.debug("无需上传")
            return

        # 上传延迟检测
        url_status = context['PluginInfo'].url_status
        if url_status[url] != 2:
            delay = int(config.get('delay', 0))
            if delay:
                logger.info(f'{name} -> {url} {delay}s 后检测是否上传')
                time.sleep(delay)
                if url_status[url] == 1:
                    # 上传延迟检测，启用的话会在一段时间后检测是否存在下载任务，若存在则跳过本次上传
                    return logger.info(f'{name} -> {url} 存在下载任务, 跳过本次上传')

        if ("title" not in stream_info) or (not stream_info["title"]):  # 如果 data 中不存在标题, 说明下载信息已丢失, 则尝试从数据库获取
            with SessionLocal() as db:
                i = 0
                # 按创建时间排序可能导致首个结果无法获取到数据
                while i < file_count:
                    data = get_stream_info_by_filename(db, file_list[i].video)
                    if data:
                        break
                    i += 1
            data, _ = fmt_title_and_desc({**data, "name": name})  # 如果 restart, data 中会缺失 name 项
            stream_info.update(data)
        filelist = upload(stream_info)
        if filelist:
            uploaded(name, stream_info.get('live_cover_path'), filelist)
    except Exception:
        logger.exception(f"上传错误: {name}")
    finally:
        # 上传结束
        # 有可能有两个同url的上传线程 保证计数正确
        with NamedLock(f'upload_count_{url}'):
            url_upload_count[url] -= 1


def uploaded(name, live_cover_path, data: List):
    # data = file_list
    post_processor = config['streamers'].get(name, {}).get("postprocessor", None)
    if post_processor is None:
        # 删除封面
        if live_cover_path is not None:
            UploadBase.remove_file(live_cover_path)
        return UploadBase.remove_filelist(data)

    file_list = []
    for i in data:
        file_list.append(i.video)
        if i.danmaku is not None:
            file_list.append(i.danmaku)

    for post_processor in post_processor:
        if post_processor == 'rm':
            # 删除封面
            if live_cover_path is not None:
                UploadBase.remove_file(live_cover_path)
            UploadBase.remove_filelist(data)
            continue
        if post_processor.get('mv'):
            for file in file_list:
                path = Path(file)
                dest = Path(post_processor['mv'])
                if not dest.is_dir():
                    dest.mkdir(parents=True, exist_ok=True)
                try:
                    shutil.move(path, dest / path.name)
                except Exception as e:
                    logger.exception(e)
                    continue
                logger.info(f"move to {(dest / path.name).absolute()}")
        if post_processor.get('run'):
            try:
                process_output = subprocess.check_output(
                    post_processor['run'], shell=True,
                    input=reduce(lambda x, y: x + str(Path(y).absolute()) + '\n', file_list, ''),
                    stderr=subprocess.STDOUT, text=True)
                logger.info(process_output.rstrip())
            except subprocess.CalledProcessError as e:
                logger.exception(e.output)
                continue
