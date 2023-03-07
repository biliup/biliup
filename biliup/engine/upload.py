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
        return file_list

    @staticmethod
    def remove_filelist(file_list):
        for r in file_list:
            os.remove(r)
            logger.info('删除-' + r)


    def filter_file(self, index):
        media_extensions = ['.mp4', '.flv', '.ts']
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
                threshold = self.data.get('threshold',2)
                if file_size <= threshold:
                    self.remove_file(r)
                    logger.info(f'过滤删除-{r}')
            if ext == '.xml': #过滤不存在对应视频的xml弹幕文件
                xml_file_name = name
                media_regex = re.compile(r'^{}({})(\.part)?$'.format(
                    re.escape(xml_file_name), '|'.join(map(re.escape, media_extensions))
                ))
                if not any(media_regex.match(x) for x in file_list):
                    self.remove_file(r)
                    logger.info(f'无视频，已过滤删除-{r}')
        file_list = UploadBase.file_list(index)
        if len(file_list) == 0:
            logger.info('视频过滤后无文件可传')
            return False



        return True

    def remove_file(self, file_path):
        with open(file_path, 'r', encoding='utf-8'):
            os.remove(file_path)

    def upload(self, file_list):
        raise NotImplementedError()

    def start(self):
        if self.filter_file(self.principal):
            logger.info('准备上传' + self.data["format_title"])
            needed2process = self.upload(UploadBase.file_list(self.principal))
            if needed2process:
                self.postprocessor(needed2process)

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
