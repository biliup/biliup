import base64
import hashlib
import json
import math
import os
import time
from dataclasses import dataclass, field, InitVar, asdict
from json import JSONDecodeError
from os.path import basename, splitext
from typing import Any, Union
from urllib import parse
from urllib.parse import quote

import requests
import rsa

from requests import utils

import engine
from common.decorators import Plugin

from engine.plugins.upload import UploadBase, logger


@Plugin.upload(platform="bili_web")
class BiliWeb(UploadBase):
    def __init__(self, principal, data):
        super().__init__(principal, data, persistence_path='engine/bili.cookie')
        # cookie = data['cookie']
        # self.__data: Upload.Data = data['config']

    def upload(self, file_list):
        video = Data()
        with BiliBili(video) as bili:
            self.login(bili)
            for file in file_list:
                video_part = bili.upload_file(file)  # 上传视频
                video.videos.append(video_part)  # 添加已经上传的视频
            video.title = self.data["format_title"]
            video.desc = '''这个自动录制上传的小程序开源在Github：http://t.cn/RgapTpf(或者在Github搜索ForgQi)
                交流群：837362626'''
            video.source = self.data["url"]  # 添加转载地址说明
            # 设置视频分区,默认为174 生活，其他分区
            tid = engine.config['streamers'][self.principal].get('tid')
            if tid:
                video.tid = tid
            tags = engine.config['streamers'][self.principal].get('tags', ['星际争霸2', '电子竞技'])
            if tags:
                video.set_tag(tags)
            img_path = engine.config['streamers'][self.principal].get('cover_path')
            if img_path:
                video.cover = bili.cover_up(img_path).replace('http:', '')
            ret = bili.submit()  # 提交视频
        logger.info(f"上传成功: {ret}")
        self.remove_filelist(file_list)

    def login(self, b):
        user = engine.config['user']
        cookies = None
        if os.path.isfile(self.persistence_path):
            print('使用持久化内容上传')
            try:
                with open(self.persistence_path) as f:
                    cookies = json.load(f)
            except JSONDecodeError:
                logger.exception('加载cookie出错')
        if not cookies and user.get('cookies'):
            cookies = user['cookies']
        if cookies:
            try:
                b.login_by_cookies(cookies)
            except:
                logger.exception('login error')
                cookies = b.login_by_password(**user['account'])
        else:
            cookies = b.login_by_password(**user['account'])
        with open(self.persistence_path, "w") as f:
            json.dump(cookies, f)


