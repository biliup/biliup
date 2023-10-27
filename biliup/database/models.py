from pathlib import Path

from peewee import CharField, DateTimeField, IntegrityError, ForeignKeyField, AutoField, Model, SqliteDatabase
from playhouse.shortcuts import ReconnectMixin, model_to_dict


def get_path(*other):
    """获取数据文件绝对路径"""
    dir_path = Path.cwd().joinpath("data")
    # 若目录不存在则创建
    if not dir_path.is_dir():
        dir_path.mkdir(parents=True)
    return str(dir_path.joinpath(*other))


# 自动重连, 避免报错导致连接丢失
class ReconnectSqliteDatabase(ReconnectMixin, SqliteDatabase):
    pass


db = ReconnectSqliteDatabase(f"{get_path('data.sqlite3')}")


class BaseModel(Model):
    class Meta:
        database = db

    @classmethod
    def add(cls, **kwargs) -> int:
        """添加行, 返回添加的行的 id 值"""
        with db.connection_context():
            try:
                dq = cls.create(**kwargs)
                return dq.id
            except IntegrityError:
                return 0

    @classmethod
    def delete_(cls, **kwargs):
        """删除行"""
        with db.connection_context():
            try:
                query = cls.get(**kwargs)
                return query.delete_instance()
            except cls.DoesNotExist:
                return 0

    @classmethod
    def create_table_(cls):
        """创建表"""
        with db.connection_context():
            if not cls.table_exists():
                cls.create_table()

    @classmethod
    def get_by_id_(cls, pk):
        """根据主键获取记录"""
        with db.connection_context():
            return cls.get_by_id(pk)

    @classmethod
    def get_dict(cls, **kwargs):
        """获取字典类型的数据"""
        with db.connection_context():
            try:
                obj = cls.get(**kwargs)
                return model_to_dict(obj)
            except cls.DoesNotExist:
                return False


class StreamerInfo(BaseModel):
    """下载信息"""
    id = AutoField(primary_key=True)  # 自增主键
    name = CharField()  # streamer 名称
    url = CharField()  # 录制的 url
    title = CharField(null=True)  # 直播标题
    date = DateTimeField()  # 开播时间
    live_cover_path = CharField(null=True)  # 封面存储路径


class FileList(BaseModel):
    """存储文件名列表, 通过外键和 StreamerInfo 表关联"""
    id = AutoField(primary_key=True)  # 自增主键
    file = CharField()  # 文件名
    # 外键, 对应 StreamerInfo 中的下载信息, 且启用级联删除
    streamer_info = ForeignKeyField(StreamerInfo, backref='file_list', on_delete='CASCADE')
