from pathlib import Path

from peewee import CharField, DateTimeField, IntegrityError, Model, SqliteDatabase
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
    def add(cls, **kwargs):
        """添加行"""
        with db.connection_context():
            try:
                cls.create(**kwargs)
                return True
            except IntegrityError:
                return False

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
    def get_dict(cls, **kwargs):
        """获取字典类型的数据"""
        with db.connection_context():
            try:
                obj = cls.get(**kwargs)
                return model_to_dict(obj)
            except cls.DoesNotExist:
                return False

class StreamerInfo(BaseModel):
    name = CharField(primary_key=True)
    url = CharField()
    title = CharField()
    date = DateTimeField()
    live_cover_path = CharField()
