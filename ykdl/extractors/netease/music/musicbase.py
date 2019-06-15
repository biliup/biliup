#!/usr/bin/env python
# -*- coding: utf-8 -*-

import base64
import random
import json
from Crypto.Cipher import AES
import sys

from ykdl.util.html import get_content, add_header
from ykdl.extractor import VideoExtractor
from ykdl.videoinfo import VideoInfo
from ykdl.util.match import match1
from ykdl.compact import urlencode, compact_bytes

#consts here

first_key = "0CoJUm6Qyw8W8jud"

iv = "0102030405060708"

def pksc7_padding(string):
    aes_block_size = 16
    padding_size = aes_block_size - len(string) % 16
    return string.ljust(len(string)+padding_size, chr(padding_size))

def make_json_data(url_id):
    fixed = {}
    fixed['br'] = 128000
    fixed['csrf_token'] = '' #in the cookie
    fixed['ids'] = '[{}]'.format(url_id)
    return json.dumps(fixed, separators=(',', ':'))


def random_string():
    base_str = '1234567890ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz'
    r_str = ''
    for i in range(0, 16):
        r_str += base_str[random.randint(0, len(base_str) - 1)]
    return r_str

def RSA_string(input_str):
    modular = 157794750267131502212476817800345498121872783333389747424011531025366277535262539913701806290766479189477533597854989606803194253978660329941980786072432806427833685472618792592200595694346872951301770580765135349259590167490536138082469680638514416594216629258349130257685001248172188325316586707301643237607
    exp = 65537

#first do LE packing
    to_number = 0
    rev_str = input_str[::-1]
    for i in rev_str:
        to_number = to_number * 256 + ord(i)
#then calc ras with exp and modular
    encSecKey = hex(pow(to_number, exp, modular))[2:]
    return encSecKey.rjust(256, '0')

def AES_128_CBC_b64_wrapper(data, key, iv):
    obj = AES.new(compact_bytes(key, 'utf-8'), AES.MODE_CBC, compact_bytes(iv, 'utf-8'))
    input_data = pksc7_padding(data)
    out = obj.encrypt(compact_bytes(input_data, 'utf-8'))
    return base64.b64encode(out).decode('utf8')

def netease_req(ids='468490608', snd_key=None, encSecKey=None):
    data = make_json_data(ids)
    if snd_key is None:
        print('here')
        snd_key = random_string()
        encSecKey = RSA_string(snd_key)
    first_pass = AES_128_CBC_b64_wrapper(data, first_key, iv)
    second_pass = AES_128_CBC_b64_wrapper(first_pass, snd_key, iv)

    payload = {}
    payload['params'] = second_pass
    payload['encSecKey'] = encSecKey

    return payload

class NeteaseMusicBase(VideoExtractor):

    mp3_api = "http://music.163.com/weapi/song/enhance/player/url?csrf_token="
    def prepare(self):
        info = VideoInfo(self.name)
        add_header("Referer", "http://music.163.com/")
        if not self.vid:
            self.vid =  match1(self.url, 'song/(\d+)', '\?id=(.*)')
        api_url = self.api_url.format(self.vid, self.vid)
        music = self.get_music(json.loads(get_content(api_url)))
        self.logger.debug("music info >" + str(music))
        info.title = music['name']
        info.artist = music['artists'][0]['name']

        real_id = music["id"]

        snd_key = random_string()
        if sys.version_info[0] == 3:
            encSecKey = RSA_string(snd_key)
        else:
            encSecKey = RSA_string(snd_key)[:-1]
        payload = netease_req(real_id, snd_key, encSecKey)

        mp3_info = json.loads(get_content(self.mp3_api, data=compact_bytes(urlencode(payload), 'utf-8')))['data'][0]
        self.logger.debug("mp3 > " + str(mp3_info))
        info.stream_types.append('current')
        info.streams['current'] =  {'container': mp3_info['type'], 'video_profile': 'current', 'src' : [mp3_info['url']], 'size': mp3_info['size']}
        return info
