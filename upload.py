import os
from selenium import webdriver
import time
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.common.keys import Keys
import logging


# 10800 18000 4110
# log_fmt = '%(asctime)s %(filename)s[line:%(lineno)d] %(levelname)s %(message)s'
# formatter = logging.Formatter(log_fmt)
# log_file_handler = TimedRotatingFileHandler(filename="ds_update.log", when="D", interval=1, backupCount=2)
# # log_file_handler.suffix = "%Y-%m-%d.log"
# # log_file_handler.extMatch = re.compile(r"^\d{4}-\d{2}-\d{2}.log$")
# log_file_handler.setFormatter(formatter)
# logging.basicConfig(level=logging.INFO)
# logger = logging.getLogger(__name__)
# logger.addHandler(log_file_handler)
logger = logging.getLogger('log01')

# def excision(Path):
#     file_size = os.path.getsize(Path)/1024/1024/1024
#
#     print(file_size)
#
# excision('innovation.flv')


def get_file(filename):
    file_list = []
    for file_name in os.listdir():
        if filename[:-15] in file_name:
            file_list.append(file_name)
    return sorted(file_list)


def upload(video_path, link, title_):
    try:
        service_log_path = "{}/chromedriver.log".format('/home')
        # service_log_path = "{}\\chromedriver.log".format('D:\\bilibiliupload')


        # video_path = '/home/wardiii.mp4'
        # video_path = 'D:\\p2.flv\nD:\\p1.flv\nD:\\享受星际！！！看会比赛.flv'
        # link = 'https://www.panda.tv/1160340'

        options = webdriver.ChromeOptions()

        options.add_argument('headless')
        # options.add_argument('no-sandbox')
        # options.binary_location = '/usr/bin/google-chrome-stable'


        driver = webdriver.Chrome(executable_path='/usr/bin/chromedriver', chrome_options=options,
                                  service_log_path=service_log_path)
        # driver = webdriver.Chrome(executable_path=r'D:\bilibiliupload\chromedriver.exe',service_log_path=service_log_path)


        driver.get("https://www.bilibili.com")
        # cookie = {
        #     'fts':'1511094141',
        #     'buvid3':'0E299359-8055-4E58-9E82-DDF2F325CCAF26543infoc',
        #     'UM_distinctid':'15fdc423182b22-07a38f91352cb7-7b113d-144000-15fdc423183a13',
        #     'pgv_pvi':'3700880384',
        #     'rpdid':'olklsxmolldosooiiqlpw',
        #     'sid':'8k9huzd0',
        #     'finger':'edc6ecda',
        #     'DedeUserID':'274986200',
        #     'DedeUserID__ckMd5':'1206e39bc044fe98',
        #     'SESSDATA':'3e02f3d0%2C1517749345%2C489ac7f7',
        #     'bili_jct':'4a71e9d389551c4bb7a9ebd05ac7dda0',
        #     'member_v2':'1',
        #     'pgv_si':'s2364394496',
        #     '_dfcaptcha':'8c4345e68b07f547aa3f246b7b058cd1',
        #     'LIVE_BUVID':'a23404d890e536c9a93b72e5e50be145',
        #     'LIVE_BUVID__ckMd5':'d5ea746722041186'
        #     }
        # 964b42c0
        # bahkjm41
        # 1515221245
        # EB49C459-95CA-415C-AA48-B3FE5B46BD9D31043infoc
        # 3e02f3d0%2C1517813257%2C06f75553
        # 1ef343ac61192cd3aa59ca9aea289ff3
        # c25a35691b8ef1fc2f2566a1c4df50b4
        # AUTO1115152212636167


        # 814ce92affe021564ee8f5ceca806c4a
        # cookies = [{'name': 'finger', 'value': 'edc6ecda', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False},
        #            {'name': 'sid', 'value': '8k9huzd0', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False},
        #            {'name': 'fts', 'value': '1511094141', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False},
        #            {'name': 'buvid3', 'value': '0E299359-8055-4E58-9E82-DDF2F325CCAF26543infoc', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False},
        #            {'name': 'DedeUserID', 'value': '274986200', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False},
        #            {'name': 'DedeUserID__ckMd5', 'value': '1206e39bc044fe98', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False},
        #            {'name': 'SESSDATA', 'value': '3e02f3d0%2C1517749345%2C489ac7f7', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': True},
        #            {'name': 'bili_jct', 'value': '4a71e9d389551c4bb7a9ebd05ac7dda0', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False},
        #            {'name': '_dfcaptcha', 'value': '8c4345e68b07f547aa3f246b7b058cd1', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': True},
        #            {'name': 'LIVE_BUVID', 'value': 'a23404d890e536c9a93b72e5e50be145', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False}]
        # driver.delete_all_cookies()
        cookies = [
            {'domain': '.bilibili.com', 'expiry': 1611825707.027044, 'httpOnly': False, 'name': 'buvid3', 'path': '/',
             'secure': False, 'value': 'A558A031-653B-4890-917D-C205F81B84F93220infoc'},
            {'domain': '.bilibili.com', 'expiry': 1611825706, 'httpOnly': False, 'name': 'fts', 'path': '/',
             'secure': False, 'value': '1517217707'},
            {'domain': '.bilibili.com', 'expiry': 1519809706, 'httpOnly': False, 'name': 'finger', 'path': '/',
             'secure': False, 'value': 'edc6ecda'},
            {'domain': '.bilibili.com', 'expiry': 1519809723.74952, 'httpOnly': False, 'name': 'DedeUserID',
             'path': '/', 'secure': False, 'value': '274986200'},
            {'domain': '.bilibili.com', 'expiry': 1548753707.683233, 'httpOnly': False, 'name': 'sid', 'path': '/',
             'secure': False, 'value': 'llkelw0h'},
            {'domain': '.bilibili.com', 'expiry': 1519809723.749743, 'httpOnly': False, 'name': 'bili_jct', 'path': '/',
             'secure': False, 'value': 'af58729164b6a192b07cd720ded677a5'},
            {'domain': '.bilibili.com', 'expiry': 1519809723.749621, 'httpOnly': False, 'name': 'DedeUserID__ckMd5',
             'path': '/', 'secure': False, 'value': '1206e39bc044fe98'},
            {'domain': '.bilibili.com', 'expiry': 1519809723.749689, 'httpOnly': True, 'name': 'SESSDATA', 'path': '/',
             'secure': False, 'value': '3e02f3d0%2C1519809723%2C3ee58d91'},
            {'domain': '.bilibili.com', 'expiry': 2177452800.339961, 'httpOnly': False, 'name': 'LIVE_BUVID',
             'path': '/', 'secure': False, 'value': 'AUTO7015172177246979'},
            {'domain': '.bilibili.com', 'expiry': 1517221325.776721, 'httpOnly': True, 'name': '_dfcaptcha',
             'path': '/', 'secure': False, 'value': 'a32213f1be85682d37cde24aedc2d135'}]

        for cookie in cookies:
            # cookie.pop('domain')
            # print(cookie)
            driver.add_cookie(cookie)
        # driver.add_cookie({'name': 'finger', 'value': 'edc6ecda'})
        # driver.add_cookie({'name': 'fts', 'value': '1511094141'})
        # driver.add_cookie({'name': 'buvid3', 'value': '0E299359-8055-4E58-9E82-DDF2F325CCAF26543infoc'})
        # # driver.add_cookie({'name': 'UM_distinctid', 'value': '15fdc423182b22-07a38f91352cb7-7b113d-144000-15fdc423183a13'})
        # # driver.add_cookie({'name': 'pgv_pvi', 'value': '3700880384'})
        # # driver.add_cookie({'name': 'rpdid', 'value': 'olklsxmolldosooiiqlpw'})
        # driver.add_cookie({'name': 'sid', 'value': '8k9huzd0'})
        # driver.add_cookie({'name': 'DedeUserID', 'value': '274986200'})
        # driver.add_cookie({'name': 'DedeUserID__ckMd5', 'value': '1206e39bc044fe98'})
        # driver.add_cookie({'name': 'SESSDATA', 'value': '3e02f3d0%2C1517749345%2C489ac7f7'})
        # driver.add_cookie({'name': 'bili_jct', 'value': '814ce92affe021564ee8f5ceca806c4a'})
        # # driver.add_cookie({'name': 'member_v2', 'value': '1'})
        # # driver.add_cookie({'name': 'pgv_si', 'value': 's2364394496'})
        # driver.add_cookie({'name': '_dfcaptcha', 'value': '8c4345e68b07f547aa3f246b7b058cd1'})
        # driver.add_cookie({'name': 'LIVE_BUVID', 'value': 'a23404d890e536c9a93b72e5e50be145'})
        # # driver.add_cookie({'name': 'LIVE_BUVID__ckMd5', 'value': 'd5ea746722041186'})
        # cookie = driver.get_cookies()
        # print(cookie)
        # driver.get("https://www.bilibili.com")

        driver.get("https://member.bilibili.com/video/upload.html")
        # driver.add_cookie(cookie)
        # time.sleep(20)
        # cookie = driver.get_cookies()
        # print(cookie)
        # print(driver.title)
        element = WebDriverWait(driver, 10).until(
            EC.presence_of_element_located((By.NAME, 'file')))
        upload = driver.find_element_by_name('file')
        # time.sleep(5)
        print(driver.title)
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
        #
        # time.sleep(1)
        # 点击转载
        click = driver.find_element_by_xpath(
            '//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[1]/div/div[1]/label[2]/div')
        # click = driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[1]/div[1]/div[2]/div[1]')
        # click = driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[1]/div/div[1]/label[2]/input')
        # click = driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[1]/div/div[1]/label[1]/input')
        # click.send_keys(Keys.ENTER)
        click.click()
        # driver.execute_script('arguments[0].className = "is-checked";',click)
        # print(click.is_selected())
        # print(clicks)
        # for click in clicks:

        # click_1.click()
        # driver.find_elements_by_class_name('is-checked')[1].click()
        # click_1 = driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[1]/div/div[3]/div[1]/div[2]/label/input')
        # click_1 = driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[1]/div/div[3]/div[1]/div[2]/label/div')
        # click_1.click()
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

        text = driver.find_element_by_xpath(
            '//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[4]/div[2]/div[2]/div[2]/div/div/input')
        text.send_keys('星际争霸2')
        text.send_keys(Keys.ENTER)
        time.sleep(0.5)
        text.send_keys('第一视角')
        text.send_keys(Keys.ENTER)
        time.sleep(0.5)
        text.send_keys('职业选手')
        text.send_keys(Keys.ENTER)
        time.sleep(0.5)
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
    except Exception:
        logger.exception('发生错误')
    finally:
        driver.quit()


