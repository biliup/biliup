import hashlib
import json
import re
import subprocess
import time
import requests
from Engine import work
from Engine.plugins import Download
from Engine.plugins.twitch import headers

VALID_URL_BASE = r'(?:https?://)?(?:(?:www|m)\.)?douyu\.com'
_API_URL = "http://www.douyutv.com/api/v1/"


class Douyu(Download):
    def check_stream(self):
        check_url = re.sub(r'.*douyu.com', 'http://open.douyucdn.cn/api/RoomApi/room', self.url)
        res = requests.get(check_url)
        res.close()
        s = res.json()
        print(self.fname)
        status = s['data']['room_status']
        if status == '2':
            return False
        else:
            return True

    def dl(self, ydl_opts):
        url = re.sub(r'.*douyu.com', 'https://m.douyu.com/room', self.url)
        res = requests.get(url)
        res.close()
        html = res.text
        room_id_patt = r'"rid"\s*:\s*(\d+),'
        room_id = work.match1(html, room_id_patt)
        if room_id == "0":
            room_id = url[url.rfind('/') + 1:]
        args = "room/%s?aid=wp&client_sys=wp&time=%d" % (room_id, int(time.time()))
        auth_md5 = (args + "zNzMV1y4EMxOHS6I5WKm").encode("utf-8")
        auth_str = hashlib.md5(auth_md5).hexdigest()
        json_request_url = "%s%s&auth=%s" % (_API_URL, args, auth_str)
        # print(json_request_url)
        content = requests.get(json_request_url, headers=headers)
        content.close()
        # content = get_content(json_request_url, headers)
        # print(content.text)
        json_content = json.loads(content.text)
        data = json_content['data']
        server_status = json_content.get('error', 0)
        if server_status is not 0:
            raise ValueError("Server returned error:%s" % server_status)

        title = data.get('room_name')
        print(title)
        show_status = data.get('show_status')
        if show_status is not "1":
            raise ValueError("The live stream is not online! (Errno:%s)" % server_status)

        real_url = data.get('rtmp_url') + '/' + data.get('rtmp_live')
        arg = ['ffmpeg', '-y', '-i', real_url, '-c', 'copy', '-f', 'flv', ydl_opts['outtmpl']+'.part']
        subprocess.call(arg)
        return real_url

    def download(self, ydl_opts, event):
        self.dl(ydl_opts)


__plugin__ = Douyu
