import json
import os
import time
from dataclasses import dataclass, field, InitVar, asdict
from os.path import basename, splitext
from typing import Any, Union
from urllib.parse import quote

import requests
import selenium.common
from requests import utils
from selenium import webdriver
from selenium.webdriver.common.by import By
from selenium.webdriver.common.keys import Keys
from selenium.webdriver.support import expected_conditions as ec
from selenium.webdriver.support.ui import WebDriverWait

import engine
from common.decorators import Plugin
from engine.plugins import logger
from engine.plugins.base_adapter import UploadBase
from engine.slider import slider_cracker


@Plugin.upload("bilibili")
class Bilibili(UploadBase):
    def __init__(self, principal, data):
        super().__init__(principal, data, 'engine/bilibili.cookie')
        # self.title = title
        # self.date_title = None
        self.driver = None

    @staticmethod
    def assemble_videopath(file_list):
        root = os.getcwd()
        videopath = ''
        for i in range(len(file_list)):
            file = file_list[i]
            videopath += root + '/' + file + '\n'
        videopath = videopath.rstrip()
        return videopath

    @staticmethod
    def is_element_exist(driver, xpath):
        s = driver.find_elements_by_xpath(xpath=xpath)
        if len(s) == 0:
            print("元素未找到:%s" % xpath)
            return False
        elif len(s) == 1:
            return True
        else:
            print("找到%s个元素：%s" % (len(s), xpath))
            return False

    def upload(self, file_list):

        filename = self.persistence_path
        videopath = self.assemble_videopath(file_list)

        # service_log_path = "{}/chromedriver.log".format('/home')
        options = webdriver.ChromeOptions()

        options.add_argument('headless')
        self.driver = webdriver.Chrome(executable_path=engine.chromedrive_path, chrome_options=options)
        # service_log_path=service_log_path)
        try:
            self.driver.get("https://www.bilibili.com")
            # driver.delete_all_cookies()
            if os.path.isfile(filename):
                with open(filename) as f:
                    new_cookie = json.load(f)

                for cookie in new_cookie:
                    if isinstance(cookie.get("expiry"), float):
                        cookie["expiry"] = int(cookie["expiry"])
                    self.driver.add_cookie(cookie)

            self.driver.get("https://member.bilibili.com/video/upload.html")

            # print(driver.title)
            self.add_videos(videopath)

            # js = "var q=document.getElementsByClassName('content-header-right')[0].scrollIntoView();"
            # driver.execute_script(js)

            cookie = self.driver.get_cookies()
            with open(filename, "w") as f:
                json.dump(cookie, f)

            self.add_information()

            self.driver.find_element_by_xpath('//*[@class="upload-v2-container"]/div[2]/div[3]/div[5]/span[1]').click()
            # screen_shot = driver.save_screenshot('bin/1.png')
            # print('截图')
            time.sleep(3)
            upload_success = self.driver.find_element_by_xpath(r'//*[@id="app"]/div/div[3]/h3').text
            if upload_success == '':
                self.driver.save_screenshot('err.png')
                logger.info('稿件提交失败，截图记录')
                return
            else:
                logger.info(upload_success)
            # logger.info('%s提交完成！' % title_)
            self.remove_filelist(file_list)
        except selenium.common.exceptions.NoSuchElementException:
            logger.exception('发生错误')
        # except selenium.common.exceptions.TimeoutException:
        #     logger.exception('超时')
        except selenium.common.exceptions.TimeoutException:
            self.login(filename)

        finally:
            self.driver.quit()
            logger.info('浏览器驱动退出')

    def login(self, filename):
        logger.info('准备更新cookie')
        # screen_shot = driver.save_screenshot('bin/1.png')
        WebDriverWait(self.driver, 10).until(
            ec.presence_of_element_located((By.XPATH, r'//*[@id="login-username"]')))
        username = self.driver.find_element_by_xpath(r'//*[@id="login-username"]')
        username.send_keys(engine.user_name)
        password = self.driver.find_element_by_xpath('//*[@id="login-passwd"]')
        password.send_keys(engine.pass_word)
        self.driver.find_element_by_class_name("btn-login").click()
        # logger.info('第四步')
        # try:
        cracker = slider_cracker(self.driver)
        cracker.crack()
        # except:
        #     logger.exception('出错')
        time.sleep(5)
        if self.driver.title == '投稿 - 哔哩哔哩弹幕视频网 - ( ゜- ゜)つロ 乾杯~ - bilibili':
            cookie = self.driver.get_cookies()
            print(cookie)
            with open(filename, "w") as f:
                json.dump(cookie, f)
            logger.info('更新cookie成功')
        else:
            logger.info('更新cookie失败')

    def add_videos(self, videopath):
        formate_title = self.data["format_title"]
        WebDriverWait(self.driver, 20).until(
            ec.presence_of_element_located((By.NAME, 'buploader')))
        upload = self.driver.find_element_by_name('buploader')
        # logger.info(driver.title)
        upload.send_keys(videopath)  # send_keys
        logger.info('开始上传' + formate_title)
        time.sleep(2)
        button = r'//*[@class="new-feature-guide-v2-container"]/div/div/div/div/div[1]'
        if self.is_element_exist(self.driver, button):
            sb = self.driver.find_element_by_xpath(button)
            sb.click()
            sb.click()
            sb.click()
            logger.debug('点击')
        while True:
            try:
                info = self.driver.find_elements_by_class_name(r'item-upload-info')
                for t in info:
                    if t.text != '':
                        print(t.text)
                time.sleep(10)
                text = self.driver.find_elements_by_xpath(r'//*[@class="item-upload-info"]/span')
                aggregate = set()
                for s in text:
                    if s.text != '':
                        aggregate.add(s.text)
                        print(s.text)

                if len(aggregate) == 1 and ('Upload complete' in aggregate or '上传完成' in aggregate):
                    break
            except selenium.common.exceptions.StaleElementReferenceException:
                logger.exception("selenium.common.exceptions.StaleElementReferenceException")
        logger.info('上传%s个数%s' % (formate_title, len(info)))

    def add_information(self):
        link = self.data.get("url")
        # 点击模板
        self.driver.find_element_by_xpath(r'//*[@class="normal-title-wrp"]/div/p').click()
        self.driver.find_element_by_class_name(r'template-list-small-item').click()
        # driver.find_element_by_xpath(
        #     r'//*[@id="app"]/div[3]/div[2]/div[3]/div[1]/div[1]/div/div[2]/div[1]').click()
        # 输入转载来源
        input_o = self.driver.find_element_by_xpath(
            '//*[@class="upload-v2-container"]/div[2]/div[3]/div[1]/div[4]/div[3]/div/div/input')
        input_o.send_keys(link)
        # 选择分区
        # driver.find_element_by_xpath(r'//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[2]/div[2]/div[1]/div[3]/div').click()
        # driver.find_element_by_xpath(r'//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[2]/div[2]/div[1]/div[3]/div[2]/div[6]').click()
        # 稿件标题
        title = self.driver.find_element_by_xpath(
            '//*[@class="upload-v2-container"]/div[2]/div[3]/div[1]/div[8]/div[2]/div/div/input')
        title.send_keys(Keys.CONTROL + 'a')
        title.send_keys(Keys.BACKSPACE)
        title.send_keys(self.data["format_title"])
        # js = "var q=document.getElementsByClassName('content-tag-list')[0].scrollIntoView();"
        # driver.execute_script(js)
        # time.sleep(3)
        # 输入相关游戏
        # driver.save_screenshot('bin/err.png')
        # print('截图')
        # text_1 = driver.find_element_by_xpath(
        #     '//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[5]/div/div/div[1]/div[2]/div/div/input')
        # text_1.send_keys('星际争霸2')
        # 简介
        text_2 = self.driver.find_element_by_xpath(
            '//*[@class="upload-v2-container"]/div[2]/div[3]/div[1]/div[12]/div[2]/div/textarea')
        text_2.send_keys('职业选手直播第一视角录像。这个自动录制上传的小程序开源在Github：'
                         'http://t.cn/RgapTpf(或者在Github搜索ForgQi)\n'
                         '交流群：837362626')

    def start(self):
        # self.date_title = self.title
        # if date:
        #     self.date_title = str(date) + self.title
        # if self.filter_file():
        #     logger.info('准备上传' + self.date_title)
        #     try:
        #         self.upload(self.file_list, link=url)
        #     except selenium.common.exceptions.WebDriverException:
        #         logger.exception('WebDriverException')
        #     # except :
        #     #     logger.exception('?')
        try:
            super().start()
        except selenium.common.exceptions.WebDriverException:
            logger.exception('WebDriverException')


