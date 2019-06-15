#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.html import get_content
from ykdl.util.match import match1, matchall
from ykdl.compact import compact_bytes

import time
import random
import sys
import hashlib

import binascii
from xml.dom.minidom import parseString

def lshift(a, b):
    return (a << b) & 0xffffffff

def rshift(a, b):
    if a >= 0:
        return a >> b
    return (0x100000000 + a) >> b

def le32_pack(b_str):
    if sys.version_info[0] == 3:
        result = 0
        result |= b_str[0]
        result |= (b_str[1] << 8)
        result |= (b_str[2] << 16)
        result |= (b_str[3] << 24)
        return result
    else:
        result = 0
        result |= ord(b_str[0])
        result |= (ord(b_str[1]) << 8)
        result |= (ord(b_str[2]) << 16)
        result |= (ord(b_str[3]) << 24)
        return result

def tea_core(data, key_seg):
    delta = 2654435769

    d0 = le32_pack(data[:4])
    d1 = le32_pack(data[4:8])

    sum_ = 0
    for rnd in range(32):
        sum_ = (sum_ + delta) & 0xffffffff
        p1 = (lshift(d1, 4) + key_seg[0]) & 0xffffffff
        p2 = (d1 + sum_) & 0xffffffff
        p3 = (rshift(d1, 5) + key_seg[1]) & 0xffffffff

        mid_p = p1 ^ p2 ^ p3
        d0 = (d0 + mid_p) & 0xffffffff

        p4 = (lshift(d0, 4) + key_seg[2]) & 0xffffffff
        p5 = (d0 + sum_) & 0xffffffff
        p6 = (rshift(d0, 5) + key_seg[3]) & 0xffffffff

        mid_p = p4 ^ p5 ^ p6
        d1 = (d1 + mid_p) & 0xffffffff

    return bytes(unpack_le32(d0) + unpack_le32(d1))

def ran_hex(size):
    result = []
    for i in range(size):
        result.append(hex(int(15 * random.random()))[2:])
    return ''.join(result)

def zpad(b_str, size):
    size_diff = size - len(b_str)
    return b_str + bytes(size_diff)

def gen_key(t):
    key_seg = [1896220160,101056625, 100692230, 7407110]
    t_s = hex(int(t))[2:].encode('utf8')
    input_data = zpad(t_s, 16)
    out = tea_core(input_data, key_seg)
    return binascii.hexlify(out[:8]).decode('utf8') + ran_hex(16)

def unpack_le32(i32):
    result = []
    result.append(i32 & 0xff)
    i32 = rshift(i32, 8)
    result.append(i32 & 0xff)
    i32 = rshift(i32, 8)
    result.append(i32 & 0xff)
    i32 = rshift(i32, 8)
    result.append(i32 & 0xff)
    return result

def get_elem(elem, tag):
    return elem.getElementsByTagName(tag)

def get_attr(elem, attr):
    return elem.getAttribute(attr)

def get_text(elem):
    return elem.firstChild.nodeValue

def shift_time(time_str):
    ts = time_str[:-4]
    return time.mktime(time.strptime(ts)) - 60

def parse_pptv_xml(dom):
    channel = get_elem(dom, 'channel')[0]
    title = get_attr(channel, 'nm')
    file_list = get_elem(channel, 'file')[0]
    item_list = get_elem(file_list, 'item')
    streams_cnt = len(item_list)
    item_mlist = []
    for item in item_list:
        rid = get_attr(item, 'rid')
        file_type = get_attr(item, 'ft')
        size = get_attr(item, 'filesize')
        width = get_attr(item, 'width')
        height = get_attr(item, 'height')
        bitrate = get_attr(item, 'bitrate')
        res = '{}x{}@{}kbps'.format(width, height, bitrate)
        item_meta = (file_type, rid, size, res)
        item_mlist.append(item_meta)

    dt_list = get_elem(dom, 'dt')
    # 部分视频没有dragdata标签
    # dragdata_list = get_elem(dom, 'dragdata')
    dragdata_list = get_elem(dom, 'dragdata') or get_elem(dom, 'drag')

    stream_mlist = []
    for dt in dt_list:
        file_type = get_attr(dt, 'ft')
        serv_time = get_text(get_elem(dt, 'st')[0])
        expr_time = get_text(get_elem(dt, 'key')[0])
        serv_addr = get_text(get_elem(dt, 'sh')[0])
        stream_meta = (file_type, serv_addr, expr_time, serv_time)
        stream_mlist.append(stream_meta)

    segs_mlist = []
    for dd in dragdata_list:
        file_type = get_attr(dd, 'ft')
        seg_list = get_elem(dd, 'sgm')
        segs = []
        segs_size = []
        for seg in seg_list:
            rid = get_attr(seg, 'rid')
            size = get_attr(seg, 'fs')
            segs.append(rid)
            segs_size.append(size)
        segs_meta = (file_type, segs, segs_size)
        segs_mlist.append(segs_meta)
    return title, item_mlist, stream_mlist, segs_mlist

