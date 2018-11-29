import re

import requests

from common import logger
from engine.plugins import FFmpegdl

VALID_URL_BASE = r'(?:https?://)?live\.bilibili\.com/(?P<id>[0-9]+)'
_API_URL = "https://api.live.bilibili.com/room/v1/Room/room_init?id="


# #https://api.live.bilibili.com/room/v1/Room/room_init?id=
class Bilibili(FFmpegdl):
    def __init__(self, fname, url, suffix='flv'):
        super().__init__(fname, url, suffix)

    def check_stream(self):
        m = re.match(VALID_URL_BASE, self.url)
        logger.debug(self.fname)
        if m:
            room_init_api_response = requests.get(_API_URL + '{}'.format(m.group('id')))
            room_init_api_response.close()
            room_init_api_response = room_init_api_response.json()
            # room_init_api_response = json.loads(get_content(_API_URL + '{}'.format(m.group('id'))))
            live_status = room_init_api_response["data"]["live_status"]
            if live_status == 1:
                room_id = room_init_api_response['data']['room_id']

                # room_info_api_response = requests.get(
                #     'https://api.live.bilibili.com/room/v1/Room/get_info?room_id={}'.format(room_id))
                # room_info_api_response.close()
                # room_info_api_response = json.loads(
                #     get_content(
                #         'https://api.live.bilibili.com/room/v1/Room/get_info?room_id={}'.format(room_id)))
                # # title = room_info_api_response['data']['title']
                api_url = 'https://api.live.bilibili.com/room/v1/Room/playUrl?cid={}&quality=0&platform=web' \
                    .format(room_id)
                json_data = requests.get(api_url)
                json_data.close()

                json_data = json_data.json()
                # json_data = json.loads(get_content(api_url))
                self.ydl_opts['absurl'] = json_data['data']['durl'][0]['url']
                # print(self.ydl_opts['absurl'])
                return True
            else:
                return False
            # usr_id = m.group('id')
            # live_url = _API_URL + usr_id
            # res_live_status = requests.get(live_url)
            # res_live_status.close()
            # live_status = res_live_status.json()["data"]["live_status"]
            # if live_status == 1:
            #     return True
            # else:
            #     return False

    # def download():
    #     # common.output_filename = self.fname, str(time.time())[:10]
    #     # p = multiprocessing.Process(target=bilibili.download, args=(self.url, ),
    #     #                             kwargs={'output_dir': '.', 'merge': True})
    #     # p.start()
    #     # p.join()
    #     # bilibili.download(self.url, output_dir='.', merge=True)
    #     room_short_id = '6186479'
    #
    #     print(urls)
    #     # self.streams['live'] = {}
    #     # self.streams['live']['src'] = urls
    #     # self.streams['live']['container'] = 'flv'
    #     # self.streams['live']['size'] = 0


# # bilibili.download('https://live.bilibili.com/10492457', info_only=True)
# # you_get.main(URL='https://live.bilibili.com/10492457')
# print(type(live_status))
# print(live_status)
__plugin__ = Bilibili
# download()
