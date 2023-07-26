import logging
import os
import re
import shutil
import subprocess
from functools import reduce
from pathlib import Path

logger = logging.getLogger('biliup')


class UploadBase:
    def __init__(self, principal, data, persistence_path=None, postprocessor=None):
        self.principal = principal
        self.persistence_path = persistence_path
        self.data = data
        self.post_processor = postprocessor

    # @property
    @staticmethod
    def file_list(index):
        file_list = []
        for file_name in os.listdir('.'):
            if index in file_name and os.path.isfile(file_name):
                file_list.append(file_name)
        file_list = sorted(file_list)
        from biliup.handler import event_manager
        # 去掉正在上传的
        upload_filename: list = event_manager.context['upload_filename']
        file_list = list(set(file_list) - set(upload_filename))
        return file_list

    @staticmethod
    def remove_filelist(file_list):
        for r in file_list:
            os.remove(r)
            logger.info('删除-' + r)

    def filter_file(self, index):
        media_extensions = ['.mp4', '.flv', '.3gp', '.webm', '.mkv', '.ts', '.flv.part']
        file_list = UploadBase.file_list(index)
        if len(file_list) == 0:
            return False
        for f in file_list:
            if f.endswith('.part'):
                new_name = os.path.splitext(f)[0]
                shutil.move(f, new_name)
                logger.info(f'{f}存在已更名为{new_name}')
        for r in file_list:
            name, ext = os.path.splitext(r)
            if ext in ('.mp4', '.flv', '.ts'):
                file_size = os.path.getsize(r) / 1024 / 1024
                threshold = self.data.get('threshold', 2)
                if file_size <= threshold:
                    self.remove_file(r)
                    logger.info(f'过滤删除-{r}')
            if ext == '.xml':  # 过滤不存在对应视频的xml弹幕文件
                xml_file_name = name
                # media_regex = re.compile(r'^{}(\.(mp4|flv|ts))?$'.format(
                #     re.escape(xml_file_name)
                # ))
                # if not any(media_regex.match(f'{xml_file_name}{ext2}') for ext2 in media_extensions for x in file_list):
                #     self.remove_file(r)
                #     logger.info(f'无视频，已过滤删除-{r}')
                have_video = False
                for video_ext in media_extensions:
                    if f"{xml_file_name}{video_ext}" in file_list: have_video = True
                if not have_video:
                    self.remove_file(r)
                    logger.info(f'无视频，已过滤删除-{r}')
        file_list = UploadBase.file_list(index)
        if len(file_list) == 0:
            logger.info('视频过滤后无文件可传')
            return False

        return True

    def remove_file(self, file_path):
        os.remove(file_path)

    def upload(self, file_list):
        raise NotImplementedError()

    def start(self):
        if self.filter_file(self.principal):
            logger.info('准备上传' + self.data["format_title"])
            file_list = UploadBase.file_list(self.principal)
            from biliup.handler import event_manager
            upload_filename: list = event_manager.context['upload_filename']
            try:
                event_manager.context['upload_filename'].extend(file_list)
                needed2process = self.upload(file_list)
                if needed2process:
                    self.postprocessor(needed2process)
            finally:
                event_manager.context['upload_filename'] = list(set(upload_filename) - set(file_list))

    def postprocessor(self, data):
        # data = file_list
        if self.post_processor is None:
            return self.remove_filelist(data)
        for post_processor in self.post_processor:
            if post_processor == 'rm':
                self.remove_filelist(data)
                continue
            if post_processor.get('mv'):
                for file in data:
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
                        input=reduce(lambda x, y: x + str(Path(y).absolute()) + '\n', data, ''),
                        stderr=subprocess.STDOUT, text=True)
                    logger.info(process_output.rstrip())
                except subprocess.CalledProcessError as e:
                    logger.exception(e.output)
                    continue
