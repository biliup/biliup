import os
from typing import Any, Callable, ClassVar, Mapping, Optional, Protocol, Sequence, Union, final

from typing_extensions import TypeAlias, deprecated

from stream_gears import Credit

__all__ = [
    "PySegment",
    "StreamGearsError",
    "UploadLine",
    "config_bindings",
    "download",
    "download_with_callback",
    "get_qrcode",
    "login_by_cookies",
    "login_by_qrcode",
    "login_by_sms",
    "login_by_web_cookies",
    "login_by_web_qrcode",
    "main_loop",
    "send_sms",
    "upload",
]

_StrPath: TypeAlias = Union[str, "os.PathLike[str]"]

class StreamGearsError(RuntimeError):
    """下载、登录或上传失败时抛出。是 `RuntimeError` 的子类。"""

@final
class PySegment:
    """视频分段设置。`time`（秒）和 `size`（字节）都为 `None` 时不分段。"""

    time: Optional[int]
    size: Optional[int]
    def __init__(self) -> None: ...

class _SegmentObject(Protocol):
    @property
    def time(self) -> Optional[int]: ...
    @property
    def size(self) -> Optional[int]: ...

_SegmentArg: TypeAlias = Union[PySegment, Mapping[str, Optional[int]], _SegmentObject]

class _CreditObject(Protocol):
    @property
    def type_id(self) -> int: ...
    @property
    def raw_text(self) -> str: ...

@final
class UploadLine:
    """上传线路。`upload` 不传 `line` 时自动测速选择。

    `Cn*`、`An*`、`At*` 是同一 CDN 经不同接入域名的变体。
    """

    Bldsa: ClassVar[UploadLine]
    """B站大陆动态加速"""
    Cnbldsa: ClassVar[UploadLine]
    Andsa: ClassVar[UploadLine]
    Atdsa: ClassVar[UploadLine]
    Bda2: ClassVar[UploadLine]
    """百度云"""
    Cnbd: ClassVar[UploadLine]
    Anbd: ClassVar[UploadLine]
    Atbd: ClassVar[UploadLine]
    Tx: ClassVar[UploadLine]
    """腾讯云EO"""
    Cntx: ClassVar[UploadLine]
    Antx: ClassVar[UploadLine]
    Attx: ClassVar[UploadLine]
    Txa: ClassVar[UploadLine]
    """腾讯云EO海外"""
    Alia: ClassVar[UploadLine]
    """阿里云海外"""
    Estx: ClassVar[UploadLine]
    """B站自建"""
    Akbd: ClassVar[UploadLine]
    """B站自建"""
    def __int__(self) -> int: ...

class _ConfigState:
    def get(self, key: str, default: Any = None) -> Any:
        """`key` 不存在且未给 `default` 时抛 `AttributeError`。"""

def download(
    url: str,
    header_map: dict[str, str],
    file_name: str,
    segment: _SegmentArg,
    proxy: Optional[str] = None,
) -> None:
    """下载 FLV 或 HLS 直播流。

    :param url: 流地址
    :param header_map: HTTP 请求头
    :param file_name: 文件名格式，不含扩展名，支持 strftime 占位符（如 `%Y-%m-%d`）
    :param segment: 分段设置：`PySegment`、带 `time` / `size` 键的 dict，或带这两个属性的对象
    :param proxy: 代理
    :raises StreamGearsError: 连接失败、HTTP 错误状态、流数据不完整或解析失败
    """

def download_with_callback(
    url: str,
    header_map: dict[str, str],
    file_name: str,
    segment: _SegmentArg,
    file_name_callback_fn: Optional[Callable[[str], object]] = None,
    proxy: Optional[str] = None,
) -> None:
    """下载 FLV 或 HLS 直播流，每个分段文件写完时调用回调。

    回调抛出的异常会立即记进日志，下载继续；下载结束后重新抛出第一个回调异常。
    下载本身也失败时抛 `StreamGearsError`，回调异常挂在它的 `__cause__` 上。

    :param file_name_callback_fn: 以写完的文件名为参数的回调
    :raises StreamGearsError: 同 `download`
    :raises TypeError: `file_name_callback_fn` 不可调用
    """

def login_by_cookies(file: str, proxy: Optional[str] = None) -> bool:
    """用 cookie 文件登录。

    :param file: cookie 文件路径
    :return: 成功时为 `True`
    :raises StreamGearsError: 登录失败
    """

def send_sms(country_code: int, phone: int, proxy: Optional[str] = None) -> str:
    """发送短信验证码。

    :return: 传给 `login_by_sms` 的 JSON 字符串
    """

