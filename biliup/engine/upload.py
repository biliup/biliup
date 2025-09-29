import logging

from typing import NamedTuple, Optional, List

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


    def upload(self, file_list: List[FileInfo]) -> List[FileInfo]:
        raise NotImplementedError()
