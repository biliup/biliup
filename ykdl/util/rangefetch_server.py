#!/usr/bin/env python
# -*- coding: utf-8 -*-

# Multithreading range fetch via proxy server.
# Use urllib3 to reusing connections.
# Auto-adjust threads number be supported.

from logging import getLogger
from ykdl.compact import (
    Queue, thread, urlsplit,
    BaseHTTPRequestHandler, SocketServer
    )
from ykdl.util.html import fake_headers as _fake_headers

import urllib3
import re
import socket
from time import time, sleep

logger = getLogger('RangeFetch')

fake_headers = _fake_headers.copy()
# Set 'keep-alive'
fake_headers['Connection'] = 'keep-alive'
del fake_headers['Accept-Encoding']

class LocalTCPServer(SocketServer.ThreadingTCPServer):

    request_queue_size = 2
    allow_reuse_address = True

    def server_bind(self):
        sock = self.socket
        sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        sock.setsockopt(socket.SOL_TCP, socket.TCP_NODELAY, True)
        self.RequestHandlerClass.bufsize = sock.getsockopt(socket.SOL_SOCKET, socket.SO_SNDBUF)
        SocketServer.TCPServer.server_bind(self)

    def server_close(self):
        self.shutdown()
        self.socket.close()

class RangeFetchHandler(BaseHTTPRequestHandler):

    protocol_version = 'HTTP/1.1'

    def do_GET(self):
        self.url = get_path(self.path)[1:]
        self.url_parts = url_parts = urlsplit(self.url)

        if not url_parts.netloc:
            self.send_error(400,
                'No host found, range fetch can not be finished, url: %s' %  self.path)
            return

        if ('range=' in url_parts.query or
            'live=1' in url_parts.query or
            'range/' in url_parts.path):
            self.send_error(500,
                'Range request not be accepted, range fetch can not be finished, url: %s' %  self.url)
            return

        request_range = self.headers.get('Range')
        if request_range:
            request_range = getbytes(request_range)
            range_start, range_end = [int(n) if n else 0 for n in request_range.group(1, 2)]
        else:
            range_start = range_end = 0

        RangeFetch(self, range_start, range_end).fetch()

