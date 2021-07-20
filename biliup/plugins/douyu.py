import platform
import json

from ykdl.common import url_to_module
from ykdl.util.jsengine import chakra_available, quickjs_available, external_interpreter
from ykdl.util.html import get_content

from ..engine.decorators import Plugin
from ..plugins import logger
from ..engine.download import DownloadBase


@Plugin.download(regexp=r'(?:https?://)?(?:(?:www|m)\.)?douyu\.com')
class Douyu(DownloadBase):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

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
        videoLoop = json.loads(get_content("https://www.douyu.com/betard/"+rid))['room']['videoLoop']
        show_status = json.loads(get_content("https://www.douyu.com/betard/"+rid))['room']['show_status']
        if (show_status != 1 or videoLoop != 0):
            logger.debug("未开播或正在放录播")
            return False
        site, url = url_to_module(self.url)
        try:
            info = site.parser(url)
        except AssertionError:
            return
        stream_id = info.stream_types[0]
        urls = info.streams[stream_id]['src']
        self.raw_stream_url = urls[0]
        # print(info.title)
        return True