class BiliBili:
    def __init__(self, video: 'Data'):
        self.app_key = 'bca7e84c2d947ac6'
        self.__session = requests.Session()
        self.video = video

        self.__session.headers.update({
            "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/63.0.3239.108",
            "Referer": "https://www.bilibili.com/", 'Connection': 'keep-alive'
        })
        self.__bili_jct = None
        self._auto_os = None

    def login_by_password(self, username, password):
        print('使用账号上传')
        key_hash, pub_key = self.get_key()
        encrypt_password = base64.b64encode(rsa.encrypt(f'{key_hash}{password}'.encode(), pub_key))
        payload = {
            "actionKey": 'appkey',
            "appkey": self.app_key,
            "build": 6040500,
            "captcha": '',
            "challenge": '',
            "channel": 'bili',
            "device": 'phone',
            "mobi_app": 'android',
            "password": encrypt_password,
            "permission": 'ALL',
            "platform": 'android',
            "seccode": "",
            "subid": 1,
            "ts": int(time.time()),
            "username": username,
            "validate": "",
        }
        response = self.__session.post("https://passport.bilibili.com/api/v3/oauth2/login", timeout=5,
                                       data={**payload, 'sign': self.sign(parse.urlencode(payload))})
        r = response.json()
        if r['code'] != 0 and r.get('data') is None:
            raise RuntimeError(r)
        for cookie in r['data']['cookie_info']['cookies']:
            self.__session.cookies.set(cookie['name'], cookie['value'])
            if 'bili_jct' == cookie['name']:
                self.__bili_jct = cookie['value']
        return self.__session.cookies.get_dict()

    def login_by_cookies(self, cookie):
        print('使用cookies上传')
        requests.utils.add_dict_to_cookiejar(self.__session.cookies, cookie)
        if 'bili_jct' in cookie:
            self.__bili_jct = cookie["bili_jct"]

        data = self.__session.get("https://api.bilibili.com/x/web-interface/nav", timeout=5).json()
        if data["code"] != 0:
            raise Exception(data)

    @staticmethod
    def sign(param):
        salt = '60698ba2f68e01ce44738920a0ffe768'
        return hashlib.md5(f"{param}{salt}".encode()).hexdigest()

    def get_key(self):
        url = "https://passport.bilibili.com/api/oauth2/getKey"
        payload = {
            'appkey': f'{self.app_key}',
            'sign': self.sign(f"appkey={self.app_key}"),
        }
        response = self.__session.post(url, data=payload, timeout=5)
        r = response.json()
        if r and r["code"] == 0:
            return r['data']['hash'], rsa.PublicKey.load_pkcs1_openssl_pem(r['data']['key'].encode())

    def probe(self):
        ret = self.__session.get('https://member.bilibili.com/preupload?r=probe', timeout=5).json()
        data, auto_os = None, None
        min_cost = 0
        if ret['probe'].get('get'):
            method = 'get'
        else:
            method = 'post'
            data = bytes(int(1024 * 0.1 * 1024))
        for line in ret['lines']:
            start = time.perf_counter()
            test = self.__session.request(method, f"https:{line['probe_url']}", data=data, timeout=30)
            cost = time.perf_counter() - start
            print(line['query'], cost)
            if test.status_code != 200:
                return
            if not min_cost or min_cost > cost:
                auto_os = line
                min_cost = cost
        auto_os['cost'] = min_cost
        return auto_os

    def upload_file(self, filepath: str):
        """上传本地视频文件,返回视频信息dict
        b站目前支持4种上传线路upos, kodo, gcs, bos
        kodo: {"os":"kodo","query": "bucket=bvcupcdnkodobm&probe_version=20200810",
        "probe_url":"//up-na0.qbox.me/crossdomain.xml"}
        gcs: {"os":"gcs","query":"bucket=bvcupcdngcsus&probe_version=20200810",
        "probe_url":"//storage.googleapis.com/bvcupcdngcsus/OK"},
        upos:
        {"os":"upos","query":"upcdn=ws&probe_version=20200810","probe_url":"//upos-sz-upcdnws.bilivideo.com/OK"}
        {"os":"upos","query":"upcdn=qn&probe_version=20200810","probe_url":"//upos-sz-upcdnqn.bilivideo.com/OK"}
        {"os":"upos","query":"upcdn=bda2&probe_version=20200810","probe_url":"//upos-sz-upcdnbda2.bilivideo.com/OK"}
        """
        if not self._auto_os:
            self._auto_os = self.probe()
            # self._auto_os = {"os": "kodo", "query": "bucket=bvcupcdnkodobm&probe_version=20200810",
            #                  "probe_url": "//up-na0.qbox.me/crossdomain.xml"}
            logger.info(f"自动路线选择{self._auto_os['os']}: {self._auto_os['query']}. time: {self._auto_os['cost']}")
        profile = 'ugcupos/bup' if 'upos' == self._auto_os['os'] else "ugcupos/bupfetch"
        query = f"r={self._auto_os['os']}&profile={quote(profile, safe='')}" \
                f"&ssl=0&version=2.8.9&build=2080900&{self._auto_os['query']}"
        if self._auto_os['os'] == 'upos':
            return self.upos(filepath, query)
        elif self._auto_os['os'] == 'kodo':
            return self.kodo(filepath, query)
        elif self._auto_os['os'] == "gcs":
            raise NotImplementedError('gcs')
        elif self._auto_os['os'] == "bos":
            raise NotImplementedError('bos')
        else:
            logger.error(f"NoSearch:{self._auto_os['os']}")

    def kodo(self, filepath, query):
        total = os.path.getsize(filepath)
        chunk_size = 4194304
        with open(filepath, 'rb') as f:
            name = f.name
            ret = self.__session.get(
                f'https://member.bilibili.com/preupload?name={quote(name)}&size={total}&{query}', timeout=5).json()
            bili_filename = ret['bili_filename']
            key = ret['key']
            endpoint = f"https:{ret['endpoint']}"
            token = ret['uptoken']
            fetch_url = ret['fetch_url']
            fetch_headers = ret['fetch_headers']
            url = f'{endpoint}/mkblk'
            headers = {
                'Authorization': f"UpToken {token}",
            }
            # 开始上传
            parts = []  # 分块信息
            chunks = math.ceil(total / chunk_size)  # 获取分块数量
            for i in range(chunks):
                chunks_data = f.read(chunk_size)  # 一次读取一个分块大小
                response = self.__session.post(f'{url}/{len(chunks_data)}', timeout=30,
                                               data=chunks_data, headers=headers).json()
                parts.append({"index": i, "ctx": response['ctx']})
                print(f'kodo: {(i + 1) / chunks:.1%}')  # 输出上传进度
        self.__session.post(f"{endpoint}/mkfile/{total}/key/{base64.urlsafe_b64encode(key.encode()).decode()}",
                            data=','.join(map(lambda x: x['ctx'], parts)), headers=headers, timeout=10)
        r = self.__session.post(f"https:{fetch_url}", headers=fetch_headers, timeout=5).json()
        if r["OK"] != 1:
            raise Exception(r)
        return {"title": splitext(name)[0], "filename": bili_filename, "desc": ""}

    def upos(self, filepath, query):
        total = os.path.getsize(filepath)
        with open(filepath, 'rb') as f:
            name = f.name
            ret = self.__session.get(
                f'https://member.bilibili.com/preupload?name={quote(name)}&size={total}&{query}', timeout=5).json()
            chunk_size = ret['chunk_size']
            auth = ret["auth"]
            endpoint = ret["endpoint"]
            biz_id = ret["biz_id"]
            upos_uri = ret["upos_uri"]
            url = f"https:{endpoint}/{upos_uri.replace('upos://', '')}"  # 视频上传路径

            # 向上传地址申请上传，得到上传id等信息
            upload_id = self.__session.post(f'{url}?uploads&output=json', timeout=5,
                                            headers={"X-Upos-Auth": auth}).json()["upload_id"]
            # 开始上传
            parts = []  # 分块信息
            chunks = math.ceil(total / chunk_size)  # 获取分块数量
            for i in range(chunks):
                chunks_data = f.read(chunk_size)  # 一次读取一个分块大小
                self.__session.put(f'{url}?partNumber={i + 1}&uploadId={upload_id}&chunk={i}&chunks={chunks}'
                                   f'&size={len(chunks_data)}&start={i * chunk_size}'
                                   f'&end={i * chunk_size + len(chunks_data)}&total={total}', timeout=30,
                                   data=chunks_data, headers={"X-Upos-Auth": auth})

                parts.append({"partNumber": i + 1, "eTag": "etag"})  # 添加分块信息，partNumber从1开始
                print(f'{(i + 1) / chunks:.1%}')  # 输出上传进度
        r = self.__session.post(
            f'{url}?output=json&name={quote(name)}&profile=ugcupos%2Fbup&uploadId={upload_id}&biz_id={biz_id}',
            json={"parts": parts}, headers={"X-Upos-Auth": auth}, timeout=5).json()
        if r["OK"] != 1:
            raise Exception(r)
        return {"title": splitext(name)[0], "filename": splitext(basename(upos_uri))[0], "desc": ""}

    def submit(self):
        if not self.video.title:
            self.video.title = self.video.videos[0]["title"]
        self.__session.get('https://member.bilibili.com/x/geetest/pre/add')
        ret = self.__session.post(f'https://member.bilibili.com/x/vu/web/add?csrf={self.__bili_jct}', timeout=5,
                                  json=asdict(self.video)).json()
        if ret["code"] == 0:
            return ret
        else:
            raise Exception(ret)

    def cover_up(self, img: str):
        """
        :param img: img path or stream
        :return: img URL
        """
        from PIL import Image
        from io import BytesIO

        with Image.open(img) as im:
            # 宽和高,需要16：10
            xsize, ysize = im.size
            if xsize / ysize > 1.6:
                delta = xsize - ysize * 1.6
                region = im.crop((delta / 2, 0, xsize - delta / 2, ysize))
            else:
                delta = ysize - xsize * 10 / 16
                region = im.crop((0, delta / 2, xsize, ysize - delta / 2))
            buffered = BytesIO()
            region.save(buffered, format=im.format)
        r = self.__session.post(
            url='https://member.bilibili.com/x/vu/web/cover/up',
            data={
                'cover': b'data:image/jpeg;base64,' + (base64.b64encode(buffered.getvalue())),
                'csrf': self.__bili_jct
            }, timeout=30
        )
        buffered.close()
        return r.json()['data']['url']

    def get_tags(self, upvideo, typeid="", desc="", cover="", groupid=1, vfea=""):
        """
        上传视频后获得推荐标签
        :param vfea:
        :param groupid:
        :param typeid:
        :param desc:
        :param cover:
        :param upvideo:
        :return: 返回官方推荐的tag
        """
        url = f'https://member.bilibili.com/x/web/archive/tags?' \
              f'typeid={typeid}&title={quote(upvideo["title"])}&filename=filename&desc={desc}&cover={cover}' \
              f'&groupid={groupid}&vfea={vfea}'
        return self.__session.get(url=url, timeout=5).json()

    def __enter__(self):
        return self

    def __exit__(self, e_t, e_v, t_b):
        self.close()

    def close(self):
        """Closes all adapters and as such the session"""
        self.__session.close()


