import time
from datetime import datetime, timedelta
from pathlib import Path
from typing import List

from sqlalchemy import Table, select, desc, delete, update
from sqlalchemy.orm import sessionmaker

from .models import (
    engine,
    logger,
    BaseModel,
    StreamerInfo,
    FileList,
    LiveStreamers,
)

Session = sessionmaker(bind=engine)


def struct_time_to_datetime(date: time.struct_time):
    return datetime.fromtimestamp(time.mktime(date))


def datetime_to_struct_time(date: datetime):
    return time.localtime(date.timestamp())


class DB:
    """数据库交互类"""

    @classmethod
    def init(cls):
        """初始化数据库"""
        run = not Path.cwd().joinpath("data/data.sqlite3").exists()
        BaseModel.metadata.create_all(engine)  # 创建所有表
        table = Table('uploadstreamers', BaseModel.metadata, autoload_with=engine)
        columns_name_list = table.c.keys()
        if 'up_selection_reply' not in columns_name_list:
            logger.error(f"检测到旧数据库，请手动删除data文件夹后重试")
            return False
        return run

    @classmethod
    def get_stream_info(cls, name: str) -> dict:
        """根据 streamer 获取下载信息, 若不存在则返回空字典"""
        with Session() as session:
            res = session.execute(
                select(StreamerInfo).
                filter_by(name=name).
                order_by(desc(StreamerInfo.id))
            ).first()
            if res:
                res = res._asdict()
                res["date"] = datetime_to_struct_time(res["date"])
                return res
        return {}

    @classmethod
    def get_stream_info_by_filename(cls, filename: str) -> dict:
        """通过文件名获取下载信息, 若不存在则返回空字典"""
        with Session() as session:
            try:
                # stream_info = FileList.get(FileList.file == filename).streamer_info
                stream_info = session.execute(
                    select(FileList).
                    where(FileList.file == filename)
                ).scalar_one().streamerinfo
                stream_info_dict = stream_info.__dict__
            except Exception:
                return {}
        stream_info_dict = {key: value for key, value in stream_info_dict.items() if value}  # 清除字典中的空元素
        stream_info_dict["date"] = datetime_to_struct_time(stream_info_dict["date"])  # 将开播时间转回 struct_time 类型
        return stream_info_dict

    @classmethod
    def add_stream_info(cls, name: str, url: str, date: time.struct_time) -> int:
        """添加下载信息, 返回所添加行的 id """
        streamerinfo = StreamerInfo(
            name=name,
            url=url,
            date=struct_time_to_datetime(date),
            title="",
            live_cover_path="",
        )
        with Session.begin() as session:
            session.add(streamerinfo)
        return streamerinfo.id

    @classmethod
    def delete_stream_info(cls, name: str) -> int:
        """根据 streamer 删除下载信息, 返回删除的行数, 若不存在则返回 0 """
        with Session.begin() as session:
            result = session.execute(
                delete(StreamerInfo).where(StreamerInfo.name == name))
            return result.rowcount()

    @classmethod
    def delete_stream_info_by_date(cls, name: str, date: time.struct_time) -> int:
        """根据 streamer 和开播时间删除下载信息, 返回删除的行数, 若不存在则返回 0 """
        start_datetime = struct_time_to_datetime(date)
        stmt = delete(StreamerInfo).where(
            (StreamerInfo.name == name),
            StreamerInfo.date.between(
                start_datetime - timedelta(minutes=1),
                start_datetime + timedelta(minutes=1)),
        )
        with Session.begin() as session:
            result = session.execute(stmt)
            return result.rowcount()

    @classmethod
    def update_cover_path(cls, database_row_id: int, live_cover_path: str):
        """更新封面存储路径"""
        if not live_cover_path:
            live_cover_path = ""
        with Session.begin() as session:
            streamerinfo = session.scalar(select(StreamerInfo).where(StreamerInfo.id == database_row_id))
            streamerinfo.live_cover_path = live_cover_path

    @classmethod
    def update_room_title(cls, database_row_id: int, title: str):
        """更新直播标题"""
        if not title:
            title = ""
        with Session.begin() as session:
            streamerinfo = session.get(StreamerInfo, database_row_id)
            streamerinfo.title = title

    @classmethod
    def update_file_list(cls, database_row_id: int, file_name: str) -> int:
        """向视频文件列表中添加文件名"""
        with Session.begin() as session:
            streamer_info = session.get(StreamerInfo, database_row_id)
            file_list = FileList(file=file_name, streamer_info_id=streamer_info.id)
            session.add(file_list)
            return file_list.id

    @classmethod
    def get_file_list(cls, database_row_id: int) -> List[str]:
        """获取视频文件列表"""
        with Session.begin() as session:
            file_list = session.get(StreamerInfo, database_row_id).filelist
            return [file.file for file in file_list]

    @classmethod
    def update_live_streamer(
            cls, id, url, remark,
            filename_prefix=None,
            upload_streamers=None,
            format=None,
            preprocessor=None,
            downloaded_processor=None,
            postprocessor=None,
            opt_args=None, **kwargs):
        """ 更新 LiveStreamers 表中的数据, 增加一层包装避免多余参数报错 """
        stmt = update(LiveStreamers).where(LiveStreamers.id == id).values(
            url=url,
            remark=remark,
            filename_prefix=filename_prefix,
            upload_streamers=upload_streamers,
            format=format,
            preprocessor=preprocessor,
            downloaded_processor=downloaded_processor,
            postprocessor=postprocessor,
            opt_args=opt_args,
        )
        with Session.begin() as session:
            session.execute(stmt)

    def backup(self):
        """备份数据库"""
        pass
