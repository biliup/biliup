import json
import os
import random
import time

from PIL import Image
from typing import List

from biliup.config import config
from ..engine import Plugin
from ..engine.upload import UploadBase, logger


@Plugin.upload("bilibili")
class BiliChrome(UploadBase):
    def __init__(self, principal, data):
        import selenium.common
        from selenium import webdriver
        from selenium.webdriver.common.action_chains import ActionChains
        from selenium.webdriver.common.by import By
        from selenium.webdriver.common.keys import Keys
        from selenium.webdriver.support import expected_conditions as EC
        from selenium.webdriver.support import expected_conditions as ec
        from selenium.webdriver.support.ui import WebDriverWait
        super().__init__(principal, data, 'engine/bilibili.cookie')
        # self.title = title
        # self.date_title = None
        self.driver = None

    @staticmethod
    def assemble_videopath(file_list):
        root = os.getcwd()
        videopath = ''
        for i in range(len(file_list)):
            file = file_list[i].video
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

    def upload(self, file_list: List[UploadBase.FileInfo]) -> List[UploadBase.FileInfo]:

        filename = self.persistence_path
        videopath = self.assemble_videopath(file_list)

        # service_log_path = "{}/chromedriver.log".format('/home')
        options = webdriver.ChromeOptions()

        options.add_argument('headless')
        self.driver = webdriver.Chrome(executable_path=config.get('chromedriver_path'), chrome_options=options)
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
            return file_list
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
        username.send_keys(config['user']['account']['username'])
        password = self.driver.find_element_by_xpath('//*[@id="login-passwd"]')
        password.send_keys(config['user']['account']['password'])
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

class slider_cracker(object):
    def __init__(self, driver):
        self.driver = driver
        self.driver.maximize_window()  # 最大化窗口
        self.driver.set_window_size(1024, 768)
        self.fn = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'true_image.png')

    def get_true_image(self, slider_xpath=r'//*[@id="gc-box"]/div/div[3]/div[1]'):
        # element = WebDriverWait(self.driver, 50).until(EC.element_to_be_clickable((By.XPATH, slider_xpath)))
        element = WebDriverWait(self.driver, 50).until(
            EC.element_to_be_clickable((By.CLASS_NAME, "geetest_slider_button")))
        ActionChains(self.driver).move_to_element(element).perform()  # 鼠标移动到滑动框以显示图片
        js = 'document.querySelector("body > div.geetest_panel.geetest_wind ' \
             '> div.geetest_panel_box.geetest_no_logo.geetest_panelshowslide ' \
             '> div.geetest_panel_next > div > div.geetest_wrap > div.geetest_widget ' \
             '> div > a > div.geetest_canvas_img.geetest_absolute > canvas").' \
             'style.display = "%s";'
        self.driver.execute_script(js % "inline")
        time.sleep(1)
        true_image = self.get_img(self.fn)
        self.driver.execute_script(js % "none")
        return true_image

    def get_img(self, img_name, img_xpath=r'//*[@id="gc-box"]/div/div[1]/div[2]/div[1]/a[2]'):  # 260*116
        fn = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'slider_screenshot.png')

        screen_shot = self.driver.save_screenshot(fn)
        # image_element = self.driver.find_element_by_xpath(img_xpath)
        image_element = self.driver.find_element_by_class_name(r'geetest_window')
        left = image_element.location['x']
        top = image_element.location['y']  # selenium截图并获取验证图片location后将其截出保存
        right = image_element.location['x'] + image_element.size['width']
        bottom = image_element.location['y'] + image_element.size['height']
        image = Image.open(fn)
        image = image.crop((left, top, right, bottom))
        image.save(img_name)
        return image

    def analysis(self, true_image, knob_xpath=r'//*[@id="gc-box"]/div/div[3]/div[2]'):
        fn = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'img2.png')
        img1 = Image.open(self.fn)
        # slider_element = self.driver.find_element_by_xpath(knob_xpath)
        slider_element = self.driver.find_element_by_class_name("geetest_slider_button")
        ActionChains(self.driver).click_and_hold(slider_element).perform()  # 点击滑块后截取残缺图
        time.sleep(1)
        img2 = self.get_img(fn)
        img1_width, img1_height = img1.size
        img2_width, img2_height = img2.size
        left = 0
        flag = False

        for i in range(69, img1_width):  # 遍历x>65的像素点（x<65是拼图块）
            for j in range(0, img1_height):
                if not self.is_pixel_equal(img1, img2, i, j):
                    left = i
                    flag = True
                    break
            if flag:
                break
        if left >= 73:
            left = left - 3  # 误差纠正
        else:
            left = left
        return left

    def is_pixel_equal(self, img1, img2, x, y):  # 通过比较俩图片像素点RGB值差值判断是否为缺口
        pix1 = img1.load()[x, y]
        pix2 = img2.load()[x, y]
        if (abs(pix1[0] - pix2[0] < 60) and abs(pix1[1] - pix2[1] < 60) and abs(pix1[2] - pix2[2] < 60)):
            return True
        else:
            return False

    def get_track(self, distance):
        """
        根据偏移量获取移动轨迹
        :param distance: 偏移量
        :return: 移动轨迹
        """
        # 移动轨迹
        track = []
        # 当前位移
        current = 0
        # 减速阈值
        mid = distance * 4 / 5
        # 计算间隔
        t = 0.2
        # 初速度
        v = 0

        while current < distance:
            if current < mid:
                # 加速度为正2
                a = 2
            else:
                # 加速度为负3
                a = -3
            # 初速度v0
            v0 = v
            # 当前速度v = v0 + at
            v = v0 + a * t
            # 移动距离x = v0t + 1/2 * a * t^2
            move = v0 * t + 1 / 2 * a * t * t
            # 当前位移
            current += move
            # 加入轨迹
            track.append(round(move))
        # print(track)
        #
        # print(sum(track))
        track.append(distance - sum(track))
        # print(track)
        # print(sum(track))
        return track

    def move_to_gap(self, slider, track):
        """
        拖动滑块到缺口处
        :param slider: 滑块
        :param track: 轨迹
        :return:
        """
        ActionChains(self.driver).click_and_hold(slider).perform()
        for x in track:
            ActionChains(self.driver).move_by_offset(xoffset=x, yoffset=random.uniform(-5, 2)).perform()
        time.sleep(0.5)
        ActionChains(self.driver).release().perform()

    def crack(self):
        true_image = self.get_true_image()
        x_offset = self.analysis(true_image)
        print(x_offset)

        track = self.get_track(x_offset)
        knob_element = WebDriverWait(self.driver, 50).until(
            EC.element_to_be_clickable((By.XPATH, r'/html/body/div[2]/div[2]/div[6]/div/div[1]/div[2]/div[2]')))
        self.move_to_gap(knob_element, track)

        # fn = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'result0.png')
        # fn1 = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'result1.png')
        # time.sleep(0.02)
        # screen_shot = self.driver.save_screenshot(fn)
        # time.sleep(2)
        # screen_shot = self.driver.save_screenshot(fn1)
