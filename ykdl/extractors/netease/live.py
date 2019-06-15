#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.util.html import get_content
from ykdl.util.match import match1
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
import json

class NeteaseLive(VideoExtractor):
    name = u"网易直播 (163)"

    def prepare(self):
        info = VideoInfo(self.name, True)
        if not self.vid:
            html = get_content(self.url)
            self.vid = match1(html, "anchorCcId\s*:\s*\'([^\']+)")
            info.title = match1(html, "title:\s*\'([^\']+)")
            info.artist = match1(html, "anchorName\s*:\s*\'([^\']+)")

        data = json.loads(get_content("http://cgi.v.cc.163.com/video_play_url/{}".format(self.vid)))

        info.stream_types.append("current")
        info.streams["current"] = {'container': 'flv', 'video_profile': "current", 'src' : [data["videourl"]], 'size': 0}
        return info

site = NeteaseLive()
