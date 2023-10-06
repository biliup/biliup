import requests

from biliup.config import config
from ..engine.decorators import Plugin
from ..plugins import match1, logger
from ..engine.download import DownloadBase

from urllib.parse import urlparse, parse_qs

@Plugin.download(regexp=r'(?:https?://)?(?:(?:live|www|v)\.)?(kuaishou)\.com')
@Plugin.download(regexp=r'(?:https?://)?(?:(?:(?:livev)\.(?:m))\.)?chenzhongtech\.com')
class Kuaishou(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self, is_check=False):
        murl = ""
        nurl = f"https://live.kuaishou.com/u/{get_kwaiId(self.url)}"

        '''
        Api
        '''
        # split_args = ["/profile/", "/fw/live/"]
        # for split_key in split_args:
        #     if split_key in self.url:
        #         self.url = f"https://live.kuaishou.com/u/{self.url.split(split_key)[1]}"
        # from urllib.parse import urlparse
        # r = requests.get(self.url, headers=self.fake_headers)
        # parsed_url = urlparse(r.url)
        # kwaiId = parsed_url.path.split('/')[-1]
        # kwai_data = requests.get(f'https://live.kuaishou.com/live_api/profile/public?principalId={kwaiId}',
        #              headers=self.fake_headers).json()
        # live_data = kwai_data['data']['live']
        # # 被风控时会返回未开播
        # if not live_data['living']:
        #     logger.debug(kwaiId + '未开播')
        #     return False
        # self.room_title = live_data['caption']
        # self.raw_stream_url = live_data['playUrls'][0]['adaptationSet']['representation'][-1]['url']


        '''
        Phone_WEB
        '''
        # 可以输入 快手号(kwaiId), 用户唯一辨识ID(userEid), v.kuaishou.com 短链
        # livev.m.chenzhongtech.com 移动端链接也需要刷新验证参数
        headers = self.fake_headers.copy()
        headers['User-Agent'] = 'Mozilla/5.0 (iPhone; CPU iPhone OS 13_2_3 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.3 Mobile/15E148 Safari/604.1'
        with requests.Session() as s:
            try:
                s.headers.update(headers)
                r = s.get(nurl, allow_redirects=False)
                murl = r.headers['Location']
            except Exception:
                formatted_log("error", MURL=murl, NURL=nurl, OURL=self.url)
                raise
            parsed_url = urlparse(murl)
            eid = parsed_url.path.split('/')[-1]
            kpn = parse_qs(parsed_url.query).get('kpn')[0]
            s.headers.update({
                'Origin': 'https://livev.m.chenzhongtech.com',
                'Referer': murl,
                'Content-Type': 'application/json',
            })
            data = {
                'source': 6,
                'eid': eid, # 写作 eid 实际是 kwaiId
                'shareMethod': 'card',
                'clientType': 'WEB_OUTSIDE_SHARE_H5'
            }
            # 我不好说，但距离上面获取移动端页面的请求必须间隔 1s 以上
            import time
            time.sleep(1)
            live_data = s.post(f"https://livev.m.chenzhongtech.com/rest/k/live/byUser?kpn={kpn}",
                                json=data).json()
        if not live_data['result'] == 1 :
            formatted_log("error", BANNED=live_data['error_msg'])
            return False
        liveStream = live_data.get('liveStream')
        if liveStream is None:
            formatted_log("error", BANNED=live_data['error_msg'])
            return False
        # liveStream['type'] != 2 or liveStream['streamType'] != 1 可能是视频轮播，未发现相关主播
        if not liveStream['living']:
            logger.debug(liveStream['user']['user_name'] + "未开播")
            return False
        self.room_title = liveStream['caption']
        if config.get('kwai_protocol', "FLV").lower() == 'hls':
            PlayUrlInfo = liveStream['multiResolutionHlsPlayUrls'][-1]
            PlayUrls = PlayUrlInfo['urls'][0]
            self.raw_stream_url = PlayUrls['url']
        elif config.get('kwai_cdn') is not None:
            PlayUrlInfo = liveStream['multiResolutionPlayUrls'][-1]
            PlayUrls = PlayUrlInfo['urls']
            for playUrlInfo in PlayUrls:
                if config.get('kwai_cdn').lower() in playUrlInfo['cdn'].lower():
                    playUrl = playUrlInfo
                    self.raw_stream_url = playUrl['url']

        if self.raw_stream_url is None:
            import random
            liveAdaptiveManifest = random.choice(liveStream['liveAdaptiveManifest'])
            PlayUrlInfo = liveAdaptiveManifest['adaptationSet']['representation'][-1]
            self.raw_stream_url = PlayUrlInfo['url']
        else:
            streamName = self.raw_stream_url.split('/gifshow/')[1].split('.')[0]
            formatted_log('', QUALITY=PlayUrlInfo['type'], QUALITY_NAME=PlayUrlInfo['name'], LEVEL=PlayUrlInfo['level'],
                        URLTYPE=PlayUrls['urlType'], STREAMNAME=streamName)

        '''
        PC_WEB
        '''
        # with requests.Session() as s:
        #     if "/profile/" in self.url:
        #         self.url = f"https://live.kuaishou.com/u/{self.url.split('/profile/')[1]}"
        #     res = s.get(self.url, timeout=5, headers=self.fake_headers)
        # initial_state = res.text.split('window.__INITIAL_STATE__=')[1].split(';(')[0]
        # liveroom = json.loads(initial_state)['liveroom']
        # if liveroom['errorType']['type'] != 1:
        #     logger.debug(liveroom['errorType']['title'])
        #     return False
        # liveStream = liveroom['liveStream']
        # if not liveroom['isLiving'] or liveStream['type'] not in 'live':
        #     logger.debug("直播间未开播或播放的不是直播")
        #     return False
        # self.raw_stream_url = liveStream['playUrls'][0]['adaptationSet']['representation'][-1]['url']
        # self.room_title = liveStream['caption']
        # author = liveroom['author']
        # if self.use_live_cover is True:
        #     try:
        #         self.live_cover_path = \
        #         super().get_live_cover(author['name'], \
        #                                author['id'], \
        #                                self.room_title, \
        #                                author['timestamp'], \
        #                                liveStream['coverUrl'])
        #     except:
        #         logger.error(f"获取直播封面失败")
        return True

def formatted_log(*args, **kwargs):
    level = args[0] if args else ''
    for key, value in kwargs.items():
        if level == "error":
            logger.error(f"Kuaishou {key}: {value}")
        elif level == "warning":
            logger.warning(f"Kuaishou {key}: {value}")
        elif level == "info":
            logger.info(f"Kuaishou {key}: {value}")
        else:
            logger.debug(f"Kuaishou {key}: {value}")

def get_kwaiId(url):
    split_args = ["/profile/", "/fw/live/", "/u/"]
    for key in split_args:
        if key in url:
            kwaiId = url.split(key)[1]
            return kwaiId