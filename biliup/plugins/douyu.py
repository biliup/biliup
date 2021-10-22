import platform
import json
from urllib.parse import urlencode

from ykdl.common import url_to_module
from ykdl.extractors.douyu.util import ub98484234
from ykdl.util.jsengine import chakra_available, quickjs_available, external_interpreter
from ykdl.util.html import get_content

from .. import config
from ..engine.decorators import Plugin
from ..plugins import logger
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m)\.)?douyu\.com')
class Douyu(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)
        self.vid = ''
        self.logger = logger

    def check_stream(self):
        logger.debug(self.fname)
        if platform.system() == 'Linux':
            if not chakra_available and not quickjs_available and external_interpreter is None:
                logger.error('''
        Please install at least one of the following Javascript interpreter.'
        python packages: PyChakra, quickjs
        applications: Gjs, CJS, QuickJS, JavaScriptCore, Node.js, etc.''')
        if len(self.url.split("douyu.com/")) < 2:
            logger.debug("直播间地址错误")
            return False
        rid = self.url.split("douyu.com/")[1]
        videoloop = json.loads(get_content("https://www.douyu.com/betard/"+rid))['room']['videoLoop']
        show_status = json.loads(get_content("https://www.douyu.com/betard/"+rid))['room']['show_status']
        if show_status != 1 or videoloop != 0:
            logger.debug("未开播或正在放录播")
            return False
        douyucdn = config.get('douyucdn') if config.get('douyucdn') else 'tct-h5'
        html_h5enc = get_content('https://www.douyu.com/swf_api/homeH5Enc?rids=' + rid)
        js_enc = json.loads(html_h5enc)['data']['room' + rid]
        params = {
            'cdn': douyucdn,
            'iar': 0,
            'ive': 0
        }
        self.vid = rid
        ub98484234(js_enc, self, params)
        params['rate'] = 0
        data = urlencode(params).encode('utf-8')
        html_content = get_content('https://www.douyu.com/lapi/live/getH5Play/{}'.format(self.vid), data=data)
        live_data = json.loads(html_content)
        live_data = live_data["data"]
        real_url = '{}/{}'.format(live_data['rtmp_url'], live_data['rtmp_live'])
        self.raw_stream_url = real_url
        return True