class RangeFetch():

    _expect_begin = 0
    _started_order = -1
    proxy = None
    http = None
    timeout = urllib3.Timeout(connect=1, read=2)
    pool_size = 24

    down_rate_min = 1024 * 160 # B/s
    down_rate_max = 1024 * 360
    check_size = 1024 * 512
    first_size = 1024 * 32
    max_size = 1024 * 32
    bufsize = 1028 * 8
    threads = 8
    delay = 0.3

    def __init__(self, handler, range_start, range_end):
        self.handler = handler
        self.write = handler.wfile.write
        self.url = handler.url
        self.scheme = handler.url_parts.scheme
        self.netloc = handler.url_parts.netloc
        self.headers = dict((k.title(), v) for k, v in handler.headers.items())
        self.headers['Host'] = self.netloc
        self.headers.update(fake_headers)

        self.range_start = range_start
        self.range_end = range_end
        self.delay_cache_size = self.max_size * self.threads * 4
        self.delay_star_size = self.delay_cache_size * 2
        self.max_threads = min(self.threads * 2, self.pool_size)

        if self.http is None:
            if self.proxy:
                self.__class__.http = urllib3.ProxyManager(self.proxy, block=True, timeout=self.timeout, maxsize=self.pool_size)
            else:
                self.__class__.http = urllib3.PoolManager(block=True, timeout=self.timeout, maxsize=self.pool_size)

        self.firstrange = range_start, range_start + self.first_size - 1

        self.data_queue = Queue.PriorityQueue()
        self.range_queue = Queue.LifoQueue()
        self._started_threads = {}

    def join_path(self, url):
        return '%s://%s%s' % (self.scheme, self.netloc, get_path(url))

    def join_redirect(self, url):
        if url.find('://', 4, 8) < 0:
            return self.join_path(url)
        else:
            return url

    def rangefetch(self, range_start, range_end, max_tries=3):
        tries= 0
        headers = self.headers.copy()
        headers['Range'] = 'bytes=%d-%d' % (range_start, range_end)

        while True:
            response = self.http.request('GET', self.url, headers=headers, redirect=False, preload_content=False)

            redirect_location = response.get_redirect_location()
            if redirect_location:
                self.url = self.join_redirect(redirect_location)
                response.read()
                response.release_conn()
                continue

            if response.status == 206:
                return response

            tries += 1
            if tries >= max_tries:
                logger.warning('request %d-%d fail' % (range_start, range_end))
                return response
            sleep(2)

    def adjust_threads(self, new_threads):
        old_threads = self._started_order + 1
        new_threads = min(new_threads, self.max_threads)
        if old_threads == new_threads:
            return

        logger.debug('changes threads number to %d' % new_threads)

        self.threads = new_threads
        self._started_order = new_threads - 1

        if old_threads > new_threads:
            return

        t = 0
        for i in range(old_threads, new_threads):
           t += 1
           spawn_later(self.delay * t, self.__fetchlet, i)

    def fetch(self):
        self.response = self.rangefetch(*self.firstrange)
        response_status = self.response.status
        if response_status != 206:
            self.handler.send_error(response_status)
            return
        response_headers = self.response.headers

        start, end, length = [int(x) for x in getrange(response_headers['Content-Range']).group(1, 2, 3)]
        content_length = end + 1 - start
        _end = length - 1
        if start == 0 and self.range_end in (0, _end) and 'Range' not in self.headers:
            response_status = 200
            response_headers['Content-Length'] = str(length)
            range_end = _end
            del response_headers['Content-Range']
        else:
            range_end = self.range_end or _end
            response_headers['Content-Range'] = 'bytes %s-%s/%s' % (start, range_end, length)
            length = range_end + 1
            response_headers['Content-Length'] = str(length - start)

        response_headers['Connection'] = 'close'
        self.handler.send_response_only(response_status)
        for k, v in response_headers.items():
            self.handler.send_header(k, v)
        self.handler.end_headers()

        a = end + 1
        b = end
        n = (length - a) // self.max_size
        for _ in range(n):
            b += self.max_size
            self.range_queue.put((a, b))
            a = b + 1
        if length > a:
            self.range_queue.put((a, length - 1))
        self.range_queue.queue.reverse()

        self.adjust_threads(self.threads)

        has_peek = hasattr(self.data_queue, 'peek')
        peek_timeout = 30
        self._expect_begin = start

        speedtest = {'prev_begin': 0,
                     'prev_cache': 0,
                     'prev_time': time() + self.delay * self.threads / 2
                     }

        while self._expect_begin < length:
            if self.handler.server.socket._closed:
                break

            # Keeping single thread
            if self._started_order > 0 and self._started_order in self._started_threads:
                pres_begin = self._expect_begin
                pres_cache = self.data_queue.qsize() * self.bufsize
                check_size = (pres_begin - speedtest['prev_begin'] +
                              pres_cache - speedtest['prev_cache'])

                if check_size > self.check_size:
                    pres_time = time()
                    down_rate = check_size / (pres_time - speedtest['prev_time'] + 0.1)

                    if down_rate < self.down_rate_min:
                        threads_adjust = self.down_rate_min // down_rate
                    elif down_rate > self.down_rate_max:
                        threads_adjust = (self.down_rate_max - down_rate) // self.down_rate_max
                    else:
                        threads_adjust = 0

                    if threads_adjust:
                        new_threads = int(max(self.threads + threads_adjust, 1))
                        self.adjust_threads(new_threads)

                    speedtest['prev_begin'] = pres_begin
                    speedtest['prev_cache'] = pres_cache
                    speedtest['prev_time'] = pres_time

            try:
                if has_peek:
                    begin, data = self.data_queue.peek(timeout=peek_timeout)
                    if self._expect_begin == begin:
                        self.data_queue.get()
                    elif self._expect_begin < begin:
                        sleep(0.1)
                        continue
                    else:
                        logger.error('error: begin(%r) < expect_begin(%r), exit.'% (begin, self._expect_begin))
                        break
                else:
                    begin, data = self.data_queue.get(timeout=peek_timeout)
                    if self._expect_begin == begin:
                        pass
                    elif self._expect_begin < begin:
                        self.data_queue.put((begin, data))
                        sleep(0.1)
                        continue
                    else:
                        logger.error('error: begin(%r) < expect_begin(%r), exit.'% (begin, self._expect_begin))
                        break
            except Queue.Empty:
                logger.error('data_queue peek timedout break')
                break

            try:
                self.write(data)
                self._expect_begin += len(data)
            except Exception as e:
                logger.warning('disconnected: %r, %r' % (self.url, e))
                break

        self._started_order = -1

    def __fetchlet(self, thread_order):
        if thread_order in self._started_threads:
            logger.debug('thread - %d already exists' % thread_order)
            return
        else:
            self._started_threads[thread_order] = True
            logger.debug('thread - %d start' % thread_order)

        try:
            while True:
                if thread_order > self._started_order:
                    return

                if self.response:
                    response, self.response = self.response, None
                    start, end = self.firstrange
                else:
                    try:
                        start, end = self.range_queue.get(timeout=1)
                    except Queue.Empty:
                        return
                    while ((start - self._expect_begin) > self.delay_star_size and
                            self.data_queue.qsize() * self.bufsize > self.delay_cache_size):
                        if thread_order > self._started_order:
                            self.range_queue.put((start, end))
                            return
                        sleep(0.1)
         
                    response = self.rangefetch(start, end)
                    if response.status != 206:
                        self.range_queue.put((start, end))
                        continue

                try:
                    data = response.read(self.bufsize)
                    while data:
                        self.data_queue.put((start, data))
                        start += len(data)
                        if thread_order > self._started_order:
                            raise
                        data = response.read(self.bufsize)
                except Exception as e:
                    response.close()
                    response._connection = None
                finally:
                    response.release_conn()
                    logger.debug('receive %d bytes, expect_begin(%d)' % (start, self._expect_begin))

                    if start < end + 1:
                        logger.warning('retry %d-%d' % (start, end))
                        self.range_queue.put((start, end))
        finally:
            del self._started_threads[thread_order]
            logger.debug('thread - %d over' % thread_order)

