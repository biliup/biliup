import json

from selenium import webdriver
import time

driver = webdriver.Chrome(executable_path=r'D:\bilibiliupload\chromedriver.exe')
driver.get("https://www.bilibili.com")

cookies = [{'domain': '.bilibili.com', 'expiry': 1611825707.027044, 'httpOnly': False, 'name': 'buvid3', 'path': '/',
            'secure': False, 'value': 'A558A031-653B-4890-917D-C205F81B84F93220infoc'},
           {'domain': '.bilibili.com', 'expiry': 1611825706, 'httpOnly': False, 'name': 'fts', 'path': '/',
            'secure': False, 'value': '1517217707'},
           {'domain': '.bilibili.com', 'expiry': 1519809706, 'httpOnly': False, 'name': 'finger', 'path': '/',
            'secure': False, 'value': 'edc6ecda'},
           {'domain': '.bilibili.com', 'expiry': 1519809723.74952, 'httpOnly': False, 'name': 'DedeUserID', 'path': '/',
            'secure': False, 'value': '274986200'},
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
           {'domain': '.bilibili.com', 'expiry': 1517221325.776721, 'httpOnly': True, 'name': '_dfcaptcha', 'path': '/',
            'secure': False, 'value': 'a32213f1be85682d37cde24aedc2d135'}]
for cookie in cookies:
    # cookie.pop('domain')
    # print(cookie)
    driver.add_cookie(cookie)

driver.get("https://member.bilibili.com/video/upload.html")
time.sleep(30)
cookie = driver.get_cookies()
print(cookie)
filename = 'bilibili.cookie'
with open(filename, "w") as f:
    json.dump(cookie, f)

with open(filename) as f:
    new_cookie = json.load(f)

print(new_cookie)
print(type(new_cookie),type([]))
