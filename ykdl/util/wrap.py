#!/usr/bin/env python
# -*- coding: utf-8 -*-

from __future__ import print_function
import os
import sys
import subprocess
import shlex
from logging import getLogger

logger = getLogger("wrap")

from ykdl.compact import compact_tempfile
from .html import fake_headers

posix = os.name == 'posix'

# The maximum length of cmd string
if posix:
    # Used in Unix is ARG_MAX in conf
    ARG_MAX = int(os.popen('getconf ARG_MAX').read())
else:
    # Used in Windows CreateProcess is 32K
    ARG_MAX = 32 * 1024

class PlayerHandle(object):
    def __init__(self, cmds, env, cleanup=[]):
        self.handle = None
        self.cmds = cmds
        self.env = env
        if cleanup:
            if callable(cleanup):
                cleanup = [cleanup]
            else:
                try:
                    cleanup = [c for c in cleanup if callable(c)]
                except:
                    cleanup = []
        self.cleanup = cleanup

    def __getattr__(self, name):
        return getattr(self.handle, name)

    def wait(self, *args, **kwargs):
        if not self.handle:
            self.play()

    def play(self):
        try:
            for cmd in self.cmds:
                self.handle = handle = subprocess.Popen(cmd, env=self.env)
                handle.wait()
        finally:
            self.terminate()

    def terminate(self):
        if self.handle:
            self.handle.terminate()
        while self.cleanup:
            self.cleanup.pop()()

    kill = terminate

def launch_player(player, urls, ext, play=True, **args):
    if ' ' in player:
        cmd = shlex.split(player, posix=posix)
        if not posix:
            cmd = [arg[1:-1] if arg[0] == arg[-1] == "'" else arg for arg in cmd]
    else:
        cmd = [player]

    if 'mpv' in cmd[0]:
        if ext == 'm3u8' and any(os.path.isfile(url) for url in urls):
            cmd += ['--demuxer-lavf-o=protocol_whitelist=[file,http,https,tls,rtp,tcp,udp,crypto,httpproxy]']
        if args['ua']:
            cmd += ['--user-agent=' + args['ua']]
        if args['referer']:
            cmd += ['--referrer=' + args['referer']]
        if args['title']:
            cmd += ['--force-media-title=' + encode_for_wrap(args['title'], 'ignore')]
        if args['header']:
            cmd += ['--http-header-fields=' + args['header']]

    urls = [encode_for_wrap(url) for url in urls]
    if args['rangefetch']:
        urls = ['http://127.0.0.1:8806/' + url for url in urls]
        cmds = split_cmd_urls(cmd, urls)
        env = os.environ.copy()
        env.pop('HTTP_PROXY', None)
        env.pop('HTTPS_PROXY', None)
        from ykdl.util.rangefetch_server import start_new_server
        cleanup = start_new_server(**args['rangefetch']).server_close
        phandle = PlayerHandle(cmds, env, cleanup=cleanup)
    else:
        urls = list(urls)
        cmds = split_cmd_urls(cmd, urls)
        if args['proxy'].lower().startswith('http'):
            env = os.environ.copy()
            env['HTTP_PROXY'] = args['proxy']
            env['HTTPS_PROXY'] = args['proxy']
        else:
            env = None
        phandle = PlayerHandle(cmds, env)
    if play:
        phandle.play()
    return phandle

def split_cmd_urls(cmd, urls):
    _cmd = cmd + urls
    cmd_len = len(subprocess.list2cmdline(_cmd))
    if cmd_len > ARG_MAX:
        n = cmd_len // ARG_MAX + 1
        m = len(urls) // n + 1
        cmds = []
        for i in range(n):
            s = i * m
            e = s + m
            cmds.append(cmd + urls[s:e])
    else:
        cmds = [_cmd]
    return cmds

def encode_for_wrap(string, errors='strict'):
    sys_code = sys.getfilesystemencoding()
    if isinstance(string, type(u'')):
        if isinstance(string, str):
            string = string.encode(sys_code, errors).decode(sys_code)
        else:
            string = string.encode(sys_code, errors)
    # ignore str in py2, bytes in py3
    return string

def launch_ffmpeg(basename, ext, lenth):
    if ext in ['ts', 'mpg', 'mpeg']:
        inputfile = []
        for i in range(lenth):
            inputfile.append('%s_%d_.%s' % (basename, i, ext))
        inputfile = 'concat:%s' % '|'.join(inputfile)

        if ext == 'ts':
            ext = 'mp4'
        outputfile = basename + '.' + ext

        cmd = ['ffmpeg', '-y', '-i', inputfile, '-c', 'copy', '-hide_banner']
    else:
        #build input
        inputfile = compact_tempfile(mode='w+t', suffix='.txt', dir='.', encoding='utf-8')
        for i in range(lenth):
            inputfile.write('file \'%s_%d_.%s\'\n' % (basename, i, ext))
        inputfile.flush()

        outputfile = basename + '.' + ext

        cmd = ['ffmpeg', '-safe', '-1', '-y', '-f', 'concat', '-i', inputfile.name, '-c', 'copy', '-hide_banner']
        if ext == 'mp4':
            cmd += ['-bsf:a', 'aac_adtstoasc']

    cmd.append(outputfile)
    print('Merging video %s using ffmpeg:' % basename)
    subprocess.call(cmd)

    if os.name == 'nt':
        try:
            inputfile.close()
            os.remove(inputfile.name)
        except:
            pass

def launch_ffmpeg_download(url, name, live):
    print('Now downloading: %s' % name)
    if live:
        print('stop downloading by press \'q\'')

    cmd = ['ffmpeg', '-headers', ''.join('%s: %s\r\n' % x for x in fake_headers.items()), '-y']

    url = encode_for_wrap(url)
    if os.path.isfile(url):
       cmd += ['-protocol_whitelist', 'file,http,https,tls,rtp,tcp,udp,crypto,httpproxy']

    cmd += ['-i', url, '-c', 'copy', '-bsf:a', 'aac_adtstoasc', '-hide_banner', name]

    subprocess.call(cmd)
