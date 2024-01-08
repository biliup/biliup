from pathlib import Path
import logging

from peewee import CharField, DateTimeField, IntegrityError, ForeignKeyField, AutoField, Model, \
    TextField, IntegerField
from playhouse.shortcuts import ReconnectMixin, model_to_dict
from playhouse.sqlite_ext import SqliteExtDatabase, JSONField

logger = logging.getLogger('biliup')


def get_path(*other):
    """获取数据文件绝对路径"""
    dir_path = Path.cwd().joinpath("data")
    # 若目录不存在则创建
    if not dir_path.is_dir():
        dir_path.mkdir(parents=True)
    return str(dir_path.joinpath(*other))


# 自动重连, 避免报错导致连接丢失
class ReconnectSqliteDatabase(ReconnectMixin, SqliteExtDatabase):
    pass


db = ReconnectSqliteDatabase(f"{get_path('data.sqlite3')}")


class BaseModel(Model):
    class Meta:
        database = db

    @classmethod
    def add(cls, **kwargs) -> int:
        """添加行, 返回添加的行的 id 值"""
        with db.atomic():
            try:
                dq = cls.create(**kwargs)
                return dq.id
            except IntegrityError:
                return 0

    @classmethod
    def delete_(cls, **kwargs):
        """删除行"""
        with db.atomic():
            try:
                query = cls.get(**kwargs)
                return query.delete_instance()
            except cls.DoesNotExist:
                return 0

    @classmethod
    def create_table_(cls):
        """创建表"""
        with db.atomic():
            if not cls.table_exists():
                cls.create_table()

    @classmethod
    def get_by_id_(cls, pk):
        """根据主键获取记录"""
        with db.connection_context():
            try:
                return cls.get_by_id(pk)
            except cls.DoesNotExist:
                return cls()  # 若不存在, 则返回一个空对象

    @classmethod
    def get_dict(cls, **kwargs):
        """获取字典类型的数据"""
        with db.connection_context():
            try:
                obj = cls.get(**kwargs)
                return model_to_dict(obj)
            except cls.DoesNotExist:
                return {}


class StreamerInfo(BaseModel):
    """下载信息"""
    id = AutoField(primary_key=True)  # 自增主键
    name = CharField()  # streamer 名称
    url = CharField()  # 录制的 url
    title = CharField()  # 直播标题
    date = DateTimeField()  # 开播时间
    live_cover_path = CharField()  # 封面存储路径


class FileList(BaseModel):
    """存储文件名列表, 通过外键和 StreamerInfo 表关联"""
    id = AutoField(primary_key=True)  # 自增主键
    file = CharField()  # 文件名
    # 外键, 对应 StreamerInfo 中的下载信息, 且启用级联删除
    streamer_info = ForeignKeyField(StreamerInfo, backref='file_list', on_delete='CASCADE')

class Configuration(BaseModel):
    """暂时将配置文件整体存入，后续可拆表拆字段"""
    id = AutoField(primary_key=True)  # 自增主键
    key = CharField()  # 全局配置键
    value = TextField()  # 全局配置值

class UploadStreamers(BaseModel):
    """投稿模板"""
    id = AutoField(primary_key=True)  # 自增主键
    template_name = CharField()  # 模板名称
    title = CharField(null=True)  # 自定义标题的时间格式, {title}代表当场直播间标题 {streamer}代表在本config里面设置的主播名称 {url}代表设置的该主播的第一条直播间链接
    tid = IntegerField(null=True)  # 投稿分区码,171为电子竞技分区
    copyright = IntegerField(null=True)  # 1为自制
    cover_path = CharField(null=True)  # 封面路径
    # 支持strftime, {title}, {streamer}, {url}占位符。
    description = TextField(null=True)  # 视频简介
    dynamic = CharField(null=True)  # 粉丝动态
    dtime = IntegerField(null=True)  # 设置延时发布时间，距离提交大于2小时，格式为时间戳
    dolby = IntegerField(null=True)  # 是否开启杜比音效, 1为开启
    hires = IntegerField(null=True)  # 是否开启Hi-Res, 1为开启
    open_elec = IntegerField(null=True)  # 是否开启充电面板, 1为开启
    no_reprint = IntegerField(null=True)  # 自制声明, 1为未经允许禁止转载
    uploader = CharField(null=True)  # 覆盖全局默认上传插件，Noop为不上传，但会执行后处理
    user_cookie = CharField(null=True)  # 使用指定的账号上传
    tags = JSONField()  # 标签
    credits = JSONField(null=True)  # 简介@模板
    up_selection_reply = IntegerField(null=True)  # 精选评论
    up_close_reply = IntegerField(null=True)  # 关闭评论
    up_close_danmu = IntegerField(null=True)  # 精选评论

class LiveStreamers(BaseModel):
    """每个直播间的配置"""
    id = AutoField(primary_key=True)  # 自增主键
    url = CharField(unique=True)  # 直播间地址
    remark = CharField()  # 对应配置文件中 {streamer} 变量
    filename_prefix = CharField(null=True)  # filename_prefix 支持模板
    # 外键, 对应 UploadStreamers, 且启用级联删除
    upload_streamers = ForeignKeyField(UploadStreamers, backref='live_streamers', on_delete='CASCADE', null=True)
    format = CharField(null=True)  # 视频格式
    preprocessor = JSONField(null=True)  # 开始下载直播时触发
    downloaded_processor = JSONField(null=True)  # 准备上传直播时触发
    postprocessor = JSONField(null=True)  # 上传完成后触发
    opt_args = JSONField(null=True)  # ffmpeg参数

class TempStreamerInfo(StreamerInfo):
    class Meta:
        table_name = 'temp_streamer_info'
