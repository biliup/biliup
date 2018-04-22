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
import logging


# 10800 18000 4110


logger = logging.getLogger('log01')

def get_file(filename):
    file_list = []
    for file_name in os.listdir():
        if filename[:-15] in file_name:
            file_list.append(file_name)
    return sorted(file_list)


def remove_filelist(file_list):
    for r in file_list:
        os.remove(r)
        logger.info('删除-' + r)

def filter_file(file_list):
    for r in file_list:
        file_size = os.path.getsize(r) / 1024 / 1024 / 1024
        if file_size <= 0.1:
            os.remove(r)
            logger.info('过滤删除-' + r)


def upload(video_list, link, title_):

    filename = 'bin/bilibili.cookie'
    user_name = ''
    pass_word = ''

    video_path = assemble_videopath(video_list)

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
        # cookies = [
        #     {'domain': '.bilibili.com', 'expiry': 1611825707.027044, 'httpOnly': False, 'name': 'buvid3', 'path': '/',
        #      'secure': False, 'value': 'A558A031-653B-4890-917D-C205F81B84F93220infoc'},
        #     {'domain': '.bilibili.com', 'expiry': 1611825706, 'httpOnly': False, 'name': 'fts', 'path': '/',
        #      'secure': False, 'value': '1517217707'},
        #     {'domain': '.bilibili.com', 'expiry': 1519809706, 'httpOnly': False, 'name': 'finger', 'path': '/',
        #      'secure': False, 'value': 'edc6ecda'},
        #     {'domain': '.bilibili.com', 'expiry': 1519809723.74952, 'httpOnly': False, 'name': 'DedeUserID',
        #      'path': '/', 'secure': False, 'value': '274986200'},
        #     {'domain': '.bilibili.com', 'expiry': 1548753707.683233, 'httpOnly': False, 'name': 'sid', 'path': '/',
        #      'secure': False, 'value': 'llkelw0h'},
        #     {'domain': '.bilibili.com', 'expiry': 1519809723.749743, 'httpOnly': False, 'name': 'bili_jct', 'path': '/',
        #      'secure': False, 'value': 'af58729164b6a192b07cd720ded677a5'},
        #     {'domain': '.bilibili.com', 'expiry': 1519809723.749621, 'httpOnly': False, 'name': 'DedeUserID__ckMd5',
        #      'path': '/', 'secure': False, 'value': '1206e39bc044fe98'},
        #     {'domain': '.bilibili.com', 'expiry': 1519809723.749689, 'httpOnly': True, 'name': 'SESSDATA', 'path': '/',
        #      'secure': False, 'value': '3e02f3d0%2C1519809723%2C3ee58d91'},
        #     {'domain': '.bilibili.com', 'expiry': 2177452800.339961, 'httpOnly': False, 'name': 'LIVE_BUVID',
        #      'path': '/', 'secure': False, 'value': 'AUTO7015172177246979'},
        #     {'domain': '.bilibili.com', 'expiry': 1517221325.776721, 'httpOnly': True, 'name': '_dfcaptcha',
        #      'path': '/', 'secure': False, 'value': 'a32213f1be85682d37cde24aedc2d135'}]

        with open(filename) as f:
            new_cookie = json.load(f)

        for cookie in new_cookie:
            # cookie.pop('domain')
            # print(cookie)
            # cookie['expiry']=1537089921
            driver.add_cookie(cookie)

        # cookie = driver.get_cookies()
        # print(cookie)
        # driver.get("https://www.bilibili.com")

        driver.get("https://member.bilibili.com/video/upload.html")

        # print(driver.title)
        WebDriverWait(driver, 10).until(
            EC.presence_of_element_located((By.NAME, 'file')))
        upload = driver.find_element_by_name('file')

        print(driver.title)

        cookie = driver.get_cookies()
        #
        with open(filename, "w") as f:
            json.dump(cookie, f)

        # logger.info(driver.title)
        logger.info('开始上传' + title_)
        upload.send_keys(video_path)  # send_keys
        # print(upload.get_attribute('value'))  # check value
        while True:
            info = driver.find_elements_by_xpath('//*[@id="sortWrp"]/li/div[2]/div[1]/div[3]')
            # print(info)
            for t in info:
                # print(t)
                if t.text != '':
                    print(t.text)
            time.sleep(10)
            text = driver.find_elements_by_xpath('//*[@id="sortWrp"]/li/div[2]/div[1]/div[2]')
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

        # 点击转载
        click = driver.find_element_by_xpath(
            '//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[1]/div/div[1]/label[2]/div')

        click.click()
        # driver.execute_script('arguments[0].className = "is-checked";',click)
        # print(click.is_selected())
        # print(clicks)
        # for click in clicks:

        # click_1.click()
        # driver.find_elements_by_class_name('is-checked')[1].click()

        # 输入转载来源
        Input = driver.find_element_by_xpath(
            '//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[1]/div/div[2]/div[2]/input')
        Input.send_keys(link)
        # 点击游戏
        driver.find_element_by_xpath(
            '//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[2]/div[1]/ul/li[3]/button').click()
        # 点击电子竞技
        driver.find_element_by_xpath(
            '//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[2]/div[1]/ul/li[3]/ul/li[6]').click()
        # 稿件标题
        title = driver.find_element_by_xpath(
            '//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[3]/div/div[1]/input')
        title.send_keys(Keys.CONTROL + 'a')
        title.send_keys(Keys.BACKSPACE)
        title.send_keys(title_)

        # WebDriverWait(driver, 10).until(
        #     EC.presence_of_element_located((By.XPATH, '//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[4]/div[2]/div[2]/div[2]/div[last()]/div/input')))
        # //*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[4]/div[2]/div[2]/div[2]/div[last()]/div/input
        text = driver.find_element_by_xpath(
            '//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[4]/div[2]/div[3]/div[2]/div[last()]/div/input')
        text.send_keys('星际争霸2')
        text.send_keys(Keys.ENTER)
        time.sleep(0.5)
        text.send_keys('第一视角')
        text.send_keys(Keys.ENTER)
        time.sleep(0.5)
        text.send_keys('职业选手')
        text.send_keys(Keys.ENTER)
        time.sleep(0.5)
        text.send_keys(Keys.CONTROL + 'a')

        text.send_keys('虚空之遗')
        text.send_keys(Keys.ENTER)
        time.sleep(0.5)
        # 输入相关游戏
        text_1 = driver.find_element_by_xpath(
            '//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[5]/div[1]/div[1]/div[1]/div/input')
        text_1.send_keys('星际争霸2')
        # 简介
        text_2 = driver.find_element_by_xpath(
            '//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[5]/div[1]/div[2]/div[1]/div/textarea')
        text_2.send_keys('职业选手直播第一视角录像。\n欢迎加入星际交流群一起玩耍，群号：178459358')

        driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[3]/button').click()

        print('稿件提交完成！')
        logger.info('%s提交完成！' % title_)
        remove_filelist(video_list)
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
    finally:
        driver.quit()
        logger.info('浏览器驱动退出')


