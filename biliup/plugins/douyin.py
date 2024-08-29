import json
from typing import Optional
from urllib.parse import unquote, urlparse, parse_qs, urlencode, urlunparse

import requests

from biliup.common.util import client
from . import logger, match1, random_user_agent
from biliup.config import config
from biliup.Danmaku import DanmakuClient
from ..common.tools import NamedLock
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m|live|v)\.)?douyin\.com')
class Douyin(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.douyin_danmaku = config.get('douyin_danmaku', False)
        self.fake_headers['user-agent'] = DouyinUtils.DOUYIN_USER_AGENT
        self.fake_headers['referer'] = "https://live.douyin.com/"
        self.fake_headers['cookie'] = config.get('user', {}).get('douyin_cookie', '')
        self.__web_rid = None # 网页端房间号 或 抖音号
        self.__room_id = None # 单场直播的直播房间
        self.__sec_uid = None

    async def acheck_stream(self, is_check=False):

        if "ttwid" not in self.fake_headers['cookie']:
            self.fake_headers['Cookie'] = f'ttwid={DouyinUtils.get_ttwid()};{self.fake_headers["cookie"]}'

        if "v.douyin" in self.url:
            try:
                resp = await client.get(self.url, headers=self.fake_headers, follow_redirects=False)
            except:
                return False
            try:
                if resp.status_code not in {301, 302}:
                    raise
                next_url = str(resp.next_request.url)
                if "webcast.amemv" not in next_url:
                    raise
            except:
                logger.error(f"{self.plugin_msg}: 不支持的链接")
                return False
            self.__sec_uid = match1(next_url, r"sec_user_id=(.*?)&")
            self.__room_id = match1(next_url.split("?")[0], r"(\d+)")
        elif "/user/" in self.url:
            sec_uid = self.url.split("user/")[1].split("?")[0]
            if len(sec_uid) == 55:
                self.__sec_uid = sec_uid
            else:
                try:
                    user_page = (await client.get(self.url, headers=self.fake_headers)).text
                    user_page_data = unquote(
                        user_page.split('<script id="RENDER_DATA" type="application/json">')[1].split('</script>')[0])
                    web_rid = match1(user_page_data, r'"web_rid":"([^"]+)"')
                    if not web_rid:
                        logger.debug(f"{self.plugin_msg}: 未开播")
                        return False
                    self.__web_rid = web_rid
                except (KeyError, IndexError):
                    logger.error(f"{self.plugin_msg}: 房间号获取失败，请检查Cookie设置")
                    return False
                except:
                    logger.exception(f"{self.plugin_msg}: 房间号获取失败")
                    return False
        else:
            web_rid = self.url.split('douyin.com/')[1].split('/')[0].split('?')[0]
            if web_rid[0] == "+":
                web_rid = web_rid[1:]
            self.__web_rid = web_rid

        try:
            _room_info = None
            if self.__web_rid:
                _room_info = await self.get_web_room_info(self.__web_rid)
                if _room_info:
                    if not _room_info['data'].get('user'):
                        # 可能是用户被封禁
                        raise Exception(f"{str(_room_info)}")
                    self.__sec_uid = _room_info['data']['user']['sec_uid']
            # PCWeb 端无流 或 没有提供 web_rid
            if not _room_info.get('data', {}).get('data'):
                _room_info = await self.get_room_info(self.__sec_uid, self.__room_id)
                if _room_info['data'].get('room', {}).get('owner'):
                    self.__web_rid = _room_info['data']['room']['owner']['web_rid']
            try:
                # 出现异常不用提示，直接到 移动网页 端获取
                room_info = _room_info['data']['data'][0]
            except (KeyError, IndexError):
                # 如果 移动网页 端也没有数据，当做未开播处理
                room_info = _room_info['data'].get('room', {})
                # 当做未开播处理
                # if not room_info:
                #     logger.info(f"{self.plugin_msg}: 获取直播间信息失败 {_room_info}")
            if room_info.get('status') != 2:
                logger.debug(f"{self.plugin_msg}: 未开播")
                return False
            self.__room_id = room_info['id_str']
        except:
            logger.exception(f"{self.plugin_msg}: 获取直播间信息失败")
            return False

        if is_check:
            return True

        try:
            pull_data = room_info['stream_url']['live_core_sdk_data']['pull_data']
            if room_info['stream_url'].get('pull_datas') and config.get('douyin_extra_record', True):
                pull_data = next(iter(room_info['stream_url']['pull_datas'].values()))
            stream_data = json.loads(pull_data['stream_data'])['data']
        except:
            logger.exception(f"{self.plugin_msg}: 加载直播流失败")
            return False

        # 原画origin 蓝光uhd 超清hd 高清sd 标清ld 流畅md 仅音频ao
        quality_items = ['origin', 'uhd', 'hd', 'sd', 'ld', 'md']
        quality = config.get('douyin_quality', 'origin')
        if quality not in quality_items:
            quality = quality_items[0]
        try:
            # 如果没有这个画质则取相近的 优先低清晰度
            if quality not in stream_data:
                # 可选的清晰度 含自身
                optional_quality_items = [x for x in quality_items if x in stream_data.keys() or x == quality]
                # 自身在可选清晰度的位置
                optional_quality_index = optional_quality_items.index(quality)
                # 自身在所有清晰度的位置
                quality_index = quality_items.index(quality)
                # 高清晰度偏移
                quality_left_offset = None
                # 低清晰度偏移
                quality_right_offset = None

                if optional_quality_index + 1 < len(optional_quality_items):
                    quality_right_offset = quality_items.index(
                        optional_quality_items[optional_quality_index + 1]) - quality_index

                if optional_quality_index - 1 >= 0:
                    quality_left_offset = quality_index - quality_items.index(
                        optional_quality_items[optional_quality_index - 1])

                # 取相邻的清晰度
                if quality_right_offset <= quality_left_offset:
                    quality = optional_quality_items[optional_quality_index + 1]
                else:
                    quality = optional_quality_items[optional_quality_index - 1]

            protocol = 'hls' if config.get('douyin_protocol') == 'hls' else 'flv'
            self.raw_stream_url = stream_data[quality]['main'][protocol]
            self.room_title = room_info['title']
        except:
            logger.exception(f"{self.plugin_msg}: 寻找清晰度失败")
            return False
        return True

    def danmaku_init(self):
        if self.douyin_danmaku:
            content = {
                'web_rid': self.__web_rid,
                'sec_uid': self.__sec_uid,
                'room_id': self.__room_id,
            }
            try:
                import jsengine
                try:
                    jsengine.jsengine()
                    self.danmaku = DanmakuClient(self.url, self.gen_download_filename(), content)
                except jsengine.exceptions.RuntimeError as e:
                    extra_msg = "如需录制抖音弹幕，"
                    logger.error(f"\n{e}\n{extra_msg}请至少安装一个 Javascript 解释器，如 pip install quickjs")
            except:
                pass

    async def get_web_room_info(self, web_rid) -> dict:
        target_url = DouyinUtils.build_request_url(f"https://live.douyin.com/webcast/room/web/enter/?web_rid={web_rid}")
        web_info = (await client.get(target_url, headers=self.fake_headers)).json()
        return web_info

    async def get_room_info(self, sec_user_id, room_id) -> dict:
        if not sec_user_id:
            raise ValueError("sec_user_id is None")
        params = {
            'type_id': 0,
            'live_id': 1,
            'version_code': '99.99.99',
            'app_id': 1128,
            'room_id': room_id if room_id else 2, # 必要但不校验
            'sec_user_id': sec_user_id
        }
        info = (await client.get("https://webcast.amemv.com/webcast/room/reflow/info/",
                    params=params, headers=self.fake_headers)).json()
        return info



class DouyinUtils:
    # 抖音ttwid
    _douyin_ttwid: Optional[str] = None
    # DOUYIN_USER_AGENT = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/92.0.4515.159 Safari/537.36'
    DOUYIN_USER_AGENT = random_user_agent()
    DOUYIN_HTTP_HEADERS = {
        'User-Agent': DOUYIN_USER_AGENT
    }

    @staticmethod
    def get_ttwid() -> Optional[str]:
        with NamedLock("douyin_ttwid_get"):
            if not DouyinUtils._douyin_ttwid:
                page = requests.get("https://live.douyin.com/1-2-3-4-5-6-7-8-9-0", timeout=15)
                DouyinUtils._douyin_ttwid = page.cookies.get("ttwid")
            return DouyinUtils._douyin_ttwid

    @staticmethod
    def build_request_url(url: str) -> str:
        parsed_url = urlparse(url)
        existing_params = parse_qs(parsed_url.query)
        existing_params['aid'] = ['6383']
        existing_params['device_platform'] = ['web']
        existing_params['browser_language'] = ['zh-CN']
        existing_params['browser_platform'] = ['Win32']
        existing_params['browser_name'] = [DouyinUtils.DOUYIN_USER_AGENT.split('/')[0]]
        existing_params['browser_version'] = [DouyinUtils.DOUYIN_USER_AGENT.split(existing_params['browser_name'][0])[-1][1:]]
        new_query_string = urlencode(existing_params, doseq=True)
        new_url = urlunparse((
            parsed_url.scheme,
            parsed_url.netloc,
            parsed_url.path,
            parsed_url.params,
            new_query_string,
            parsed_url.fragment
        ))
        return new_url
