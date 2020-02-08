#!/usr/bin/env python
# -*- coding: utf-8 -*-

from logging import getLogger

from ykdl.compact import compact_isstr

class VideoExtractor():
    def __init__(self):
        self.logger = getLogger(self.name)
        self.url = None
        self.vid = None

    def parser(self, url):
        self.__init__()
        if compact_isstr(url) and url.startswith('http'):
            self.url = url
            if self.list_only():
                return self.parser_list(url)
        else:
            self.vid= url

        info = self.prepare()
        return info

    def parser_list(self, url):
        self.url = url
        video_list = self.prepare_list()
        if not video_list:
            raise NotImplementedError(u'playlist not support for {} with url: {}'.format(self.name, self.url))
        for video in video_list:
            yield self.parser(video)

    def __getattr__(self, attr):
        return None

    def prepare(self):
        pass

    def prepare_list(self):
        pass

    def list_only(self):
        """
        this API is to check if only the list informations is included
        if true, go to parser list mode
        MUST override!!
        """
        pass
