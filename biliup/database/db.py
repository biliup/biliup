import time
from datetime import datetime

from .models import StreamerInfo, db


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

    @classmethod
    def connect(cls):
        """打开数据库连接"""
        db.connect()

    @classmethod
    def close(cls):
        """关闭数据库连接"""
        db.close()

    @classmethod
    def get_stream_info(cls, name: str):
        """获取下载信息"""
        res = StreamerInfo.get_dict(name=name)
        if res:
            res["date"] = datetime_to_struct_time(res["date"])
            return res
        return {}

    @classmethod
    def add_stream_info(cls, name: str, url: str, title: str, date: time.struct_time, live_cover_path: str):
        """添加下载信息"""
        return StreamerInfo.add(
            name=name,
            url=url,
            title=title,
            date=struct_time_to_datetime(date),
            live_cover_path=live_cover_path
        )

    @classmethod
    def delete_stream_info(cls, name: str):
        """删除下载信息"""
        return StreamerInfo.delete_(name=name)

    @classmethod
    def update_stream_info(cls, name: str, url: str, title: str, date: time.struct_time, live_cover_path: str):
        """更新下载信息"""
        return StreamerInfo.update(
            url=url,
            title=title,
            date=struct_time_to_datetime(date),
            live_cover_path=live_cover_path
        ).where(StreamerInfo.name == name).execute()

    def backup(self):
        """备份数据库"""
        pass
