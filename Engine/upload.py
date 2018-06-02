import json
import os
from selenium import webdriver
import selenium
import time
from bin.slider import slider_cracker
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.common.keys import Keys
from Engine import Enginebase, logger


# 10800 18000 4110
# logger = logging.getLogger('log01')

class Upload(Enginebase):
    def __init__(self, items, suffix='*'):
        Enginebase.__init__(self, items, suffix)

    @property
    def file_list(self):
        file_list = []
        for file_name in os.listdir('.'):
            if self.key in file_name:
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
            if file_size <= 0.1:
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

    def upload(self, file_list, link):

        filename = 'bin/bilibili.cookie'
        user_name = 'y446970841@163.com'
        pass_word = '1122000'
        title_ = self.file_name
        videopath = self.assemble_videopath(file_list)

        service_log_path = "{}/chromedriver.log".format('/home')
        options = webdriver.ChromeOptions()

        options.add_argument('headless')
        driver = webdriver.Chrome(executable_path='/usr/bin/chromedriver', chrome_options=options,
                                  service_log_path=service_log_path)
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
            # driver = webdriver.Chrome(executable_path=r'D:\bilibiliupload\chromedriver.exe',service_log_path=service_log_path)
            driver.get("https://www.bilibili.com")
            # driver.delete_all_cookies()

            with open(filename) as f:
                new_cookie = json.load(f)

            for cookie in new_cookie:
                # print(cookie)
                driver.add_cookie(cookie)

            driver.get("https://member.bilibili.com/video/upload.html")

            # print(driver.title)
            WebDriverWait(driver, 20).until(
                EC.presence_of_element_located((By.NAME, 'file')))
            upload = driver.find_element_by_name('file')

            print(driver.title)

            cookie = driver.get_cookies()
            #
            with open(filename, "w") as f:
                json.dump(cookie, f)

            # logger.info(driver.title)

            upload.send_keys(videopath)  # send_keys
            logger.info('开始上传' + title_)

            while True:
                info = driver.find_elements_by_xpath(
                    '//*[@id="item"]/div/div[2]/div[3]/div[1]/div[1]/div/div/div[2]/div[1]/div[3]')
                # print(info)
                for t in info:
                    # print(t)
                    if t.text != '':
                        print(t.text)
                    # else:
                    #     print('出问题啦')
                time.sleep(10)
                text = driver.find_elements_by_xpath(
                    '//*[@id="item"]/div/div[2]/div[3]/div[1]/div[1]/div/div/div[2]/div[1]/div[2]')
                aggregate = set()
                for s in text:
                    print(s.text)
                    aggregate.add(s.text)
                # if text == 'Upload complete' or text == '上传完成':
                #     break

                if len(aggregate) == 1 and ('Upload complete' in aggregate or '上传完成' in aggregate):
                    break
            logger.info('上传%s\n个数%s' % (title_, len(info)))

            # js = "var q=document.getElementsByClassName('content-header-right')[0].scrollIntoView();"
            # driver.execute_script(js)
            if self.is_element_exist(driver, r'//*[@id="app"]/div[3]/div/div/div/div/div'):
                sb = driver.find_element_by_xpath(r'//*[@id="app"]/div[3]/div/div/div/div/div')
                sb.click()
            # 点击模板
            driver.find_element_by_xpath(r'//*[@id="item"]/div/div[2]/div[3]/div[2]/div[1]/div[2]/div[1]').click()
            driver.find_element_by_xpath(r'//*[@id="item"]/div/div[2]/div[3]/div[2]/div[1]/div[3]/div[1]').click()

            # 输入转载来源
            Input = driver.find_element_by_xpath(
                '//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[1]/div[3]/div/div[1]/div/input')
            Input.send_keys(link)

            # 选择分区
            # driver.find_element_by_xpath(r'//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[2]/div[2]/div[1]/div[3]/div').click()
            # driver.find_element_by_xpath(r'//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[2]/div[2]/div[1]/div[3]/div[2]/div[6]').click()

            # 稿件标题
            title = driver.find_element_by_xpath(
                '//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[3]/div[2]/div[1]/div/input')
            title.send_keys(Keys.CONTROL + 'a')
            title.send_keys(Keys.BACKSPACE)
            title.send_keys(title_)

            # js = "var q=document.getElementsByClassName('content-tag-list')[0].scrollIntoView();"
            # driver.execute_script(js)
            time.sleep(3)
            # 输入相关游戏
            # driver.save_screenshot('bin/err.png')
            # print('截图')
            text_1 = driver.find_element_by_xpath(
                '//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[5]/div/div/div[1]/div[2]/div/div/input')
            # text_1 = driver.find_element_by_css_selector(r'#item > div > div.upload-step-two-container > div.step-2-col-right > div.content-container > div.content-body > div.content-normal-container > div.content-desc-container > div > div > div:nth-child(1) > div.content-input-box-container > div > div > input[type="text"]')
            text_1.send_keys('星际争霸2')
            # 简介
            text_2 = driver.find_element_by_xpath(
                '//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[5]/div/div/div[2]/div[2]/div[1]/textarea')
            text_2.send_keys('职业选手直播第一视角录像。\n欢迎加入星际交流群一起玩耍，群号：178459358')

            driver.find_element_by_xpath('//*[@id="item"]/div/div[2]/div[3]/div[2]/div[3]/div[1]').click()
            # screen_shot = driver.save_screenshot('bin/1.png')
            # print('截图')
            time.sleep(5)
            upload_success = driver.find_element_by_xpath(r'//*[@id="item"]/div/div[3]/p[1]').text
            if upload_success == '':
                driver.save_screenshot('bin/err.png')
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
                EC.presence_of_element_located((By.XPATH, r'//*[@id="login-username"]')))

            username = driver.find_element_by_xpath(r'//*[@id="login-username"]')
            username.send_keys(user_name)

            password = driver.find_element_by_xpath('//*[@id="login-passwd"]')
            password.send_keys(pass_word)
            # logger.info('第四步')
            cracker = slider_cracker(driver)

            cracker.crack()

            time.sleep(5)
            if driver.title == '投稿 - 哔哩哔哩弹幕视频网 - ( ゜- ゜)つロ 乾杯~ - bilibili':
                cookie = driver.get_cookies()
                print(cookie)
                with open(filename, "w") as f:
                    json.dump(cookie, f)
                logger.info('更新cookie成功')
            else:
                logger.info('更新cookie失败')
        except:
            logger.exception('未知错误')
        finally:
            driver.quit()
            logger.info('浏览器驱动退出')

    def start(self, event):
        # try:
        url_ = event.dict_['url']
        if self.filter_file():
            logger.info('准备上传' + self.file_name)
            self.upload(self.file_list, link=url_)
        # finally:
            # self.dic[self.key] = value_
            # logger.info('退出上传')