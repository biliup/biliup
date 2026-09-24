import typing as _typing

from . import stream_gears
from .stream_gears import *
from .stream_gears import PySegment

__doc__ = stream_gears.__doc__
# Literal so type checkers see the exports; the tests keep it in sync with the
# native module's `__all__`.
__all__ = [
    "Credit",
    "PySegment",
    "Segment",
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

Segment = PySegment


class _CreditRequired(_typing.TypedDict):
    type: int
    raw_text: str


class Credit(_CreditRequired, total=False):
    """`upload` 的 `desc_v2` 元素：`type` 为 1 时是普通文本，为 2 时 `raw_text` 是被 @ 的用户名、`biz_id` 是其 mid。"""

    biz_id: _typing.Optional[str]
