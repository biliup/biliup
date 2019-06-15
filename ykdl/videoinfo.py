#!/usr/bin/env python
# -*- coding: utf-8 -*-

from __future__ import print_function
import json
import sys
import datetime
import random
from ykdl.util.fs import legitimize
from ykdl.util import log
from ykdl.util.wrap import encode_for_wrap

class VideoInfo():
    def __init__(self, site, live = False):
        self.site = site
        self.title = None
        self.artist = None
        self.stream_types = []
        self.streams = {}
        self.live = live
        self.extra = {"ua": "", "referer": "", "header": "", "proxy": "", "rangefetch": ""}

    def print_stream_info(self, stream_id, show_all = False):
        stream = self.streams[stream_id]
        print("    - format:        %s" % log.sprint(stream_id, log.NEGATIVE))
        if 'container' in stream:
            print("      container:     %s" % stream['container'])
        if 'video_profile' in stream:
            print("      video-profile: %s" % stream['video_profile'])
        if 'quality' in stream:
            print("      quality:       %s" % stream['quality'])
        if 'size' in stream and stream['size'] != 0 and stream['size'] != float('inf'):
            print("      size:          %s MiB (%s bytes)" % (round(stream['size'] / 1048576, 1), stream['size']))
        print("    # download-with: %s" % log.sprint("ykdl --format=%s [URL]" % stream_id, log.UNDERLINE))
        if show_all:
            print("Real urls:")
            for url in stream['src']:
                print("%s" % url)

    def jsonlize(self):
        json_dict = { 'site'   : self.site,
                      'title'  : self.title,
                      'artist'    : self.artist,
                    }
        json_dict['streams'] = self.streams
        json_dict['stream_types'] = self.stream_types
        json_dict['extra'] = self.extra
        for s in json_dict['streams']:
            if json_dict['streams'][s].get('size') == float('inf'):
                json_dict['streams'][s].pop('size')
        return json_dict

    def print_info(self, stream_id = None, show_all = False):
        print("site:                %s" % self.site)
        print("title:               %s" % self.title)
        print("artist:              %s" % self.artist)
        print("streams:")
        if not show_all:
            stream_id = stream_id or self.stream_types[0]
            self.print_stream_info(stream_id, show_all)
        else:
            for stream_id in self.stream_types:
                self.print_stream_info(stream_id, show_all)

    def build_file_name(self,stream_id):
        if not self.title:
            self.title = self.site + str(random.randint(1, 9999))
        name_list = [self.title]
        if not stream_id == 'current':
            name_list.append(stream_id)
        if self.live:
            name_list.append(datetime.datetime.now().isoformat())
        return encode_for_wrap(legitimize('_'.join(name_list)), 'ignore')
