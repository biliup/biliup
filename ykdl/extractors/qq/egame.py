#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.compact import unescape
import json
import time

class QQEGame(VideoExtractor):
    name = u'QQ EGAME (企鹅电竟)'

    mutli_bitrate = ['0', '900', '550']

    bitrate_2_type = {'0': 'BD', '900': 'HD', '550': 'SD'}

    bitrate_2_profile = {'0': u'超清', '900': u'高清', '550': u'标清'}

    def prepare(self):
        info = VideoInfo(self.name, True)
        if not self.vid:
            self.vid = match1(self.url, '/(\d+)')
        if not self.vid:
            html = get_content(self.url)
            self.vid = match1(html, '"liveAddr":"([0-9\_]+)"')
        self.pid = self.vid

        # from upstream!!
        serverDataTxt = match1(html, 'serverData = {([\S\ ]+)};')
        serverDataTxt = '{%s}' % (serverDataTxt)
        self.logger.debug("serverDataTxt => %s" % (serverDataTxt))

        serverData = json.loads(serverDataTxt)
        self.logger.debug(serverData)

        assert serverData["liveInfo"]["data"]["profileInfo"]["isLive"] == 1, 'error: live show is not on line!!'

        info.title = serverData["liveInfo"]["data"]["videoInfo"]["title"]
        info.artist = serverData["liveInfo"]["data"]["profileInfo"]["nickName"]

        for data in serverData["liveInfo"]["data"]["videoInfo"]["streamInfos"]:
            info.stream_types.append(self.bitrate_2_type[data["bitrate"]])
            info.streams[self.bitrate_2_type[data["bitrate"]]] = {'container': 'flv', 'video_profile': data["desc"], 'src': ["%s&_t=%s000"%(unescape(data["playUrl"]),int(time.time()))], 'size': float('inf')}

        return info

site = QQEGame()
