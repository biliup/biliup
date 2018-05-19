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
    def __init__(self, dic, key, suffix='*'):
        Enginebase.__init__(self, dic, key, suffix)

    def get_file(self):
        file_list = []
        for file_name in os.listdir('.'):
            if self.file_name[:-11] in file_name:
                file_list.append(file_name)
        file_list = sorted(file_list)
        return file_list

    @staticmethod
    def remove_filelist(file_list):
        for r in file_list:
            os.remove(r)
            logger.info('删除-' + r)

    @staticmethod
    def filter_file(file_list):
        for r in file_list:
            file_size = os.path.getsize(r) / 1024 / 1024 / 1024
            if file_size <= 0.1:
                os.remove(r)
                logger.info('过滤删除-' + r)

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
        user_name = ''
        pass_word = ''
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
            logger.info('开始上传' + title_)
            upload.send_keys(videopath)  # send_keys
            # sb = driver.find_element_by_xpath(r'//*[@id="app"]/div[3]/div/div/div/div/div')
            # sb.click()
            # with open('bilibili.html', 'w', encoding='utf-8') as f:
            #     f.write(driver.page_source)  # 保存网页到本地
            # print('截图')
            # screen_shot = driver.save_screenshot('bin/1.png')
            # time.sleep(20)
            # print(upload.get_attribute('value'))  # check value

            time.sleep(1)
            if self.is_element_exist(driver, r'//*[@id="app"]/div[3]/div/div/div/div/div'):
                sb = driver.find_element_by_xpath(r'//*[@id="app"]/div[3]/div/div/div/div/div')
                # js = "var q=document.getElementsByClassName('new-feature-guide-container')[0].style.display='none';"
                # driver.execute_script(js)
                sb.click()
            # screen_shot = driver.save_screenshot('bin/0.png')
            # print('截图')
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

            # print(text)
            # pageSource = driver.page_source
            # print(pageSource)
            # hid = driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[1]/div/div[1]/label[1]/div')
            # driver.execute_script('arguments[0].style.display="none";',hid)
            # hid_1 = driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[1]/div/div[1]/label[1]')
            # driver.execute_script('arguments[0].style.display="none";',hid_1)

            js = "var q=document.getElementsByClassName('content-header-right')[0].scrollIntoView();"
            driver.execute_script(js)

            # screen_shot = driver.save_screenshot('bin/0.png')
            # print('截图')
            # 点击模板
            driver.find_element_by_xpath(r'//*[@id="item"]/div/div[2]/div[3]/div[2]/div[1]/div[2]/div[1]').click()
            driver.find_element_by_xpath(r'//*[@id="item"]/div/div[2]/div[3]/div[2]/div[1]/div[3]/div[1]').click()
            # 点击转载
            # click = driver.find_element_by_xpath(
            #     '//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[1]/div[2]/div[2]/div')
            #
            # click.click()

            # 输入转载来源
            Input = driver.find_element_by_xpath(
                '//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[1]/div[3]/div/div[1]/div/input')
            Input.send_keys(link)
            # 点击游戏
            # driver.find_element_by_xpath(
            #     '//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[2]/div[2]/div[1]/div[3]/div').click()
            # 点击电子竞技
            # print('第1步')
            # driver.find_element_by_xpath(
            #     '//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[2]/div[2]/div[1]/div[3]/div[2]/div[6]/span[1]').click()
            # print('第2步')
            # 稿件标题
            title = driver.find_element_by_xpath(
                '//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[3]/div[2]/div[1]/div/input')
            title.send_keys(Keys.CONTROL + 'a')
            title.send_keys(Keys.BACKSPACE)
            title.send_keys(title_)
            # WebDriverWait(driver, 10).until(
            #     EC.presence_of_element_located((By.XPATH, '//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[4]/div[2]/div[2]/div[2]/div[last()]/div/input')))
            # //*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[4]/div[2]/div[2]/div[2]/div[last()]/div/input
            # //*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[4]/div[2]/div/div[2]/div[2]/div[1]/input
            # //*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[4]/div[2]/div/div[2]/div[2]/div[last()]/input
            # //*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[4]/div[2]/div/div[2]/div[2]/div[1]/input

            # text = driver.find_element_by_xpath(
            #     '//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[4]/div[2]/div/div[2]/div[2]/div[1]/input')
            # text.send_keys('星际争霸2')
            # text.send_keys(Keys.ENTER)
            # time.sleep(0.5)
            # text.send_keys('第一视角')
            # text.send_keys(Keys.ENTER)
            # time.sleep(0.5)
            # text.send_keys('职业选手')
            # text.send_keys(Keys.ENTER)
            # time.sleep(0.5)
            #
            # text.send_keys(Keys.CONTROL + 'a')
            #
            # text.send_keys('虚空之遗')
            # text.send_keys(Keys.ENTER)
            # time.sleep(0.5)

            # 输入相关游戏
            text_1 = driver.find_element_by_xpath(
                '//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[5]/div/div/div[1]/div[2]/div/div/input')
            text_1.send_keys('星际争霸2')
            # 简介
            text_2 = driver.find_element_by_xpath(
                '//*[@id="item"]/div/div[2]/div[3]/div[2]/div[2]/div[1]/div[5]/div/div/div[2]/div[2]/div[1]/textarea')
            text_2.send_keys('职业选手直播第一视角录像。\n欢迎加入星际交流群一起玩耍，群号：178459358')

            driver.find_element_by_xpath('//*[@id="item"]/div/div[2]/div[3]/div[2]/div[3]/div[1]').click()
            # screen_shot = driver.save_screenshot('bin/1.png')
            # print('截图')
            time.sleep(1)
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

    def uploads(self, event, file_name):
        url_ = event.dict_['url']
        total_url = event.dict_['file_name']
        logger.info('准备上传' + file_name)
        for tu in total_url:
            if os.path.isfile(tu):
                os.rename(tu, file_name + str(time.time())[:10] + tu[-4:])
        bffile_list = self.get_file()
        self.filter_file(bffile_list)
        file_list = self.get_file()
        if len(file_list) == 0:
            logger.info('视频过滤后无文件可传')
            return
        logger.debug('获取%s文件列表' % file_name)
        self.upload(file_list, link=url_)

    def supplemental_upload(self, event):
        file_name_ = self.file_name
        value_ = self.dic[self.key]
        self.dic.pop(self.key)
        try:
            for f in os.listdir('.'):
                if file_name_[:-11] in f:
                    try:
                        # os.rename(file_name_ + '.part', file_name_[:-4] + str(time.time())[:10] + file_name_[-4:])
                        os.rename(file_name_ + '.mp4' + '.part',
                                  file_name_ + str(time.time())[:10] + '.mp4')
                        logger.info('%s存在已更名' % (file_name_ + '.mp4' + '.part'))
                    except FileNotFoundError:
                        # logger.info('%s不存在' % (file_name_ + '.part'))
                        # logger.info('%s不存在' % (file_name_[:-4] + '.mp4' + '.part'))
                        pass
                    try:
                        os.rename(file_name_ + '.flv' + '.part',
                                  file_name_ + str(time.time())[:10] + '.flv')
                        logger.info('%s存在已更名' % (file_name_ + '.flv' + '.part'))
                    except FileNotFoundError:
                        # logger.info('%s不存在' % (file_name_[:-4] + '.flv' + '.part'))
                        pass
                    logger.debug('补充上传' + self.key)
                    self.uploads(event, file_name_)
                    break
        finally:
            self.dic[self.key] = value_
            # os._exit(0)


if __name__ == '__main__':
    # upload('D:\\p2.flv','5','k')
    # file_list = get_file('星际2soo输本虫族天梯第一视角.mp4')
    # videopath = ''
    # root = os.getcwd()
    # for i in range(len(file_list)):
    #     file = file_list[i]
    #     videopath += root + '/' + file + '\n'
    # # videopath += 'test2018年01月22日.mp4'
    # print(file_list)
    # print(videopath)
    # upload(videopath.rstrip(), '5', 'k')
    # for i in range(1):
    #     print(i)
    pass
