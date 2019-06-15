#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content
from ykdl.util.match import match1, matchall


from xml.dom.minidom import parseString
from ykdl.compact import compact_unquote

def location_dec(string):
    head = int(string[0])
    string = string[1:]
    rows = head
    cols = int(len(string)/rows) + 1
    
    out = ""
    full_row = len(string) % head
    for c in range(cols):
        for r in range(rows):
            if c == (cols - 1) and r >= full_row:
                continue
            if r < full_row:
                char = string[r*cols+c]
            else:
                char = string[cols*full_row+(r-full_row)*(cols-1)+c]
            out += char
    return compact_unquote(out).replace("^", "0")

class Xiami(VideoExtractor):
    name = u"Xiami (虾米音乐)"

    song_data = None

    def prepare(self):
        info = VideoInfo(self.name)
        if not self.vid:
            self.vid = match1(self.url, 'http://www.xiami.com/song/(\d+)', 'http://www.xiami.com/song/detail/id/(\d+)')

        if not self.vid or len(self.vid) < 10:
            html = get_content(self.url)
            line = match1(html, '(.*)立即播放</a>')
            self.vid = match1(line, 'play\(\'(\d+)')

        xml = get_content('http://www.xiami.com/song/playlist/id/{}/object_name/default/object_id/0'.format(self.vid) , charset = 'ignore')
        doc = parseString(xml)
        self.song_data = doc.getElementsByTagName("track")[0]
        self.extract_song(info)
        return info

    def extract_song(self, info):
        i = self.song_data
        info.artist = i.getElementsByTagName("artist")[0].firstChild.nodeValue
        info.title = i.getElementsByTagName("songName")[0].firstChild.nodeValue
        url = location_dec(i.getElementsByTagName("location")[0].firstChild.nodeValue)
        info.stream_types.append('current')
        info.streams['current'] = {'container': 'mp3', 'video_profile': 'current', 'src' : [url], 'size': 0}


    def parser_list(self, url):
        if "album" in url:
            _id = match1(url, 'http://www.xiami.com/album/(\d+)')
            t = '1'
        elif "collect" in url:
            _id =match1(url, 'http://www.xiami.com/collect/(\d+)')
            t = '3'

        xml = get_content('http://www.xiami.com/song/playlist/id/{}/type/{}'.format(_id, t), charset = 'ignore')
        doc = parseString(xml)
        tracks = doc.getElementsByTagName("trackList")[0]

        info_list = []
        #ugly code TODO
        n = 0
        for t in tracks.getElementsByTagName('track'):
            if not n % 2:
                info = VideoInfo(self.name)
                self.song_data = t
                self.extract_song(info)
                info_list.append(info)
            n += 1
        return info_list

site = Xiami()