#mergs 3 meta_data
def merge_meta(item_mlist, stream_mlist, segs_mlist):
    streams = {}
    for i in range(len(segs_mlist)):
        streams[str(i)] = {}
    for item in item_mlist:
        stream = streams[item[0]]
        stream['rid'] = item[1]
        # 无size情况,如无需求,指定默认值
        # stream['size'] = item[2]
        stream['size'] = item[2] or 12653713
        stream['res'] = item[3]

    for s in stream_mlist:
        stream = streams[s[0]]
        stream['serv_addr'] = s[1]
        stream['expr_time'] = s[2]
        stream['serv_time'] = s[3]

    for seg in segs_mlist:
        stream = streams[seg[0]]
        stream['segs'] = seg[1]
        stream['segs_size'] = seg[2]
    return streams


def make_url(stream):
    host = stream['serv_addr']
    rid = stream['rid']
    key = gen_key(shift_time(stream['serv_time']))
    key_expr = stream['expr_time']

    src = []
    for i, seg in enumerate(stream['segs']):
        url = 'http://{}/{}/{}?key={}&k={}'.format(host, i, rid, key, key_expr)
        # type修改成ppbox.launcher
        # url += '&fpp.ver=1.3.0.23&type=web.fpp'
        url += '&fpp.ver=1.3.0.23&type=ppbox.launcher'
        src.append(url)
    return src

class Pptv(VideoExtractor):
    name = u'PPTV (PP聚力)'

    supported_stream_types = ['LD', 'SD', 'HD', 'TD', 'BD']

    def prepare(self):
        info = VideoInfo(self.name)
        self.vid = match1(self.url, '\.pptv\.com/vod/(\d+)')
        if not self.vid:
            html = get_content(self.url)
            self.vid = match1(html, 'webcfg\s*=\s*{"id":\s*(\d+)')
         # API修改
        # param = "type%3dppbox.launcher%26ahl_ver%3d1%26ahl_random%3d6c2b3072426c42253c754c4460624b76%26ahl_signa%3d8544ec938b8b6e4153320931d5079e7aadfbed5855a5ccc40c66d470338b7056%26userType%3d0%26o%3d0"
        # xml = get_content('http://web-play.pptv.com/webplay3-0-{}.xml?version=4&param={}&type=web.fpp&appplt=flp&appid=pptv.flashplayer.vod&appver=3.4.2.32'.format(self.vid,param))
        xml = get_content('https://web-play.pptv.com/webplay3-0-{}.xml?zone=8&version=4&username=&ppi=302c3333&type=ppbox.launcher&pageUrl=http%3A%2F%2Fv.pptv.com&o=0&referrer=&kk=&scver=1&appplt=flp&appid=pptv.flashplayer.vod&appver=3.4.3.3&nddp=1'.format(self.vid))
        dom = parseString(compact_bytes(xml, 'utf-8'))
        info.title, m_items, m_streams, m_segs = parse_pptv_xml(dom)
        xml_streams = merge_meta(m_items, m_streams, m_segs)
        for stream_id in xml_streams:
            stream_data = xml_streams[stream_id]
            src = make_url(stream_data)
            s = self.supported_stream_types[int(stream_id)]
            info.stream_types.append(s)
            info.streams[s] = { 'container': 'mp4', 'video_profile': stream_data['res'], 'size': int(stream_data['size']), 'src': src}
        info.stream_types = sorted(info.stream_types, key = self.supported_stream_types.index)
        info.stream_types.reverse()
        return info

site=Pptv()
