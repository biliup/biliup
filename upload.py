from selenium import webdriver
import time
from selenium.webdriver.common.by import By
from selenium.webdriver.support.ui import WebDriverWait
from selenium.webdriver.support import expected_conditions as EC
from selenium.webdriver.common.keys import Keys
service_log_path = "{}/chromedriver.log".format('/home')
video_path = '/home/wardiii.mp4'
options = webdriver.ChromeOptions()
options.add_argument('headless')
# options.add_argument('no-sandbox')
# options.binary_location = '/usr/bin/google-chrome-stable'
driver = webdriver.Chrome(executable_path='/usr/bin/chromedriver',chrome_options=options,service_log_path=service_log_path)
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
cookies = [{'name': 'finger', 'value': 'edc6ecda', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False},
           {'name': 'sid', 'value': '8k9huzd0', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False},
           {'name': 'fts', 'value': '1511094141', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False},
           {'name': 'buvid3', 'value': '0E299359-8055-4E58-9E82-DDF2F325CCAF26543infoc', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False},
           {'name': 'DedeUserID', 'value': '274986200', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False},
           {'name': 'DedeUserID__ckMd5', 'value': '1206e39bc044fe98', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False},
           {'name': 'SESSDATA', 'value': '3e02f3d0%2C1517749345%2C489ac7f7', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': True},
           {'name': 'bili_jct', 'value': '4a71e9d389551c4bb7a9ebd05ac7dda0', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False},
           {'name': '_dfcaptcha', 'value': '8c4345e68b07f547aa3f246b7b058cd1', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': True},
           {'name': 'LIVE_BUVID', 'value': 'a23404d890e536c9a93b72e5e50be145', 'path': '/', 'domain': '.bilibili.com', 'expiry': None, 'secure': False, 'httpOnly': False}]
# driver.delete_all_cookies()
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
element = WebDriverWait(driver, 10).until(
        EC.presence_of_element_located((By.NAME, 'file')))
upload = driver.find_element_by_name('file')
# time.sleep(5)
print(driver.title)
upload.send_keys(video_path)  # send_keys
# print(upload.get_attribute('value'))  # check value
while 1:
    info = driver.find_element_by_xpath('//*[@id="sortWrp"]/li/div[2]/div[1]/div[3]')
    print(info.text)
    time.sleep(20)
    text = driver.find_element_by_class_name('upload-status').text
    print(text)
    if text == 'Upload complete':
        break
# print(text)
# pageSource = driver.page_source
# print(pageSource)
# hid = driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[1]/div/div[1]/label[1]/div')
# driver.execute_script('arguments[0].style.display="none";',hid)
# hid_1 = driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[1]/div/div[1]/label[1]')
# driver.execute_script('arguments[0].style.display="none";',hid_1)
#
# time.sleep(1)
click = driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[1]/div/div[1]/label[1]/div')
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
click_1 = driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[1]/div/div[3]/div[1]/div[2]/label/div')
click_1.click()
# 点击游戏
driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[2]/div[1]/ul/li[3]/button').click()
# 点击电子竞技
driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[2]/div[1]/ul/li[3]/ul/li[6]').click()
text = driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[4]/div[2]/div[2]/div[2]/div/div/input')
text.send_keys('星际2')
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
text_1 = driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[5]/div[1]/div[1]/div[1]/div/input')
text_1.send_keys('星际争霸2')
# 简介
text_2 = driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[1]/div[5]/div[1]/div[2]/div[1]/div/textarea')
text_2.send_keys('职业选手直播第一视角录像。')

driver.find_element_by_xpath('//*[@id="app"]/div[2]/div[2]/div/div[2]/div[2]/div[2]/div[3]/button').click()

print('稿件提交完成！')
driver.quit()
