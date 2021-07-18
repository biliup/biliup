import logging
import os

logger = logging.getLogger('biliup')


class UploadBase:
    def __init__(self, principal, data, persistence_path=None):
        self.principal = principal
        self.persistence_path = persistence_path
        self.data = data

    # @property
    @staticmethod
    def file_list(index):
        file_list = []
        for file_name in os.listdir('.'):
            if index in file_name:
                file_list.append(file_name)
        file_list = sorted(file_list)
        return file_list

    @staticmethod
    def remove_filelist(file_list):
        for r in file_list:
            os.remove(r)
            logger.info('删除-' + r)

    @staticmethod
    def filter_file(index):
        file_list = UploadBase.file_list(index)
        if len(file_list) == 0:
            return False
        for r in file_list:
            file_size = os.path.getsize(r) / 1024 / 1024 / 1024
            if file_size <= 0.02:
                os.remove(r)
                logger.info('过滤删除-' + r)
        file_list = UploadBase.file_list(index)
        if len(file_list) == 0:
            logger.info('视频过滤后无文件可传')
            return False
        for f in file_list:
            if f.endswith('.part'):
                os.rename(f, os.path.splitext(f)[0])
                logger.info('%s存在已更名' % f)
        return True

    def upload(self, file_list):
        raise NotImplementedError()

    def start(self):
        if self.filter_file(self.principal):
            logger.info('准备上传' + self.data["format_title"])
            needed2process = self.upload(UploadBase.file_list(self.principal))
            self.postprocessor(needed2process)

    def postprocessor(self, data):
        # data = file_list
        if data:
            self.remove_filelist(data)
