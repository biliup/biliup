import time
import json
import re
import math
import random
import hmac
import hashlib

from typing import Dict, List, Optional
from httpx import RequestError, HTTPStatusError

from biliup.common.util import client
from biliup.config import config
from biliup.plugins import match1, logger, wbi, json_loads
from biliup.Danmaku import DanmakuClient
from biliup.engine.decorators import Plugin
from biliup.engine.download import DownloadBase

BILIBILI_API = "https://api.bilibili.com"
BILILIVE_API = "https://api.live.bilibili.com"
STREAM_NAME_REGEXP = r"/live-bvc/\d+/(live_[^/\.]+)"
WBI_WEB_LOCATION = "444.8"

@Plugin.download(regexp=r'https?://(b23\.tv|live\.bilibili\.com)')
class Bililive(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.__cookies: Dict[str, str] = dict()
        self.live_start_time: int = 0
        self.bilibili_danmaku: bool = config.get('bilibili_danmaku', False)
        self.bilibili_danmaku_detail: bool = config.get('bilibili_danmaku_detail', False)
        self.bilibili_danmaku_raw: bool = config.get('bilibili_danmaku_raw', False)
        self.__real_room_id: Optional[int] = None
        self.__login_mid: int = 0
        self.bili_cookie: Optional[str] = config.get('user', {}).get('bili_cookie')
        self.bili_cookie_file: Optional[str] = config.get('user', {}).get('bili_cookie_file')
        self.bili_qn: int = int(config.get('bili_qn', 20000))
        self.bili_protocol: str = config.get('bili_protocol', 'stream')
        self.bili_cdn: List[str] = config.get('bili_cdn', [])
        self.bili_hls_timeout: int = config.get('bili_hls_transcode_timeout', 60)
        self.bili_api_list: List[str] = [
            normalize_url(config.get('bili_liveapi', BILILIVE_API)),
            normalize_url(config.get('bili_fallback_api', BILILIVE_API)).rstrip('/'),
        ]
        self.bili_force_source: bool = config.get('bili_force_source', False)
        self.bili_anonymous_origin: bool = config.get('bili_anonymous_origin', False)
        self.bili_ov2cn: bool = config.get('bili_ov2cn', False)
        self.bili_normalize_cn204: bool = config.get('bili_normalize_cn204', False)
        self.cn01_sids: List[str] = config.get('bili_replace_cn01', [])
        self.bili_cdn_fallback: bool = config.get('bili_cdn_fallback', False)

    async def acheck_stream(self, is_check=False):

        if int(time.time()) - wbi.last_update >= wbi.UPDATE_INTERVAL:
            await self.update_wbi()

        if "b23.tv" in self.url:
            try:
                resp = await client.get(self.url, follow_redirects=False)
                if resp.status_code not in {301, 302} or resp.next_request is None:
                    raise Exception("不支持的链接")
                url = str(resp.next_request.url)
                if "live.bilibili" not in url:
                    raise Exception("不支持的链接")
                self.url = url
            except Exception as e:
                logger.error(f"{self.plugin_msg}: {e}")
                return False

        self.fake_headers['referer'] = self.url
        room_id = match1(self.url, r'bilibili.com/(\d+)')
        client.headers.update(self.fake_headers)
        await self.load_cookies()

        # 获取直播状态与房间标题
        try:
            params = {
                "room_id": room_id,
                "web_location": WBI_WEB_LOCATION,
            }
            wbi.sign(params)
            room_info = await client.get(
                f"{BILILIVE_API}/xlive/web-room/v1/index/getInfoByRoom",
                params=params,
                headers=self.fake_headers,
                cookies=self.__cookies
            )
            room_info.raise_for_status()
            room_info = json_loads(room_info.text)
        except Exception as e:
            logger.error(f"{self.plugin_msg}: {e}", exc_info=True)
            return False
        if room_info['code'] != 0:
            logger.error(f"{self.plugin_msg}: {room_info}")
            return False
        else:
            room_info = room_info['data']
        if room_info['room_info']['live_status'] != 1:
            logger.debug(f"{self.plugin_msg}: 未开播")
            self.raw_stream_url = None
            return False

        self.live_cover_url = room_info['room_info']['cover']
        self.room_title = room_info['room_info']['title']
        self.__real_room_id = room_info['room_info']['room_id']
        live_start_time = room_info['room_info']['live_start_time']
        special_type = room_info['room_info']['special_type'] # 0: 公开直播, 1: 付费公开直播
        if live_start_time > self.live_start_time:
            self.live_start_time = live_start_time
            is_new_live = True
        else:
            is_new_live = False

        if is_check:
            return True

        # 复用原画 m3u8 流
        if  self.raw_stream_url is not None \
            and ".m3u8" in self.raw_stream_url \
            and self.bili_qn >= 10000 \
            and not is_new_live:
            url = await self.acheck_url_healthy(self.raw_stream_url)
            if url is not None:
                logger.debug(f"{self.plugin_msg}: 复用 {url}")
                return True
            else:
                self.raw_stream_url = None


        stream_urls = await self.aget_stream(self.bili_qn, self.bili_protocol, special_type)
        if not stream_urls:
            if self.bili_protocol == 'hls_fmp4':
                if int(time.time()) - live_start_time <= self.bili_hls_timeout:
                    logger.warning(f"{self.plugin_msg}: 暂未提供 hls_fmp4 流，等待下一次检测")
                    return False
                else:
                    # 回退首个可用格式
                    stream_urls = await self.aget_stream(self.bili_qn, 'stream', special_type)
            else:
                logger.error(f"{self.plugin_msg}: 获取{self.bili_protocol}流失败")
                return False

        target_quality_stream = stream_urls.get(
            str(self.bili_qn), next(iter(stream_urls.values()))
        )
        stream_url = {}
        if self.bili_cdn is not None:
            for cdn in self.bili_cdn:
                stream_info = target_quality_stream.get(cdn)
                if stream_info is not None:
                    current_cdn = cdn
                    stream_url = stream_info['url']
                    break
        if not stream_url:
            current_cdn, stream_info = next(iter(target_quality_stream.items()))
            stream_url = stream_info['url']
            logger.debug(f"{self.plugin_msg}: 使用 {current_cdn} 流")

        self.raw_stream_url = self.normalize_cn204(
            f"{stream_url['host']}{stream_url['base_url']}{stream_url['extra']}"
        )

        # 替换 cn-gotcha01 节点
        if self.cn01_sids and "cn-gotcha01" in current_cdn:
            if isinstance(self.cn01_sids, str):
                self.cn01_sids = self.cn01_sids.split(',')
            for sid in self.cn01_sids:
                new_url = f"https://{sid}.bilivideo.com{stream_url['base_url']}{stream_url['extra']}"
                new_url = await self.acheck_url_healthy(new_url)
                if new_url is not None:
                    self.raw_stream_url = new_url
                    break
                else:
                    logger.debug(f"{self.plugin_msg}: {sid} is not available")

        # 强制原画
        if self.bili_qn <= 10000 and self.bili_force_source:
            # 不处理 qn 20000
            if stream_info['suffix'] not in {'origin', 'uhd', 'maxhevc'}:
                __base_url = stream_url['base_url'].replace(f"_{stream_info['suffix']}", "")
                __sk = match1(stream_url['extra'], r'sk=([^&]+)')
                __extra = stream_url['extra'].replace(__sk, __sk[:32])
                __url = self.normalize_cn204(f"{stream_url['host']}{__base_url}{__extra}")
                if (await self.acheck_url_healthy(__url)) is not None:
                    self.raw_stream_url = __url
                    logger.info(f"{self.plugin_msg}: force_source 处理 {current_cdn} 成功 {stream_info['stream_name']}")
                else:
                    logger.debug(
                        f"{self.plugin_msg}: force_source 处理 {current_cdn} 失败 {stream_info['stream_name']}"
                    )

        # 回退
        if self.bili_cdn_fallback:
            __url = await self.acheck_url_healthy(self.raw_stream_url)
            if __url is None:
                for cdn, stream_info in target_quality_stream.items():
                    stream_url = stream_info['url']
                    __fallback_url = self.normalize_cn204(
                        f"{stream_url['host']}{stream_url['base_url']}{stream_url['extra']}"
                    )
                    try:
                        __url = await self.acheck_url_healthy(__fallback_url)
                        if __url is not None:
                            self.raw_stream_url = __url
                            logger.info(f"{self.plugin_msg}: cdn_fallback 回退到 {cdn} - {__fallback_url}")
                            break
                    except Exception as e:
                        logger.error(f"{self.plugin_msg}: cdn_fallback {e} - {__fallback_url}")
                        continue
                else:
                    logger.error(f"{self.plugin_msg}: 所有 cdn 均不可用")
                    self.raw_stream_url = None
                    return False
            else:
                self.raw_stream_url = __url

        return True

    def danmaku_init(self):
        if self.bilibili_danmaku:
            self.danmaku = DanmakuClient(
                self.url, self.gen_download_filename(), {
                    'room_id': self.__real_room_id,
                    'cookie': self.__cookies,
                    'detail': self.bilibili_danmaku_detail,
                    'raw': self.bilibili_danmaku_raw,
                    'uid': self.__login_mid
                }
            )

    async def get_play_info(self, api: str, qn: int = 10000) -> dict:
        full_url = f"{api}/xlive/web-room/v2/index/getRoomPlayInfo"
        try:
            params = {
                'room_id': str(self.__real_room_id),
                # 'no_playurl': '0',
                # 'mask': '1',
                'qn': str(qn),
                'platform': 'html5',  # 平台名称，web, html5, android, ios
                'protocol': '0,1',  # 流协议，0: http_stream(flv), 1: http_hls
                'format': '0,1,2',  # 编码格式，0: flv, 1: ts, 2: fmp4
                'codec': '0',  # 编码器，0: avc, 1: hevc, 2: av1
                # 'ptype': '8', # P2P配置，-1: disable, 8: WebRTC, 8192: MisakaTunnel
                'dolby': '5', # 杜比格式，5: 杜比音频
                # 'panorama': '1', # 全景(不支持 html5)
                # 'hdr_type': '0,1', # HDR类型(不支持 html5)，0: SDR, 1: PQ
                # 'req_reason': '0', # 请求原因，0: Normal, 1: PlayError
                # 'http': '1', # 优先 http 协议
                'web_location': WBI_WEB_LOCATION,
            }
            wbi.sign(params)
            api_res = await client.get(
                full_url, params=params, headers=self.fake_headers
            )
            api_res = json.loads(api_res.text)
            if api_res['code'] != 0:
                logger.error(f"{self.plugin_msg}: {api} 返回内容错误: {api_res}")
                return {}
            return api_res['data']
        except json.JSONDecodeError:
            logger.error(f"{self.plugin_msg}: {api} 返回内容错误: {api_res.text}")
        except Exception as e:
            logger.error(f"{self.plugin_msg}: {api} 获取 play_info 失败 -> {e}", exc_info=True)
        return {}

    async def get_master_m3u8(self, api: str, stream_name: str) -> dict:
        """
        获取 Master playlist
        :param api: 接口地址
        :param stream_name: 流名称，用于选择 Codec
        :return: 解析后的流信息
        """
        full_url = f"{api}/xlive/play-gateway/master/url"
        params = {
            "cid": self.__real_room_id,
            "mid": self.__login_mid,
            "pt": "html5", # platform
            # "p2p_type": "-1",
            # "net": 0, # 网络类型；0: 有线, 1: Wlan
            # "free_type": 0,
            # "build": 0,
            # "feature": 2 # 1:? 2:?
            "qn": self.bili_qn,
        }
        if stream_name:
            params['stream_name'] = stream_name
        try:
            m3u8_res = await client.get(
                full_url, params=params, headers=self.fake_headers
            )
            if m3u8_res.status_code == 200 and m3u8_res.text.startswith("#EXTM3U"):
                return self.parse_master_m3u8(m3u8_res.text)
        except Exception as e:
            logger.error(f"{self.plugin_msg}: {api} 获取 m3u8 失败 -> {e}", exc_info=True)
        return {}

    async def aget_stream(self, qn: int = 10000, protocol: str = 'stream', special_type: int = 0) -> dict:
        """
        :param qn: 目标画质
        :param protocol: 流协议
        :param special_type: 特殊直播类型
        :return: 流信息
        """
        stream_urls = {}
        for api in self.bili_api_list:
            play_info = await self.get_play_info(api, qn)
            if not play_info or check_areablock(play_info):
                # logger.error(f"{self.plugin_msg}: {api} 返回内容错误: {play_info}")
                continue
            streams = play_info['playurl_info']['playurl']['stream']
            if protocol == 'hls_fmp4':
                if self.bili_anonymous_origin:
                    if special_type in play_info['all_special_types'] and not self.__login_mid:
                        logger.warn(f"{self.plugin_msg}: 特殊直播{special_type}")
                    else:
                        stream_urls = await self.get_master_m3u8(api)
                        if stream_urls:
                            break
                # 处理 API 信息
                stream = streams[1] if len(streams) > 1 else streams[0]
                for format in stream['format']:
                    if format['format_name'] == 'fmp4':
                        stream_urls = self.parse_stream_url(format['codec'][0])
                        # fmp4 可能没有原画
                        if qn == 10000 and qn in stream_urls.keys():
                            break
                        else:
                            stream_urls = {}
            else:
                stream_urls = self.parse_stream_url(streams[0]['format'][0]['codec'][0])
            if stream_urls:
                break
        # 空字典照常返回，重试交给上层方法处理
        return stream_urls

    async def get_user_status(self) -> dict:
        try:
            nav_res = await client.get(
                'https://api.bilibili.com/x/web-interface/nav',
                headers=self.fake_headers
            )
            nav_res.raise_for_status()
            nav_res = json.loads(nav_res.text)
            if (
                nav_res['code'] == 0 or
                (nav_res['code'] == -101 and nav_res['message'] == '账号未登录')
            ):
                return nav_res['data']
            logger.error(f"{self.plugin_msg}: 获取 nav 失败-{nav_res}")
        except:
            logger.error(f"{self.plugin_msg}: 获取 nav 失败", exc_info=True)
        return {}

    async def update_wbi(self):
        def _extract_key(url):
            if not url:
                return None
            slash = url.rfind('/')
            dot = url.find('.', slash)
            if slash == -1 or dot == -1:
                return None
            return url[slash + 1:dot]
        data = await self.get_user_status()
        wbi_key = data.get('wbi_img')
        if wbi_key:
            img_key = _extract_key(wbi_key.get('img_url'))
            sub_key = _extract_key(wbi_key.get('sub_url'))
            if img_key and sub_key:
                wbi.update_key(img_key, sub_key)
            else:
                logger.warning(f"img_key-{img_key}, sub_key-{sub_key}")
        else:
            logger.warning(f"Can not get wbi key by {data}")

    async def check_login_status(self) -> int:
        """
        检查B站登录状态
        :return: 当前登录用户 mid
        """
        try:
            data = await self.get_user_status()
            if data.get('isLogin'):
                logger.info(f"{self.plugin_msg}: Login test -> 用户名：{data['uname']}, mid: {data['mid']}")
                return data['mid']
            else:
                logger.warning(f"{self.plugin_msg}: 未登录，或将只能录制到最低画质。")
        except Exception as e:
            logger.error(f"{self.plugin_msg}: 登录态校验失败 {e}")
        return 0

    def normalize_cn204(self, url: str) -> str:
        if self.bili_normalize_cn204 and "cn-gotcha204" in url:
            return re.sub(r"(?<=cn-gotcha204)-[1-4]", "", url, 1)
        return url

    def parse_stream_url(self, *args) -> dict:
        suffix_regexp = r'suffix=([^&]+)'
        if isinstance(args[0], str):
            url = args[0]
            host = "https://" + match1(url, r'https?://([^/]+)')
            stream_url = {
                'host': host if not self.bili_ov2cn else host.replace("ov-", "cn-"),
                'base_url': url.split("?")[0].split(host)[1] + "?",
                'extra': url.split("?")[1]
            }
            return {
                'url': stream_url,
                'stream_name': match1(url, STREAM_NAME_REGEXP),
                'suffix': match1(url, suffix_regexp)
            }
        elif isinstance(args[0], dict):
            streams = {}
            current_qn = args[0]['current_qn']
            streams.setdefault(current_qn, {})
            base_url = args[0]['base_url']
            for info in args[0]['url_info']:
                cdn_name = match1(info['extra'], r'cdn=([^&]+)')
                stream_url = {
                    'host': info['host'] if not self.bili_ov2cn else info['host'].replace("ov-", "cn-"),
                    'base_url': base_url,
                    'extra': info['extra']
                }
                streams[current_qn].setdefault(cdn_name, {
                    'url': stream_url,
                    'stream_name': match1(base_url, STREAM_NAME_REGEXP),
                    'suffix': match1(info['extra'], suffix_regexp)
                })
            return streams
        return {}

    def parse_master_m3u8(self, m3u8_content: str) -> dict:
        """
        Returns:
            {
                "qn值": {
                    "cdn名称": {
                        "url": parsed_stream_url,
                        "stream_name": "流名称",
                        "suffix": "二压后缀"
                    }
                }
            }
        """
        lines = m3u8_content.strip().splitlines()
        current_qn = None
        result = {}

        if not lines[0].startswith('#EXTM3U'):
            raise ValueError('Invalid m3u8 file')

        for line in lines:
            if line.startswith('#EXT-X-STREAM-INF:'):
                codec = match1(line, r'CODECS="([^"]+)"')
                current_qn = match1(line, r'BILI-QN=(\d+)')

                if codec and current_qn:
                    if 'avc' in codec.lower():
                        result.setdefault(current_qn, {})
                    else:
                        current_qn = None

            elif line.startswith('http') and current_qn is not None:
                cdn_name = match1(line, r'cdn=([^&]+)')
                if cdn_name:
                    result[current_qn].setdefault(cdn_name, self.parse_stream_url(line))

        return dict(sorted(result.items(), key=lambda x: int(x[0]), reverse=True))

    async def load_cookies(self):
        if self.bili_cookie:
            for cookie in self.bili_cookie.split(';'):
                name, value = cookie.split('=', 1)
        if self.bili_cookie_file:
            try:
                with open(self.bili_cookie_file, encoding='utf-8') as stream:
                    _cookies = json.load(stream)["cookie_info"]["cookies"]
                    for i in _cookies:
                        self.__cookies[i['name']] = i['value']
            except (json.JSONDecodeError, FileNotFoundError, KeyError):
                logger.exception("load_cookies error")
        self.__login_mid = await self.check_login_status()
        if self.__login_mid == 0:
            logger.debug(f"{self.plugin_msg}: 登录校验失败，清空 cookie")
            self.__cookies = {}
        self.__cookies.update(await (bililive_utils.get_risk_cookies(self.__cookies, self.__login_mid)))
        # print(self.__cookies)

class BililiveUtils:
    def __init__(self):
        # _cookie_store: {mid: {"cookies": dict, "expires": timestamp}}
        self._cookie_store = {}

    async def get_risk_cookies(self, user_cookies: Optional[dict] = None, mid: int = 0) -> dict:
        """
        获取风控相关 cookies
        :param user_cookies: 已登录用户 cookies
        :param mid: 用户 mid，未登录时为 0
        :return: 已添加风控相关 cookies
        """
        now = int(time.time())
        entry = self._cookie_store.get(mid)
        if entry and entry["expires"] > now:
            return entry["cookies"]
        full_cookies, expires = await self._refresh_cookies(user_cookies)
        self._cookie_store[mid] = {"cookies": full_cookies, "expires": expires}
        return full_cookies

    async def get_all_cookies(self) -> dict:
        return self._cookie_store

    async def _refresh_cookies(self, cookies: Optional[dict] = None):
        # 更新风控相关 cookies
        expire_time = 60 * 60 * 6
        full_cookies = {} if cookies is None else cookies
        full_cookies.update(self._gen_b_nut())
        full_cookies.update(self._gen_b_lsid())
        full_cookies.update(self._gen_uuid())
        full_cookies.update(await self._get_buvid())
        full_cookies.update(await self._get_bili_ticket(cookies))
        return full_cookies, int(time.time()) + expire_time

    @staticmethod
    async def _get_buvid(cookies: Optional[dict] = None) -> Dict[str, str]:
        """
        :param cookies: 任意用户 Cookies（可选）
        :return: 获取到的 buvid3 和 buvid4
        """
        if not cookies:
            cookies = {}
        result = {}
        full_url = f"{BILIBILI_API}/x/frontend/finger/spi"
        try:
            resp = await client.get(full_url, cookies=cookies)
            resp.raise_for_status()
            resp = json_loads(resp.text)
            if resp['code'] != 0:
                raise ValueError(f"Error content {resp}")
            result = {
                'buvid3': resp['data']['b_3'],
                'buvid4': resp['data']['b_4'],
            }
        except RequestError as e:
            logger.warning(f"请求 {full_url} 失败 -> {e}")
        except (ValueError, HTTPStatusError) as e:
            logger.error(f"获取 buvid 失败 {e}")
        return result

    @staticmethod
    async def _get_bili_ticket(cookies: Optional[dict] = None) -> Dict[str, str]:
        """
        :param cookies: 已登录用户的Cookies（可选）
        :return: 包含 bili_ticket 和 bili_ticket_expires 的字典
        """
        result = {}
        if not cookies:
            cookies = {}
        context = {
            'ts': str(int(time.time())),
        }
        full_url = f"{BILIBILI_API}/bapis/bilibili.api.ticket.v1.Ticket/GenWebTicket"
        try:
            sign_data = ''.join(f"{k}{v}" for k, v in context.items())
            params = {
                'key_id': 'ec02',
                'hexsign': hmac_sha256("XgwSnGZ1p", sign_data),
                'csrf': cookies.get('bili_jct', ''),
                **{f'context[{k}]': v for k, v in context.items()}
            }
            resp = await client.post(full_url, params=params, cookies=cookies)
            resp.raise_for_status()
            resp = json_loads(resp.text)
            if resp['code'] != 0:
                raise ValueError(f"{resp}")
            result = {
                'bili_ticket': resp['data']['ticket'],
                'bili_ticket_expires': str(int(resp['data']['created_at'] + resp['data']['ttl']))
            }
        except RequestError as e:
            logger.warning(f"请求 {full_url} 失败 -> {e}")
        except (ValueError, HTTPStatusError) as e:
            logger.error(f"获取 buvid 失败 {e}")
        return result

    @staticmethod
    def _gen_b_lsid(timestamp: Optional[int]=None) -> Dict[str, str]:
        """
        生成 b_lsid cookie
        :param timestamp: 时间戳（可选）
        :return: 包含 b_lsid 的字典
        """
        if not timestamp:
            timestamp = int(time.time())
        # 生成8位随机字符串
        random_part = ''.join(format(math.ceil(random.random() * 16), 'X') for _ in range(8))
        # 确保长度为8，不足则补0
        random_part = random_part.zfill(8)
        # 时间戳部分
        time_part = format(timestamp, 'X')
        b_lsid = f"{random_part}_{time_part}"
        return {'b_lsid': b_lsid}

    @staticmethod
    def _gen_uuid() -> Dict[str, str]:
        """
        生成 _uuid cookie
        :return: 包含 _uuid 的字典
        """
        # 生成UUID各部分
        parts = []
        lengths = [8, 4, 4, 4, 12]
        for length in lengths:
            hex_str = ''.join(format(int(random.random() * 16), 'x') for _ in range(length))
            # 确保长度正确，不足则补0
            hex_str = hex_str.zfill(length)
            parts.append(hex_str)
        # 时间戳后缀
        timestamp_suffix = str(int(time.time()) % 100).zfill(5)
        # 组合UUID
        uuid_value = f"{'-'.join(parts)}{timestamp_suffix}infoc"
        return {'_uuid': uuid_value}

    @staticmethod
    def _gen_b_nut() -> Dict[str, str]:
        """
        生成 b_nut cookie
        :return: 包含 b_nut 的字典
        """
        return {'b_nut': str(int(time.time()))}

def pad_string(string: str, target_length: int) -> str:
    """Pad string with leading zeros to reach target length"""
    padding = "0" * max((target_length - len(string)), 0)
    return f"{padding}{string}"

# Copy from room-player.js
def check_areablock(data):
    """
    :return: True if area block
    """
    if not data['playurl_info']['playurl']:
        logger.error('Sorry, bilibili is currently not available in your country according to copyright restrictions.')
        logger.error('非常抱歉，根据版权方要求，您所在的地区无法观看本直播')
        return True
    return False

def normalize_url(url: str) -> str:
    return BILILIVE_API if not url else (url if url.startswith(('http://', 'https://')) else 'http://' + url).rstrip('/')

def hmac_sha256(key: str, data: str) -> str:
    return hmac.new(
        key.encode('utf-8'), data.encode('utf-8'), hashlib.sha256
    ).hexdigest()

bililive_utils = BililiveUtils()

# if __name__ == "__main__":
#     import asyncio
#     async def main():
#         # bililive_utils = BililiveUtils()
#         headers = {
#             'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36',
#             'Referer': 'https://live.bilibili.com',
#             'Origin': 'https://live.bilibili.com',
#             'Accept': '*/*',
#             'Cache-Control': 'no-cache',
#             'Host': 'api.bilibili.com',
#             'Connection': 'keep-alive'
#         }
#         client.headers.update(headers)
#         cookies = await bililive_utils.get_risk_cookies()
#         print(cookies)
#         all_cookies = await bililive_utils.get_all_cookies()
#         print(all_cookies)
#     asyncio.run(main())