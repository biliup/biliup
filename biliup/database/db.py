import time
import inspect
from datetime import datetime, timedelta
from pathlib import Path
from typing import List

from peewee import Field, ForeignKeyField
from playhouse.shortcuts import model_to_dict
from playhouse.migrate import SqliteMigrator, migrate

from .models import StreamerInfo, FileList, db, logger, LiveStreamers, UploadStreamers, Configuration


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
        StreamerInfo.create_table_()
        FileList.create_table_()
        LiveStreamers.create_table_()
        UploadStreamers.create_table_()
        Configuration.create_table_()
        # with db.connection_context():
        #     columns_name_list = [column_meta.name for column_meta in db.get_columns('uploadstreamers')]
        # if 'up_selection_reply' not in columns_name_list:
        #     logger.error(f"检测到旧数据库，请手动删除data文件夹后重试")
        #     return False
        try:
            cls.auto_migrate(StreamerInfo, FileList, LiveStreamers, UploadStreamers, Configuration)
        except Exception as e:
            logger.error(f"自动迁移旧数据库失败，请手动删除data文件夹后重试: {e}")
            return False
        return run

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
        """根据 streamer 获取下载信息, 若不存在则返回空字典"""
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
            except FileList.DoesNotExist:
                return {}
        stream_info_dict = {key: value for key, value in stream_info_dict.items() if value}  # 清除字典中的空元素
        stream_info_dict["date"] = datetime_to_struct_time(stream_info_dict["date"])  # 将开播时间转回 struct_time 类型
        return stream_info_dict

    @classmethod
    def add_stream_info(cls, name: str, url: str, date: time.struct_time) -> int:
        """添加下载信息, 返回所添加行的 id """
        return StreamerInfo.add(
            name=name,
            url=url,
            date=struct_time_to_datetime(date),
            title="",
            live_cover_path="",
        )

    @classmethod
    def delete_stream_info(cls, name: str) -> int:
        """根据 streamer 删除下载信息, 返回删除的行数, 若不存在则返回 0 """
        return StreamerInfo.delete_(name=name)

    @classmethod
    def delete_stream_info_by_date(cls, name: str, date: time.struct_time) -> int:
        """根据 streamer 和开播时间删除下载信息, 返回删除的行数, 若不存在则返回 0 """
        start_datetime = struct_time_to_datetime(date)
        with db.atomic():
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
        if not live_cover_path:
            live_cover_path = ""
        with db.atomic():
            return StreamerInfo.update(
                live_cover_path=live_cover_path
            ).where(StreamerInfo.id == database_row_id).execute()

    @classmethod
    def update_room_title(cls, database_row_id: int, title: str):
        """更新直播标题"""
        if not title:
            title = ""
        with db.atomic():
            return StreamerInfo.update(
                title=title
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
    def get_file_list(cls, database_row_id: int) -> List[str]:
        """获取视频文件列表"""
        file_list = StreamerInfo.get_by_id_(database_row_id).file_list
        return [file.file for file in file_list]

    @classmethod
    def auto_migrate(cls, *models):
        """迁移不同版本数据库"""
        migrator = SqliteMigrator(db)
        migrate_action = []  # 需要进行的操作列表
        for model in models:
            old_columns_name_list = [  # 当前数据库文件中的列名列表
                column_meta.name for column_meta in db.get_columns(model._meta.table_name)
            ]

            for name, value in inspect.getmembers(model):  # 遍历模型中的成员变量
                if isinstance(value, ForeignKeyField) and not name.endswith("_id"):
                    name += "_id"  # 基于 Django 约定, 外键会自动加 _id 后缀
                if (not isinstance(value, Field)) or (name in old_columns_name_list):  # 排除无关成员变量和已经存在的列
                    continue
                logger.info(f"添加行: {name}")
                migrate_action.append(migrator.add_column(model._meta.table_name, name, value))
        if len(migrate_action) > 0:
            migrate(*migrate_action)

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
        LiveStreamers.update(
            url=url,
            remark=remark,
            filename_prefix=filename_prefix,
            upload_streamers=upload_streamers,
            format=format,
            preprocessor=preprocessor,
            downloaded_processor=downloaded_processor,
            postprocessor=postprocessor,
            opt_args=opt_args,
        ).where(LiveStreamers.id == id).execute()

    def backup(self):
        """备份数据库"""
        pass
