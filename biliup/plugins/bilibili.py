import time
import requests

from biliup.config import config
from . import match1, logger
from biliup.plugins.Danmaku import DanmakuClient
from ..engine.decorators import Plugin
from ..engine.download import DownloadBase

@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m|live)\.)?bilibili\.com')
class Bililive(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.fake_headers['referer'] = 'https://live.bilibili.com'
        self.bilibili_danmaku = config.get('bilibili_danmaku', False)
        self.live_time = 0

    def check_stream(self, is_check=False):

        official_api = "https://api.live.bilibili.com"
        room_id = match1(self.url, r'/(\d+)')
        qualityNumber = config.get('bili_qn', 10000)

        with requests.Session() as s:
            s.headers = self.fake_headers.copy()
            # 获取直播状态与房间标题
            info_by_room_url = f"{official_api}/xlive/web-room/v1/index/getInfoByRoom?room_id={room_id}"
            try:
                room_info = s.get(info_by_room_url, timeout=5).json()
            except Exception as e:
                logger.debug(e)
                logger.error(f"在连接到 {info_by_room_url} 时出现错误")
                return False
            if room_info['code'] != 0:
                logger.debug(f"Bililive-{room_id}: {room_info}")
                return False
            self.live_cover_url = room_info['data']['room_info']['cover']
            live_start_time = room_info['data']['room_info']['live_start_time']
            if room_info['data']['room_info']['live_status'] != 1:
                logger.debug(f"Bililive-{room_id}: 直播间未开播")
                return False
            if self.room_title is None:
                self.room_title = room_info['data']['room_info']['title']
            if live_start_time > self.live_time:
                self.live_time = live_start_time
                is_new_live = True
            else:
                is_new_live = False

        if is_check:
            return True

        s = requests.Session()
        self.fake_headers['cookie'] = (
            load_cookies(config.get('user', {}).get('bili_cookie_file')) or
            config.get('user', {}).get('bili_cookie')
        )
        s.headers = self.fake_headers
        user_data = do_login(s).get('data', {})
        is_login = user_data.get('isLogin', False)
        if not is_login:
            logger.debug(f"Bililive: Cookie 不存在或失效")
            logger.debug(self.fake_headers.pop('cookie', 'Bililive: Not Found Cookie'))
        else:
            logger.info(f"用户名：{user_data['uname']}, mid：{user_data['mid']}, isLogin：{is_login}")

        # 原画链接复用
        if self.raw_stream_url is not None \
            and qualityNumber >= 10000 \
            and not is_new_live:
            # 同一个 streamName 即可复用，与其他参数无关，目前没有报告说链接会超时失效。
            # 前面拿不到 streamName，目前使用开播时间判断
            if check_url_healthy(s, self.raw_stream_url):
                logger.debug(f"Bililive-{room_id}: 复用 {self.raw_stream_url}")
                s.close()
                return True

        protocol = config.get('bili_protocol', 'stream')
        perf_cdn = config.get('bili_perfCDN')
        bili_cdn_fallback = config.get('bili_cdn_fallback', True)
        force_source = config.get('bili_force_source', False)
        ov05_ip = config.get('bili_force_ov05_ip')
        main_api = config.get('bili_liveapi', official_api).rstrip('/')
        fallback_api = config.get('bili_fallback_api', main_api).rstrip('/')
        cn01_domains = config.get('bili_force_cn01_domains', '').split(",")

        params = {
            'room_id': room_id,
            'protocol': '0,1',# 0: http_stream, 1: http_hls
            'format': '0,1,2',# 0: flv, 1: ts, 2: fmp4
            'codec': '0', # 0: avc, 1: hevc, 2: av1
            'qn': qualityNumber,
            'platform': 'html5', # web, html5, android, ios
            # 'ptype': '8',
            'dolby': '5',
            # 'panorama': '1' # 全景(不支持 html5)
        }
        streamname_regexp = r"(live_\d+_\w+_\w+_?\w+?)" # 匹配 streamName

        try:
            play_info = get_play_info(s, main_api, params)
            if check_areablock(play_info['data']['playurl_info']['playurl']):
                logger.debug(f"{main_api} 返回 {play_info}")
                if fallback_api:
                    play_info = get_play_info(s, fallback_api, params)
                if check_areablock(play_info['data']['playurl_info']['playurl']):
                    logger.debug(f"{fallback_api} 返回 {play_info}")
                    return False
        except Exception as e:
            logger.debug(e)
            return False
        if play_info['code'] != 0:
            logger.debug(play_info)
            return False

        playurl_info = play_info['data']['playurl_info']['playurl']
        streams = playurl_info['stream']
        stream = streams[1] if protocol.startswith('hls') and len(streams) > 1 else streams[0]
        stream_format = stream['format'][0]
        if protocol == "hls_fmp4":
            if len(stream['format']) > 1:
                stream_format = stream['format'][1]
            elif int(time.time()) - live_start_time <= 60: # 60s 宽容等待 fmp4
                return False
            elif stream_format['format_name'] == 'ts': # 海外无 fmp4，优先回退 FLV
                stream_format = streams[0]['format'][0]

        if self.downloader == 'stream-gears' and stream_format['format_name'] == 'fmp4':
            logger.error('stream-gears 不支持 fmp4 格式，请修改配置文件内的 downloader')
            return False
        stream_info = stream_format['codec'][0]

        stream_url = {
            'base_url': stream_info['base_url'],
        }
        if perf_cdn is not None:
            perf_cdn_list = perf_cdn.split(',')
            for url_info in stream_info['url_info']:
                if 'host' in stream_url:
                    break
                for cdn in perf_cdn_list:
                    if cdn in url_info['extra']:
                        stream_url['host'] = url_info['host']
                        stream_url['extra'] = url_info['extra']
                        logger.debug(f"找到了perfCDN {stream_url['host']}")
                        break
        if len(stream_url) < 3:
            stream_url['host'] = stream_info['url_info'][-1]['host']
            stream_url['extra'] = stream_info['url_info'][-1]['extra']

        url_path = f"{stream_url['base_url']}{stream_url['extra']}"
        streamName = match1(stream_url['base_url'], streamname_regexp)

        is_cn01 = "cn-gotcha01" in stream_url['extra']
        # 替换 cn-gotcha01 域名
        if is_cn01 and cn01_domains[0] != '':
            import random
            i = len(cn01_domains)
            while i:  # 测试节点是否可用
                host = cn01_domains.pop(random.choice(range(i)))
                i-=1
                try:
                    if check_url_healthy(s, f"https://{host}{url_path}"):
                        stream_url['host'] = "https://" + host
                        logger.debug(f'节点 {host} 可用，替换为该节点')
                        break
                except Exception as e:
                    logger.debug(e)
                    logger.debug(f'节点 {host} 无法访问，尝试下一个节点。')
                    continue
            else:
                logger.error("配置文件中的cn-gotcha01节点均不可用")

        # 移除 streamName 内画质标签。
        if streamName is not None and is_cn01 \
            and force_source and qualityNumber >= 10000:
            logger.debug(streamName) # 替换了 FLV 不会通过健康检查，不用添加判断
            new_base_url = stream_url['base_url'].replace(f"_{streamName.split('_')[-1]}", '')
            if check_url_healthy(s, f"{stream_url['host']}{new_base_url}{stream_url['extra']}"):
                stream_url['base_url'] = new_base_url
                logger.debug(stream_url['base_url'])

        self.raw_stream_url = f"{stream_url['host']}{stream_url['base_url']}{stream_url['extra']}"

        if ov05_ip and "ov-gotcha05" in stream_url['host']:
            self.raw_stream_url = oversea_expand(s, self.raw_stream_url, ov05_ip)

        if bili_cdn_fallback:
            stream_info['url_info'].reverse()
            if not check_url_healthy(s, self.raw_stream_url):
                i = len(stream_info['url_info'])
                while i:
                    i -= 1
                    try:
                        self.raw_stream_url = stream_info['url_info'][i]['host'] + stream_url['base_url'] + stream_info['url_info'][i]['extra']
                        if check_url_healthy(s, self.raw_stream_url):
                            break
                    except Exception as e:
                        logger.debug(e)
                        continue
                else:
                    logger.debug(play_info)
                    self.raw_stream_url = None
                    return False

        s.close()
        return True

    def danmaku_download_start(self, filename):
        if self.bilibili_danmaku:
            self.danmaku = DanmakuClient(self.url, filename + "." + self.suffix)
            self.danmaku.start()

    def close(self):
        if self.danmaku:
            self.danmaku.stop()

def get_play_info(s, api, params):
    api = (lambda a: a if a.startswith(('http://', 'https://')) else 'http://' + a) (api)
    try:
        return s.get(f'{api}/xlive/web-room/v2/index/getRoomPlayInfo', params=params,
                        timeout=5).json()
    except Exception:
        logger.error(f'{api} 连接失败，尝试回退到官方Api')
    return s.get('https://api.live.bilibili.com/xlive/web-room/v2/index/getRoomPlayInfo',
                params=params, timeout=5).json()

# Copy from room-player.js
def check_areablock(data):
    if data is None:
        logger.error('Sorry, bilibili is currently not available in your country according to copyright restrictions.')
        logger.error('非常抱歉，根据版权方要求，您所在的地区无法观看本直播')
        return True
    return False

def check_url_healthy(s, url):
    if s.get(url, stream=True, timeout=5).status_code == 200:
        return True
    return False

def do_login(s):
    try:
        return s.get('https://api.bilibili.com/x/web-interface/nav', timeout=5).json()
    except Exception:
        logger.error(f'无法验证登录态')
        return {}

def oversea_expand(s, url, ov05_ip):
    # 强制替换ov05 302redirect之后的真实地址为指定的域名或ip达到自选ov05节点的目的
    import re
    r = s.get(url, stream=True)
    logger.debug(f'将ov-gotcha05的节点ip替换为了{ov05_ip}')
    return re.sub(r".*(?=/d1--ov-gotcha05)", f"http://{ov05_ip}", r.url, 1)

def load_cookies(filename):
    if filename is not None:
        try:
            import json
            cookies = ""
            with open(filename, encoding='utf-8') as stream:
                s = json.load(stream)
                for i in s["cookie_info"]["cookies"]:
                    cookies += f"{i['name']}={i['value']};"
                return cookies
        except Exception as e:
            logger.debug(e)
            logger.error(f"读取 {filename} 文件失败")
    return None