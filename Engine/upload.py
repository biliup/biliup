import json
import os
from selenium import webdriver
import selenium.common
import time
import Engine
import Engine.kernel
import Engine.work
from Engine.slider import slider_cracker
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as ec
from selenium.webdriver.common.keys import Keys
from common import logger


# 10800 18000 4110


class Upload(object):
    def __init__(self, title):
        self.title = title
        # self.date = date
        # self.url = url

    # @property
    # def file_name(self):
    #     now = Engine.work.time_now()
    #     file_name = '%s%s' % (now, self.title)
    #     return file_name

    @property
    def file_list(self):
        file_list = []
        for file_name in os.listdir('.'):
            if self.title in file_name:
                file_list.append(file_name)
        file_list = sorted(file_list)
        return file_list

    @staticmethod
    def remove_filelist(file_list):
        for r in file_list:
            os.remove(r)
            logger.info('删除-' + r)

    def filter_file(self):
        file_list = self.file_list
        if len(file_list) == 0:
            return False
        for r in file_list:
            file_size = os.path.getsize(r) / 1024 / 1024 / 1024
            if file_size <= 0.02:
                os.remove(r)
                logger.info('过滤删除-' + r)
        file_list = self.file_list
        if len(file_list) == 0:
            logger.info('视频过滤后无文件可传')
            return False
        for f in file_list:
            if f.endswith('.part'):
                os.rename(f, os.path.splitext(f)[0])
                logger.info('%s存在已更名' % f)
        return True

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

    def upload(self, title_, file_list, link):

        filename = 'Engine/bilibili.cookie'
        # title_ = self.r_title
        videopath = self.assemble_videopath(file_list)

        # service_log_path = "{}/chromedriver.log".format('/home')
        options = webdriver.ChromeOptions()

        options.add_argument('headless')
        driver = webdriver.Chrome(executable_path=Engine.work.chromedrive_path, chrome_options=options)
        # service_log_path=service_log_path)
        try:
            # service_log_path = "{}/chromedriver.log".format('/home')
            # service_log_path = "{}\\chromedriver.log".format('D:\\bilibiliupload')

            # video_path = '/home/wardiii.mp4'
            # video_path = 'D:\\p2.flv\nD:\\p1.flv\nD:\\享受星际！！！看会比赛.flv'
            # link = 'https://www.panda.tv/1160340'

            # options = webdriver.ChromeOptions()
            #
            # options.add_argument('headless')

            # options.add_argument('no-sandbox')
            # options.binary_location = '/usr/bin/google-chrome-stable'

            # driver = webdriver.Chrome(executable_path='/usr/bin/chromedriver', chrome_options=options,
            #                           service_log_path=service_log_path)
            # driver = webdriver.Chrome(executable_path=r'D:\bilibiliupload\chromedriver.exe',\
            # service_log_path=service_log_path)
            driver.get("https://www.bilibili.com")
            # driver.delete_all_cookies()
            if os.path.isfile(filename):
                with open(filename) as f:
                    new_cookie = json.load(f)

                for cookie in new_cookie:
                    # print(cookie)
                    driver.add_cookie(cookie)

            driver.get("https://member.bilibili.com/video/upload.html")

            # print(driver.title)
            WebDriverWait(driver, 20).until(
                ec.presence_of_element_located((By.NAME, 'buploader')))
            upload = driver.find_element_by_name('buploader')

            print(driver.title)

            # logger.info(driver.title)

            upload.send_keys(videopath)  # send_keys
            logger.info('开始上传' + title_)
            time.sleep(2)
            button = r'//*[@class="new-feature-guide-v2-container"]/div/div/div/div/div[1]'
            if self.is_element_exist(driver, button):
                sb = driver.find_element_by_xpath(button)
                sb.click()
                sb.click()
                sb.click()
                logger.debug('点击')

            while True:
                # info = driver.find_elements_by_xpath(
                #     '//*[@id="app"]/div[3]/div[2]/div[2]/div[2]/div/div/div[2]/div[2]')
                # print(info)
                info = driver.find_elements_by_class_name(r'item-upload-info')
                for t in info:
                    # print(t)
                    if t.text != '':
                        print(t.text)
                    # else:
                    #     print('出问题啦')
                time.sleep(10)
                # text = driver.find_elements_by_xpath(
                #     '//*[@id="app"]/div[3]/div[2]/div[2]/div[2]/div/div/div[2]/div[2]/span')
                text = driver.find_elements_by_xpath(r'//*[@class="item-upload-info"]/span')
                aggregate = set()
                for s in text:
                    if s.text != '':
                        aggregate.add(s.text)
                        print(s.text)
                # if text == 'Upload complete' or text == '上传完成':
                #     break

                if len(aggregate) == 1 and ('Upload complete' in aggregate or '上传完成' in aggregate):
                    break
            logger.info('上传%s个数%s' % (title_, len(info)))

            # js = "var q=document.getElementsByClassName('content-header-right')[0].scrollIntoView();"
            # driver.execute_script(js)

            cookie = driver.get_cookies()
            with open(filename, "w") as f:
                json.dump(cookie, f)

            # 点击模板
            driver.find_element_by_xpath(r'//*[@class="normal-title-wrp"]/div/p').click()
            driver.find_element_by_class_name(r'template-list-small-item').click()
            # driver.find_element_by_xpath(
            #     r'//*[@id="app"]/div[3]/div[2]/div[3]/div[1]/div[1]/div/div[2]/div[1]').click()
            # 输入转载来源
            input_o = driver.find_element_by_xpath(
                 '//*[@class="upload-v2-container"]/div[2]/div[3]/div[1]/div[4]/div[3]/div/div/input')
            input_o.send_keys(link)

            # 选择分区
            # driver.find_element_by_xpath(r'//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[2]/div[2]/div[1]/div[3]/div').click()
            # driver.find_element_by_xpath(r'//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[2]/div[2]/div[1]/div[3]/div[2]/div[6]').click()

            # 稿件标题
            title = driver.find_element_by_xpath(
                '//*[@class="upload-v2-container"]/div[2]/div[3]/div[1]/div[8]/div[2]/div/div/input')
            title.send_keys(Keys.CONTROL + 'a')
            title.send_keys(Keys.BACKSPACE)
            title.send_keys(title_)

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
            text_2 = driver.find_element_by_xpath(
                '//*[@class="upload-v2-container"]/div[2]/div[3]/div[1]/div[12]/div[2]/div/textarea')
            text_2.send_keys('职业选手直播第一视角录像。这个自动录制上传的小程序开源在Github：'
                             'http://t.cn/RgapTpf(或者在Github搜索ForgQi)交流群：837362626'
                             '\n顺便推广一下自己的网站http://web-form.me/')

            driver.find_element_by_xpath('//*[@class="upload-v2-container"]/div[2]/div[3]/div[5]/span[1]').click()
            # screen_shot = driver.save_screenshot('bin/1.png')
            # print('截图')
            time.sleep(3)
            upload_success = driver.find_element_by_xpath(r'//*[@id="app"]/div/div[3]/h3').text
            if upload_success == '':
                driver.save_screenshot('err.png')
                logger.info('稿件提交失败，截图记录')
                return
            else:
                logger.info(upload_success)
            # print('稿件提交完成！')
            # logger.info('%s提交完成！' % title_)
            self.remove_filelist(file_list)
        except selenium.common.exceptions.NoSuchElementException:
            logger.exception('发生错误')
        # except selenium.common.exceptions.TimeoutException:
        #     logger.exception('超时')
        except selenium.common.exceptions.TimeoutException:
            logger.info('准备更新cookie')
            # screen_shot = driver.save_screenshot('bin/1.png')
            WebDriverWait(driver, 10).until(
                ec.presence_of_element_located((By.XPATH, r'//*[@id="login-username"]')))

            username = driver.find_element_by_xpath(r'//*[@id="login-username"]')
            username.send_keys(Engine.work.user_name)

            password = driver.find_element_by_xpath('//*[@id="login-passwd"]')
            password.send_keys(Engine.work.pass_word)
            # logger.info('第四步')
            # try:
            cracker = slider_cracker(driver)
            cracker.crack()
            # except:
            #     logger.exception('出错')
            time.sleep(5)
            if driver.title == '投稿 - 哔哩哔哩弹幕视频网 - ( ゜- ゜)つロ 乾杯~ - bilibili':
                cookie = driver.get_cookies()
                print(cookie)
                with open(filename, "w") as f:
                    json.dump(cookie, f)
                logger.info('更新cookie成功')
            else:
                logger.info('更新cookie失败')
        # except:
        #     logger.exception('未知错误')
        finally:
            driver.quit()
            logger.info('浏览器驱动退出')

    def start(self, url, date=None):
        # try:
        # url_ = event.dict_['url']
        # url_ = list(self.url.values())[0]
        title = self.title
        if date:
            title = str(date) + self.title
        if self.filter_file():
            logger.info('准备上传' + title)
            self.upload(title, self.file_list, link=url)
        # finally:
        # self.dic[self.key] = value_
        # logger.info('退出上传')