def assemble_videopath(file_list):
    videopath = ''
    root = os.getcwd()
    for i in range(len(file_list)):
        file = file_list[i]
        videopath += root + '/' + file + '\n'
    videopath = videopath.rstrip()
    return videopath

def uploads(file_name_, url_):

    logger.info('准备上传' + file_name_[:-4])
    if os.path.isfile(file_name_):
        os.rename(file_name_, file_name_[:-4] + str(time.time())[:10] + file_name_[-4:])
    file_list = get_file(file_name_)
    filter_file(file_list)
    file_list = get_file(file_name_)
    logger.debug('获取%s文件列表' % file_name_[:-4])
    # videopath = ''
    # root = os.getcwd()
    # for i in range(len(file_list)):
    #     file = file_list[i]
    #     videopath += root + '/' + file + '\n'
    # videopath = videopath.rstrip()
    upload(video_list=file_list, link=url_, title_=file_name_[:-4])

    # for r in file_list:
    #     os.remove(r)
    #     logger.info('删除-' + r)


def supplemental_upload(dict, file_name_, key_, url_, value_):
    try:
        for f in os.listdir():
            if file_name_[:-15] in f:
                try:
                    # os.rename(file_name_ + '.part', file_name_[:-4] + str(time.time())[:10] + file_name_[-4:])
                    os.rename(file_name_[:-4] + '.mp4' + '.part',
                              file_name_[:-4] + str(time.time())[:10] + '.mp4')
                    logger.info('%s存在已更名' % (file_name_[:-4] + '.mp4' + '.part'))
                except FileNotFoundError:
                    # logger.info('%s不存在' % (file_name_ + '.part'))
                    # logger.info('%s不存在' % (file_name_[:-4] + '.mp4' + '.part'))
                    pass
                try:
                    os.rename(file_name_[:-4] + '.flv' + '.part',
                              file_name_[:-4] + str(time.time())[:10] + '.flv')
                    logger.info('%s存在已更名' % (file_name_[:-4] + '.flv' + '.part'))
                except FileNotFoundError:
                    # logger.info('%s不存在' % (file_name_[:-4] + '.flv' + '.part'))
                    pass
                logger.info('补充上传' + key_)
                uploads(file_name_, url_)
                break
    finally:
        dict[key_] = value_
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


