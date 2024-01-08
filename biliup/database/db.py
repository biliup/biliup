import time
from datetime import datetime, timedelta
from pathlib import Path
from typing import List

from peewee import OperationalError
from playhouse.shortcuts import model_to_dict

from .models import StreamerInfo, FileList, db, logger, TempStreamerInfo, LiveStreamers, UploadStreamers, Configuration


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
        with db.connection_context():
            columns_name_list = [column_meta.name for column_meta in db.get_columns('uploadstreamers')]
        if 'up_selection_reply' not in columns_name_list:
            logger.error(f"检测到旧数据库，请手动删除data文件夹后重试")
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
    def migrate_streamer_info(cls):
        """迁移旧版数据库中数据到新版"""
        logger.info("检测到旧版数据表，正在自动迁移")
        with db.atomic():
            # 创建新的临时表格
            TempStreamerInfo.create_table()
            # 将数据从原表格拷贝到新表格
            db.execute_sql(
                'INSERT INTO temp_streamer_info (name, url, title, date, live_cover_path) SELECT name, url, title, date, live_cover_path FROM streamerinfo')
            # 删除原表格
            StreamerInfo.drop_table()
            # 将新表格重命名为原表格的名字
            db.execute_sql('ALTER TABLE temp_streamer_info RENAME TO streamerinfo')

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