@Plugin.upload(platform="bili_web")
class Upload(UploadBase):
    def __init__(self, principal, data):
        super().__init__(principal, data)
        # cookie = data['cookie']
        # self.__data: Upload.Data = data['config']
        self.video = Data()
        self.__bili_jct = ''
        self.__session = None

    def upload(self, file_list):
        with requests.Session() as self.__session:
            self.login()
            for file in file_list:
                video_part = self.upload_file(file)  # 上传视频
                self.video.videos.append(video_part)  # 添加已经上传的视频
            # bilivideo.setDesc(f'转载于：{url}') #添加简介
            self.video.title = self.data["format_title"]
            self.video.desc = '''这个自动录制上传的小程序开源在Github：http://t.cn/RgapTpf(或者在Github搜索ForgQi)
                交流群：837362626'''
            self.video.source = self.data["url"]  # 添加转载地址说明
            # 设置视频分区,默认为 生活，其他分区
            tid = engine.config['streamers'][self.principal].get('tid')
            if tid:
                self.video.tid = tid
            tags = engine.config['streamers'][self.principal].get('tags', ['星际争霸2', '电子竞技'])
            if tags:
                self.video.set_tag(tags)
            img_path = engine.config['streamers'][self.principal].get('cover_path')
            if img_path:
                self.video.cover = self.cover_up(img_path).replace('http:', '')
            ret = self.submit()  # 提交视频
            logger.info(f"upload_success:{ret}")
            self.remove_filelist(file_list)

    def login(self):
        cookie = engine.config['user']['cookies']
        # self.__session = requests.session()
        self.__session.headers.update({
            "User-Agent": "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/63.0.3239.108",
            "Referer": "https://www.bilibili.com/", 'Connection': 'keep-alive'
        })
        requests.utils.add_dict_to_cookiejar(self.__session.cookies, cookie)
        if 'bili_jct' in cookie:
            self.__bili_jct = cookie["bili_jct"]

        data = self.__session.get("https://api.bilibili.com/x/web-interface/nav").json()
        if data["code"] != 0:
            raise Exception(data["message"])
        # self.__name = data["data"]["uname"]
        # self.__uid = data["data"]["mid"]

    def upload_file(self, filepath: str):
        """上传本地视频文件,返回视频信息dict, fsize=8388608"""
        import math
        path, name = os.path.split(filepath)  # 分离路径与文件名

        with open(filepath, 'rb') as f:
            # size = f.seek(0, 2)  # 获取文件大小
            total = os.path.getsize(filepath)
            # 申请上传返回上传信息
            ret = self.__session.get(f'https://member.bilibili.com/preupload?name={quote(name)}&size={total}'
                                     f'&r=upos&profile=ugcupos%2Fbup&ssl=0&version=2.8.9'
                                     f'&build=2080900&upcdn=bda2&probe_version=20200628').json()
            chunk_size = ret['chunk_size']
            auth = ret["auth"]
            endpoint = ret["endpoint"]
            biz_id = ret["biz_id"]
            upos_uri = ret["upos_uri"]
            url = f"https:{endpoint}/{upos_uri.replace('upos://', '')}"  # 视频上传路径

            # 向上传地址申请上传，得到上传id等信息
            upload_id = self.__session.post(f'{url}?uploads&output=json',
                                            headers={"X-Upos-Auth": auth}).json()["upload_id"]
            # 开始上传
            parts = []  # 分块信息
            chunks = math.ceil(total / chunk_size)  # 获取分块数量
            for i in range(chunks):  # 单线程分块上传，官方支持三线程
                chunks_data = f.read(chunk_size)  # 一次读取一个分块大小
                self.__session.put(
                    f'{url}?partNumber={i + 1}&uploadId={upload_id}&chunk={i}&chunks={chunks}&size={len(chunks_data)}'
                    f'&start={i * chunk_size}&end={i * chunk_size + len(chunks_data)}&total={total}',
                    data=chunks_data, headers={"X-Upos-Auth": auth})
                parts.append({"partNumber": i + 1, "eTag": "etag"})  # 添加分块信息，partNumber从1开始
                print(f'{(i+1)/chunks:.1%}')  # 输出上传进度

        prefix = splitext(name)[0]
        r = self.__session.post(
            f'{url}?output=json&name={quote(name)}&profile=ugcupos%2Fbup&uploadId={upload_id}&biz_id={biz_id}',
            json={"parts": parts}, headers={"X-Upos-Auth": auth}).json()
        if r["OK"] != 1:
            raise Exception(r)
        return {"title": prefix, "filename": splitext(basename(upos_uri))[0], "desc": ""}

    def submit(self):
        if not self.video.title:
            self.video.title = self.video.videos[0]["title"]
        self.__session.get('https://member.bilibili.com/x/geetest/pre/add')
        ret = self.__session.post(f'https://member.bilibili.com/x/vu/web/add?csrf={self.__bili_jct}',
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
        import base64
        # from PIL import Image
        # from io import BytesIO
        #
        # with Image.open(img) as im:
        #     # 宽和高,需要16：10
        #     xsize, ysize = im.size
        #     if xsize / ysize > 1.6:
        #         delta = xsize - ysize * 1.6
        #     else:
        #         delta = ysize - xsize * 10 /16
        #     region = im.crop((delta/2, 0, xsize-delta/2, ysize))
        #     buffered = BytesIO()
        #     region.save(buffered, format=im.format)
        with open(img, 'rb') as f:
            # self.__session.headers['Content-Type'] = 'application/x-www-form-urlencoded', buffered.getvalue()
            r = self.__session.post(
                url='https://member.bilibili.com/x/vu/web/cover/up',
                data={
                    'cover': b'data:image/jpeg;base64,' + (base64.b64encode(f.read())),
                    'csrf': self.__bili_jct
                },
            )
        # buffered.close()
        # {"code":0,"data":{"url":"http://i0.hdslb.com/bfs/archive/67db4a6eae398c309244e74f6e85ae8d813bd7c9.jpg"},"message":"","ttl":1}
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
        return self.__session.get(url=url).json()


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
