#!/usr/bin/env python
# -*- coding: utf-8 -*-

from importlib import import_module
from logging import getLogger
from .common import alias, url_to_module

class EmbedExtractor():
    """
    this class is to help video embed site to handle
    video from other site.
    we just need to know the source URL, or source site name, and video ID
    that's enough.
    with site name and VID, develop can easily to find out the real URL.

        embedextractor.video_info['url'] = url

    or

        embedextractor.video_info['site'] = site
        embedextractor.video_info['vid'] = vid

    compatible: also receive the video info which will return directly.

        embedextractor.video_info['info'] = info

    because embed site don't have video info, so they don't need stream_info.
    """

    def __init__(self):
        self.video_info = None
        self.video_info_list = None
        self.logger = getLogger(self.name)

    @staticmethod
    def new_video_info():
        return {'extra': {}}

    def prepare(self):
        """
        this API is to do real job to get source URL, or site and VID
        sometimes title
        MUST override!!
        """
        pass

    def prepare_playlist(self):
        """
        this API is to do real job to get source URL, or site and VID
        sometimes title
        MUST override!!
        """
        pass

    def list_only(self):
        """
        this API is to check if only the list informations is included
        if true, go to parser list mode
        MUST override!!
        """
        pass

    def _parser(self, video_info):
        if 'info' in video_info:
            return video_info['info']

        elif 'site' in video_info:
            site = video_info['site']
            vid = video_info['vid']
            if site in alias.keys():
                site = alias[site]
            s = import_module('.'.join(['ykdl','extractors',site])).site
            info = s.parser(vid)
 
        elif 'url' in video_info:
            url = video_info['url']
            s, u = url_to_module(url)
            info = s.parser(u)

        if 'title' in video_info:
            info.title = video_info['title']
        if 'artist' in video_info:
            info.artist = video_info['artist']
        if 'extra' in video_info and video_info['extra']:
            info.extra.update(video_info['extra'])

        return info

    def parser(self, url):
        self.url = url
        if self.list_only():
            return self.parser_list(url)

        self.video_info = self.new_video_info()
        self.prepare()

        if not self.video_info:
            raise NotImplementedError(self.url + ' is not supported')

        info = self._parser(self.video_info)
        if self.name != info.site:
            info.site = u'{} / {}'.format(self.name, info.site)
        return info

    def parser_list(self, url):
        self.url = url
        self.video_info_list = []
        self.prepare_playlist()

        if not self.video_info_list:
            raise NotImplementedError('Playlist is not supported for ' + self.name + 'with url: ' + self.url)

        for v in self.video_info_list:
            yield self._parser(v)

    def __getattr__(self, attr):
        return None