@dataclass
class Data:
    """
    cover: 封面图片，可由recovers方法得到视频的帧截图
    tid: 分区,174为生活，其他分区
    """
    copyright: int = 2
    source: str = ''
    tid: int = 174
    cover: str = ''
    title: str = ''
    desc_format_id: int = 0
    desc: str = ''
    dynamic: str = ''
    subtitle: dict = field(init=False)
    tag: Union[list, str] = ''
    videos: list = field(default_factory=list)
    dtime: Any = None
    open_subtitle: InitVar[bool] = False

    # interactive: int = 0
    # no_reprint: int 1
    # open_elec: int 1

    def __post_init__(self, open_subtitle):
        self.subtitle = {"open": int(open_subtitle), "lan": ""}
        if self.dtime and self.dtime - int(time.time()) <= 14400:
            self.dtime = None
        if isinstance(self.tag, list):
            self.dynamic = f"#{'##'.join(self.tag)}#"
            self.tag = ','.join(self.tag)

    def delay_time(self, dtime: int):
        """设置延时发布时间，距离提交大于4小时，格式为10位时间戳"""
        if dtime - int(time.time()) > 14400:
            self.dtime = dtime

    def set_tag(self, tag: list):
        """设置标签，tag为数组"""
        self.dynamic = f"#{'##'.join(tag)}#"
        self.tag = ','.join(tag)
