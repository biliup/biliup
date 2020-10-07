import requests
from engine.plugins import BatchCheckBase, SDownload, YDownload, logger

VALID_URL_BASE = r'(?:https?://)?(?:www\.)?panda\.tv/(?P<id>[0-9]+)'
API_ROOMS = 'https://www.panda.tv/api_rooms_videoinfo?roomids='


class PandaS(SDownload):
    def __init__(self, fname, url):
        super().__init__(fname, url, suffix='flv')

    def download(self):
        retval = super().download()
        if retval == 0:
            if self.check_stream():
                logger.info('实际未下载完成' + self.fname)
                self.rename(self.ydl_opts['outtmpl'])
                logger.info('准备递归下载')
                self.run()
                return 0
        return retval


class PandaY(YDownload):
    def __init__(self, fname, url):
        super().__init__(fname, url, suffix='flv')

    # def download(self):
        #     # signal.signal(signal.SIGTERM, common.DesignPattern.signal_handler)
        #     # info_list = self.get_sinfo()
        #
        #     # if 'SD-m3u8' in info_list:
        #     #     ydl_opts['format'] = 'SD-m3u8'
        #     # elif 'HD-m3u8' in info_list:
        #     #     ydl_opts['format'] = 'HD-m3u8'
        #


class BatchCheck(BatchCheckBase):
    def __init__(self, urls):
        BatchCheckBase.__init__(self, pattern_id=VALID_URL_BASE, urls=urls)

    def check(self):
        live = []
        if not self.usr_list:
            return
        url = API_ROOMS + ','.join(self.usr_list)
        res = requests.get(url, timeout=5)
        res.close()
        for i in res.json()['data']:
            if type(i) == str:
                status = res.json()['data'][i]['stream_status']
                if status == '2':
                    pass
                elif status == '1':
                    live.append(res.json()['data'][i]['id'])
                else:
                    print('err')
            else:
                status = i['stream_status']
                if status == '2':
                    pass
                elif status == '1':
                    live.append(i['id'])
                else:
                    print('err')

        return map(lambda x: self.usr_dict.get(x.lower()), live)


__plugin__ = PandaY