getbytes = re.compile(r'^bytes=(\d*)-(\d*)').search
getrange = re.compile(r'^bytes (\d+)-(\d+)/(\d+)').search

def spawn_later(seconds, target, *args, **kwargs):
    def wrap(*args, **kwargs):
        sleep(seconds)
        target(*args, **kwargs)
    thread.start_new_thread(wrap, args, kwargs)

def get_path(url):
    if url[0] == '/':
        return url
    if not url.find('://', 4, 8) < 0:
        url = url[url.find('/', 12):]
    if url[0] != '/':
        url = '/' + url
    return url


def start_new_server(bind='', port=8806, first_size=None, max_size=None,
                     threads=None, down_rate=None, proxy=None, headers=None, **kwargs):
    if first_size:
        RangeFetch.first_size = first_size
    if max_size:
        RangeFetch.max_size = max_size
    if threads:
        RangeFetch.threads = threads
    if down_rate:
        RangeFetch.down_rate_min = int(down_rate * 2)
        RangeFetch.down_rate_max = RangeFetch.down_rate_min + min(max(down_rate, 1024 * 100), 1024 * 200)
    if proxy:
        RangeFetch.proxy = proxy
    if headers:
        RangeFetch.headers.update(headers)
    new_server = LocalTCPServer((bind, port), RangeFetchHandler)
    RangeFetch.bufsize = RangeFetchHandler.bufsize
    thread.start_new_thread(new_server.serve_forever, ())
    return new_server
