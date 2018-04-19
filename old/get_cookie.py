import json
from selenium import webdriver
import time
from selenium.webdriver.chrome.options import Options

chrome_options = Options()
chrome_options.add_argument("--headless")
driver = webdriver.Chrome(executable_path=r'D:\bilibiliupload\chromedriver.exe',
    chrome_options=chrome_options)

driver = webdriver.Chrome(executable_path=r'D:\bilibiliupload\chromedriver.exe')

driver.get("https://www.bilibili.com")

# with open('bilibili.cookie') as f:
#     cookies = json.load(f)
# # cookies = [{'name': 'sid', 'value': '97rjjtc1'}, {'name': 'DedeUserID', 'value': '274986200'}, {'name': 'DedeUserID__ckMd5', 'value': '1206e39bc044fe98'}, {'name': 'SESSDATA', 'value': '3e02f3d0%2C1521242566%2Cbd06c98d'}, {'name': 'bili_jct', 'value': '3b092ca1cacc59b2b7679bd997afe5ca'}]
# cookies = bilibili_api.cform('buvid2=52416955-9EB7-46AD-9422-46E2345654E126585infoc; dssid=7bjk90112abbebf8c; dsess=BAh7CkkiD3Nlc3Npb25faWQGOgZFVEkiFTkwMTEyYWJiZWJmOGMzOTkGOwBG%0ASSIJY3NyZgY7AEZJIiVhMTBmMTM2MzkyYWYyOWNiNmQyZTJlNWUxMTMxZTJj%0AZgY7AEZJIg10cmFja2luZwY7AEZ7B0kiFEhUVFBfVVNFUl9BR0VOVAY7AFRJ%0AIi1jYmQyZDYyYTc2MGZjMmViYzk1NDA2MGQ5NGNmZWVkYWFhNDk0YTU1BjsA%0ARkkiGUhUVFBfQUNDRVBUX0xBTkdVQUdFBjsAVEkiLWNhNGFlZTBlODEyMTRh%0AZGRjNWZiMTI4NzdjZjllNWM4YjhiZWI3ZDYGOwBGSSIKY3RpbWUGOwBGbCsH%0AY3cRWkkiCGNpcAY7AEYiEzExNS4xNTUuMTE0Ljk1%0A--b014e84af2e4612099ac4348d29205b2813fd930; fts=1511094141; UM_distinctid=15fdc423182b22-07a38f91352cb7-7b113d-144000-15fdc423183a13; pgv_pvi=3700880384; rpdid=olklsxmolldosooiiqlpw; sid=8k9huzd0; LIVE_BUVID=a23404d890e536c9a93b72e5e50be145; LIVE_BUVID__ckMd5=d5ea746722041186; buvid3=755A6D1D-A15D-40D5-BCEB-F96E5A67F85444996infoc; im_notify_type_274986200=0; finger=edc6ecda; im_seqno_274986200=14; im_local_unread_274986200=0; DedeUserID=274986200; DedeUserID__ckMd5=1206e39bc044fe98; SESSDATA=3e02f3d0%2C1523167736%2C0bbe16f2; bili_jct=11d9ff352f4a89846abebe85daadde3a; pgv_si=s6206720000; _dfcaptcha=0755ca3de2fba71d32605654777511a9')
# print(cookies)
# # driver.delete_all_cookies()
# for cookie in cookies:
#     # cookie.pop('domain')
#     # print(cookie)
#     driver.add_cookie(cookie)
# driver.get("https://www.bilibili.com")
driver.get("https://member.bilibili.com/video/upload.html")

# WebDriverWait(driver, 10).until(
#             EC.presence_of_element_located((By.XPATH, r'/html/body/div[2]/div/div/div[3]/div[3]/div/div/ul/li[6]/a[1]')))

# time.sleep(1)
# xl = driver.find_element_by_xpath(r'/html/body/div[2]/div/div/div[3]/div[3]/div/div/ul/li[6]/a[1]')
# xl.click()

# WebDriverWait(driver, 10).until(JSESSIONID=E9A43CDCEEE9204A5D67180C9E22CF67
#             EC.presence_of_element_located((By.XPATH, r'//*[@id="userId"]')))

# time.sleep(2)
username = driver.find_element_by_xpath(r'//*[@id="login-username"]')
username.send_keys('y446970841@163.com')

password = driver.find_element_by_xpath('//*[@id="login-passwd"]')
password.send_keys('1122000')

# login = driver.find_element_by_xpath(r'/html/body/div[1]/div/div[2]/form/div/div[2]/div/p/a[1]')
# ActionChains(driver).double_click(login).perform()
# # login.click()
#
# time.sleep(2)
# email = driver.find_element_by_xpath(r'//*[@id="email"]')
# email.click()
#
# loging = driver.find_element_by_xpath(r'/html/body/div/div/div[2]/div/div[2]/div[2]/p/a[1]')
# loging.click()



# driver.save_screenshot('screenshot.png')

time.sleep(8)

# driver.save_screenshot('screenshot1.png')

cookie = driver.get_cookies()
print(cookie)
filename = 'bilibili.cookie'
with open(filename, "w") as f:
    json.dump(cookie, f)

with open(filename) as f:
    new_cookie = json.load(f)

print(new_cookie)
print(type(new_cookie),type([]))
