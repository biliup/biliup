import time
import json
from datetime import datetime, timedelta
from playhouse.shortcuts import model_to_dict

from .models import StreamerInfo, FileList, db


def struct_time_to_datetime(date: time.struct_time):
    return datetime.fromtimestamp(time.mktime(date))


def datetime_to_struct_time(date: datetime):
    return time.localtime(date.timestamp())


class DB:
    """数据库交互类"""

    @classmethod
    def init(cls):
        """初始化数据库"""
        StreamerInfo.create_table_()
        FileList.create_table_()

    @classmethod
    def connect(cls):
        """打开数据库连接"""
        db.connect()

    @classmethod
    def close(cls):
        """关闭数据库连接"""
        db.close()

    @classmethod
    def get_stream_info(cls, name: str) -> dict:
        """获取下载信息, 若不存在则返回空字典"""
        res = StreamerInfo.get_dict(name=name)
        if res:
            res["date"] = datetime_to_struct_time(res["date"])
            return res
        return {}

    @classmethod
    def get_stream_info_by_filename(cls, filename: str) -> dict:
        """通过文件名获取下载信息, 若不存在则返回空字典"""
        with db.connection_context():
            try:
                stream_info = FileList.get(FileList.file == filename).streamer_info
                stream_info_dict = model_to_dict(stream_info)
                stream_info_dict["date"] = datetime_to_struct_time(stream_info_dict["date"])
                return stream_info_dict
            except FileList.DoesNotExist:
                return {}

    @classmethod
    def add_stream_info(cls, name: str, url: str, title: str, date: time.struct_time) -> int:
        """添加下载信息, 返回所添加行的 id """
        return StreamerInfo.add(
            name=name,
            url=url,
            title=title,
            date=struct_time_to_datetime(date),
        )

    @classmethod
    def delete_stream_info(cls, name: str) -> int:
        """根据 streamer 删除下载信息, 返回删除的行数, 若不存在则返回 0 """
        return StreamerInfo.delete_(name=name)

    @classmethod
    def delete_stream_info_by_date(cls, name: str, date: time.struct_time) -> int:
        """根据 streamer 和开播时间删除下载信息, 返回删除的行数, 若不存在则返回 0 """
        start_datetime = struct_time_to_datetime(date)
        with db.connection_context():
            dq = StreamerInfo.delete().where(
                (StreamerInfo.name == name) &
                (StreamerInfo.date.between(  # 传入的开播时间前后一分钟内都可以匹配
                    start_datetime - timedelta(minutes=1),
                    start_datetime + timedelta(minutes=1)))
            )
            return dq.execute()

    @classmethod
    def update_cover_path(cls, database_row_id: int, live_cover_path: str):
        """更新封面存储路径"""
        with db.connection_context():
            return StreamerInfo.update(
                live_cover_path=live_cover_path
            ).where(StreamerInfo.id == database_row_id).execute()

    @classmethod
    def update_file_list(cls, database_row_id: int, file_name: str) -> int:
        """向视频文件列表中添加文件名"""
        streamer_info = StreamerInfo.get_by_id_(database_row_id)
        return FileList.add(
            file=file_name,
            streamer_info=streamer_info
        )

    @classmethod
    def get_file_list(cls, database_row_id: int) -> list[str]:
        """获取视频文件列表"""
        file_list = StreamerInfo.get_by_id_(database_row_id).file_list
        return [file.file for file in file_list]

    def backup(self):
        """备份数据库"""
        pass
