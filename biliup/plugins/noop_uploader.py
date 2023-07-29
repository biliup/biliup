import time

from ..engine.upload import UploadBase, logger
from ..engine import Plugin


@Plugin.upload(platform="Noop")
class NoopUploader(UploadBase):
    def upload(self, file_list: list[UploadBase.FileInfo]) -> list[UploadBase.FileInfo]:
        logger.info("NoopUploader")
        time.sleep(10000)
        return file_list
