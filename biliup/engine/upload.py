import logging
import os
import pathlib
import shutil

from typing import NamedTuple, Optional, List

from sqlalchemy import desc

from biliup.common.tools import NamedLock, get_file_create_timestamp
from biliup.config import config
from biliup.database import models
from biliup.database.db import SessionLocal

logger = logging.getLogger('biliup')


class UploadBase:
    class FileInfo(NamedTuple):
        video: str
        danmaku: Optional[str]

    def __init__(self, principal, data, persistence_path=None, postprocessor=None):
        self.principal = principal
        self.persistence_path = persistence_path
        self.data: dict = data
        self.post_processor = postprocessor

    @staticmethod
    def file_list(index) -> List[FileInfo]:
        from biliup.handler import event_manager
        media_extensions = ['.mp4', '.flv', '.3gp', '.webm', '.mkv', '.ts']

        # 获取文件列表
        file_list = []
        # 数据库中保存的文件名, 不含后缀
        save = []
        with SessionLocal() as db:
            dbinfo = db.query(models.StreamerInfo).filter(models.StreamerInfo.name == index).order_by(
                desc(models.StreamerInfo.id)).first()
            if dbinfo:
                for dbfile in dbinfo.filelist:
                    save.append(dbfile.file)
        for file_name in os.listdir('.'):
            # 可能有两层后缀.with_suffix('')去掉一层.stem取文件名
            if (index in file_name or pathlib.Path(file_name).with_suffix('').stem in save ) and os.path.isfile(file_name):
                file_list.append(file_name)
        if len(file_list) == 0:
            return []

        file_list = sorted(file_list, key=lambda x: get_file_create_timestamp(x))

        # 正在上传的文件列表
        upload_filename: list = event_manager.context['upload_filename']

        results = []
        for index, file in enumerate(file_list):
            old_name = file
            if file.endswith('.part'):
                file_list[index] = os.path.splitext(file)[0]
                file = os.path.splitext(file)[0]

            name, ext = os.path.splitext(file)
            # 过滤正在上传的
            if name in upload_filename:
                continue
            # 过滤不是视频的
            if ext not in media_extensions:
                continue

            if old_name != file:
                logger.info(f'{old_name} 已更名为 {file}')
                shutil.move(old_name, file)

            file_size = os.path.getsize(file) / 1024 / 1024
            threshold = config.get('filtering_threshold', 0)
            if file_size <= threshold:
                os.remove(file)
                logger.info(f'过滤删除 - {file}')
                continue

            video = file
            danmaku = None
            if f'{name}.xml' in file_list:
                danmaku = f'{name}.xml'

            result = UploadBase.FileInfo(video=video, danmaku=danmaku)
            results.append(result)

        # 过滤弹幕
        for file in file_list:
            name, ext = os.path.splitext(file)
            # 过滤正在上传的
            if name in upload_filename:
                continue
            if ext == '.xml':
                have_video = False
                for result in results:
                    if result.danmaku == file:
                        have_video = True
                        break
                if not have_video:
                    logger.info(f'无视频，过滤删除 - {file}')
                    UploadBase.remove_file(file)
        return results

    @staticmethod
    def remove_filelist(file_list: List[FileInfo]):
        for f in file_list:
            UploadBase.remove_file(f.video)
            if f.danmaku is not None:
                UploadBase.remove_file(f.danmaku)

    @staticmethod
    def remove_file(file: str):
        try:
            os.remove(file)
            logger.info(f'删除 - {file}')
        except:
            logger.warning(f'删除失败 - {file}')

    def upload(self, file_list: List[FileInfo]) -> List[FileInfo]:
        raise NotImplementedError()

    def start(self):
        from biliup.handler import event_manager
        # 保证一个name同时只有一个上传线程扫描文件列表
        lock = NamedLock(f'upload_file_list_{self.principal}')
        upload_filename_list = []
        try:
            lock.acquire()
            file_list = UploadBase.file_list(self.principal)

            if len(file_list) > 0:
                upload_filename_list = [os.path.splitext(file.video)[0] for file in file_list]

                logger.info('准备上传' + self.data["format_title"])
                with NamedLock('upload_filename'):
                    event_manager.context['upload_filename'].extend(upload_filename_list)
                lock.release()
                file_list = self.upload(file_list)
                return file_list
        finally:
            with NamedLock('upload_filename'):
                event_manager.context['upload_filename'] = list(
                    set(event_manager.context['upload_filename']) - set(upload_filename_list))
            if lock.locked():
                lock.release()