def login_by_sms(
    code: int,
    ret: str,
    proxy: Optional[str] = None,
    file: _StrPath = "cookies.json",
) -> bool:
    """短信验证码登录，成功后把登录信息写入 `file`。

    :param code: 验证码
    :param ret: `send_sms` 的返回值
    :return: 成功时为 `True`
    :raises StreamGearsError: 登录失败
    :raises ValueError: `ret` 不是合法 JSON
    """

def get_qrcode(proxy: Optional[str] = None) -> str:
    """获取登录二维码。

    :return: 传给 `login_by_qrcode` 的 JSON 字符串
    """

def login_by_qrcode(ret: str, proxy: Optional[str] = None, file: Optional[_StrPath] = None) -> str:
    """等待扫码登录。

    :param ret: `get_qrcode` 的返回值
    :param file: 给了就同时把登录信息写入该文件
    :return: 登录信息的 JSON 字符串
    :raises StreamGearsError: 登录失败
    :raises ValueError: `ret` 不是合法 JSON
    """

def login_by_web_cookies(
    sess_data: str,
    bili_jct: str,
    proxy: Optional[str] = None,
    file: _StrPath = "cookies.json",
) -> bool:
    """用网页 Cookie（`SESSDATA`、`bili_jct`）登录，成功后把登录信息写入 `file`。

    :return: 成功时为 `True`
    :raises StreamGearsError: 登录失败
    """

def login_by_web_qrcode(
    sess_data: str,
    dede_user_id: str,
    proxy: Optional[str] = None,
    file: _StrPath = "cookies.json",
) -> bool:
    """用网页 Cookie（`SESSDATA`、`DedeUserID`）登录，成功后把登录信息写入 `file`。

    :return: 成功时为 `True`
    :raises StreamGearsError: 登录失败
    """

def upload(
    video_path: Sequence[_StrPath],
    cookie_file: _StrPath,
    title: str,
    tid: int = 171,
    tid_v2: Optional[int] = None,
    tag: str = "",
    copyright: int = 2,
    source: str = "",
    desc: str = "",
    dynamic: str = "",
    cover: str = "",
    dolby: int = 0,
    lossless_music: int = 0,
    no_reprint: int = 0,
    charging_pay: int = 0,
    up_close_reply: bool = False,
    up_selection_reply: bool = False,
    up_close_danmu: bool = False,
    limit: int = 3,
    desc_v2: Sequence[Union[Credit, _CreditObject]] = [],
    dtime: Optional[int] = None,
    line: Optional[UploadLine] = None,
    extra_fields: Optional[str] = "",
    submit: Optional[str] = None,
    proxy: Optional[str] = None,
) -> None:
    """上传视频并投稿。

    :param video_path: 视频文件路径，每个文件是一个分P
    :param cookie_file: cookie 文件路径
    :param title: 标题
    :param tid: 分区
    :param tid_v2: 新版分区
    :param tag: 标签，英文逗号分隔
    :param copyright: 1 自制，2 转载
    :param source: 转载来源
    :param desc: 简介
    :param dynamic: 空间动态
    :param cover: 封面图片路径，空串表示不设置
    :param dolby: 杜比音效，0 关闭，1 开启
    :param lossless_music: Hi-Res 无损音质，0 关闭，1 开启
    :param no_reprint: 禁止转载，0 允许，1 禁止
    :param charging_pay: 充电专属，0 关闭，1 开启
    :param up_close_reply: 关闭评论
    :param up_selection_reply: 开启精选评论
    :param up_close_danmu: 关闭弹幕
    :param limit: 单个文件的并发上传数
    :param desc_v2: 带 @ 的简介，元素见 `Credit`；也接受带 `type_id` / `raw_text` / `biz_id` 属性的对象
    :param dtime: 定时发布的 10 位时间戳，须在提交后 2 小时到 15 天之间
    :param line: 上传线路，`None` 时自动测速
    :param extra_fields: 追加到投稿请求里的 JSON 对象字符串；`None` 或空串表示没有
    :param submit: 投稿接口：`app`（默认）、`web`、`bcutandroid`，不区分大小写，无法识别时用 `app`
    :param proxy: 代理
    :raises StreamGearsError: 登录、上传或投稿失败
    :raises ValueError: `extra_fields` 不是合法的 JSON 对象
    :raises TypeError: `desc_v2` 元素缺少必填字段
    """

@deprecated("config_bindings() always returns the built-in default configuration and will be removed.")
def config_bindings() -> _ConfigState:
    """总是返回内置的默认配置；运行中的服务不读写它。"""

def main_loop() -> None:
    """`biliup` 命令行入口，参数取自 `sys.argv`。无参数或 `start` 时按 `server` 运行。

    :raises SystemExit: 命令行参数错误
    :raises RuntimeError: 服务运行出错
    """
