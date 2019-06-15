#!/usr/bin/env python
# -*- coding: utf-8 -*-

from ykdl.compact import compact_bytes

import hashlib
import random

macids = {}

def get_random_str(l):
    string = []
    chars = list('abcdefghijklnmopqrstuvwxyz0123456789')
    size = len(chars)
    for i in range(l):
        string.append(random.choice(chars))
    return ''.join(string)

def get_macid(l=32):
    '''获取macid,此值是通过mac地址经过算法变换而来,对同一设备不变'''
    try:
        macid = macids[l]
    except KeyError:
        macids[l] = macid = get_random_str(l)
    return macid

def md5(s):
    return hashlib.md5(compact_bytes(s, 'utf8')).hexdigest()

def md5x(s):
    #sufix = ''
    #for j in range(8):
    #    for k in range(4):
    #        v4 = 13 * (66 * k + 27 * j) % 35
    #        if ( v4 >= 10 ):
    #            v8 = v4 + 88
    #        else:
    #            v8 = v4 + 49
    #        sufix += chr(v8)
    return md5(s + '1j2k2k3l3l4m4m5n5n6o6o7p7p8q8q9r')

def cmd5x(s):
    # the param src below uses salt h2l6suw16pbtikmotf0j79cej4n8uw13
    #    01010031010000000000
    #    01010031010010000000
    #    01080031010000000000
    #    01080031010010000000
    #    03020031010000000000
    #    03020031010010000000
    #    03030031010000000000
    #    03030031010010000000
    #    02020031010000000000
    #    02020031010010000000
    return md5(s + 'h2l6suw16pbtikmotf0j79cej4n8uw13')