def uploads(file_name_, url_):

    logger.info('准备上传' + file_name_[:-4])
    if os.path.isfile(file_name_):
        os.rename(file_name_, file_name_[:-4] + str(time.time())[:10] + file_name_[-4:])
    file_list = get_file(file_name_)
    logger.debug('获取%s文件列表' % file_name_[:-4])
    videopath = ''
    root = os.getcwd()
    for i in range(len(file_list)):
        file = file_list[i]
        videopath += root + '/' + file + '\n'
    videopath = videopath.rstrip()
    upload(video_path=videopath, link=url_, title_=file_name_[:-4])

    for r in file_list:
        os.remove(r)
        logger.info('删除-' + r)


def supplemental_upload(dict, file_name_, key_, url_, value_):
    try:
        for f in os.listdir():
            if file_name_[:-15] in f:
                try:
                    # os.rename(file_name_ + '.part', file_name_[:-4] + str(time.time())[:10] + file_name_[-4:])
                    os.rename(file_name_[:-4] + '.mp4' + '.part',
                              file_name_[:-4] + str(time.time())[:10] + '.mp4')
                except FileNotFoundError:
                    # logger.info('%s不存在' % (file_name_ + '.part'))
                    logger.info('%s不存在' % (file_name_[:-4] + '.mp4' + '.part'))
                try:
                    os.rename(file_name_[:-4] + '.flv' + '.part',
                              file_name_[:-4] + str(time.time())[:10] + '.flv')
                except FileNotFoundError:
                    logger.info('%s不存在' % (file_name_[:-4] + '.flv' + '.part'))
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
    os.rename('3', '4')
