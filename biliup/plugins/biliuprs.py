import time

import stream_gears

from ..engine import Plugin
from ..engine.upload import UploadBase, logger


@Plugin.upload(platform="biliup-rs")
class BiliWeb(UploadBase):
    def __init__(
            self, principal, data, submit_api=None, copyright=2, postprocessor=None, dtime=None,
            dynamic='', lines='AUTO', threads=3, tid=122, tags=None, cover_path=None, description='',
            user_cookie='cookies.json'
    ):
        super().__init__(principal, data, persistence_path='bili.cookie', postprocessor=postprocessor)
        if tags is None:
            tags = []
        self.lines = lines
        self.submit_api = submit_api
        self.threads = threads
        self.tid = tid
        self.tags = tags
        self.cover_path = cover_path
        self.desc = description
        self.dynamic = dynamic
        self.copyright = copyright
        self.dtime = dtime
        self.user_cookie = user_cookie

    def upload(self, file_list):
        line = None
        if self.lines == 'kodo':
            line = stream_gears.UploadLine.Kodo
        elif self.lines == 'bda2':
            line = stream_gears.UploadLine.Bda2
        elif self.lines == 'ws':
            line = stream_gears.UploadLine.Ws
        elif self.lines == 'qn':
            line = stream_gears.UploadLine.Qn
        elif self.lines == 'cos':
            line = stream_gears.UploadLine.Cos
        elif self.lines == 'cos-internal':
            line = stream_gears.UploadLine.CosInternal
        tag = ','.join(self.tags)
        source = self.data["url"] if self.copyright == 2 else ""
        cover = self.cover_path if self.cover_path is not None else ""
        dtime = None
        if self.dtime:
            dtime = int(time.time() + self.dtime)
        stream_gears.upload(
            file_list,
            self.user_cookie,
            self.data["format_title"][:80],
            self.tid,
            tag,
            self.copyright,
            source,
            self.desc,
            self.dynamic,
            cover,
            dtime,
            line,
            self.threads,
        )
        logger.info(f"上传成功: {self.principal}")
        return file_list
