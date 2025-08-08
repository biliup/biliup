from typing import Dict, List, Optional, Callable
from enum import Enum

from .pyobject import Segment, Credit


def download(url: str,
             header_map: Dict[str, str],
             file_name: str,
             segment: Segment,
             proxy: Optional[str]) -> None:
    """
    下载视频

    :param str url: 视频地址
    :param Dict[str, str] header_map: HTTP请求头
    :param str file_name: 文件名格式
    :param Segment segment: 视频分段设置
    :param Optional[str] proxy: 代理
    """


def download_with_callback(url: str,
               header_map: Dict[str, str],
               file_name: str,
               segment: Segment,
               file_name_callback_fn: Callable[[str], None],
               proxy: Optional[str]) -> None:
    """
    下载视频

    :param str url: 视频地址
    :param Dict[str, str] header_map: HTTP请求头
    :param str file_name: 文件名格式
    :param Segment segment: 视频分段设置
    :param Callable[[str], None] file_name_callback_fn: 回调已下载完成文件名
    :param Optional[str] proxy: 代理
    """


def login_by_cookies(proxy: Optional[str]) -> bool:
    """
    cookie登录

    :param Optional[str] proxy: 代理
    :return: 是否登录成功
    """


def send_sms(country_code: int, phone: int, proxy: Optional[str]) -> str:
    """
    发送短信验证码

    :param int country_code: 国家/地区代码
    :param int phone: 手机号
    :param Optional[str] proxy: 代理
    :return: 短信登录JSON信息
    """


def login_by_sms(code: int, ret: str, proxy: Optional[str]) -> bool:
    """
    短信登录

    :param int code: 验证码
    :param str ret: 短信登录JSON信息
    :param Optional[str] proxy: 代理
    :return: 是否登录成功
    """


def get_qrcode(proxy: Optional[str]) -> str:
    """
    获取二维码

    :param Optional[str] proxy: 代理
    :return: 二维码登录JSON信息
    """


def login_by_qrcode(ret: str, proxy: Optional[str]) -> bool:
    """
    二维码登录

    :param str ret: 二维码登录JSON信息
    :param Optional[str] proxy: 代理
    :return: 是否登录成功
    """


def login_by_web_cookies(sess_data: str, bili_jct: str, proxy: Optional[str]) -> bool:
    """
    网页Cookie登录1

    :param str sess_data: SESSDATA
    :param str bili_jct: bili_jct
    :param Optional[str] proxy: 代理
    :return: 是否登录成功
    """


def login_by_web_qrcode(sess_data: str, dede_user_id: str, proxy: Optional[str]) -> bool:
    """
    网页Cookie登录2

    :param str sess_data: SESSDATA
    :param str dede_user_id: DedeUserID
    :param Optional[str] proxy: 代理
    :return: 是否登录成功
    """


class UploadLine(Enum):
    """上传线路"""

    Bda2 = 1
    """百度云"""

    Qn = 2
    """七牛"""

    Bda = 3
    """百度云海外"""

    Tx = 4
    """腾讯云EO"""

    Txa = 5
    """腾讯云EO海外"""

    Bldsa = 6
    """Bilibili大陆动态加速"""

    Alia = 7
    """阿里云海外"""


def upload(video_path: List[str],
           cookie_file: str,
           title: str,
           tid: int,
           tag: str,
           copyright: int,
           source: str,
           desc: str,
           dynamic: str,
           cover: str,
           dolby: int,
           lossless_music: int,
           no_reprint: int,
           charging_pay: int,
           limit: int,
           desc_v2: List[Credit],
           dtime: Optional[int],
           line: Optional[UploadLine],
           extra_fields: Optional[str],
           submit: Optional[str],
           proxy: Optional[str]) -> None:

    """
    上传视频稿件

    :param List[str] video_path: 视频文件路径
    :param str cookie_file: cookie文件路径
    :param str title: 视频标题
    :param int tid: 投稿分区
    :param str tag: 视频标签, 英文逗号分隔多个tag
    :param int copyright: 是否转载, 1-自制 2-转载
    :param str source: 转载来源
    :param str desc: 视频简介
    :param str dynamic: 空间动态
    :param str cover: 视频封面
    :param int dolby: 是否开启杜比音效, 0-关闭 1-开启
    :param int lossless_music: 是否开启Hi-Res, 0-关闭 1-开启
    :param int no_reprint: 是否禁止转载, 0-允许 1-禁止
    :param int charging_pay: 是否开启充电, 0-关闭 1-开启
    :param int limit: 单视频文件最大并发数
    :param List[Credit] desc_v2: 视频简介v2
    :param Optional[dtime] int dtime: 定时发布时间, 距离提交大于2小时小于15天, 格式为10位时间戳
    :param Optional[UploadLine] line: 上传线路
    :param Optional[ExtraFields] line: 上传额外参数
    :param Optional[str] submit: 提交接口, 可选值: BCutAndroid, App（默认）
    :param Optional[str] proxy: 代理
    """
