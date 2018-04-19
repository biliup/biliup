import requests
import re

'/html/body/div[2]/div/div/div[3]/div[3]/div/div/ul/li[6]/a[1]'
'//*[@id="userId"]'
'//*[@id="passwd"]'
'/html/body/div[1]/div/div[2]/form/div/div[2]/div/p/a[1]'
'//*[@id="email"]'
'/html/body/div/div/div[2]/div/div[2]/div[2]/p/a[1]'
class Bilibili(object):
    def __init__(self):
        self.session = requests.session()

    def login(self, user, pwd):
        """
        .. warning::
           | THE API IS NOT OFFICIAL API
           | DETAILS: https://api.kaaass.net/biliapi/docs/
        :param user: username
        :type user: str
        :param pwd: password
        :type pwd: str
        :return: if success return True
                 else return msg json
        """
        r = requests.post(
                url='https://api.kaaass.net/biliapi/user/login',
                data={
                    'user'  : user,
                    'passwd': pwd
                },
                headers={
                    'x-requested-with': 'XMLHttpRequest'
                }

        )
        # {"ts":1498361700,"status":"OK","mid":132604873,"access_key":"fb6c52162481d92a20875aca101ebe92","expires":1500953701}
        # print(r.text)
        if r.json()['status'] != 'OK':
            return r.json()

        access_key = r.json()['access_key']
        r = requests.get(
                url='https://api.kaaass.net/biliapi/user/sso?access_key=' + access_key,
                headers={
                    'x-requested-with': 'XMLHttpRequest'
                }
        )
        # {"ts":1498361701,"status":"OK","cookie":"sid=4jj9426i; DedeUserID=132604873; DedeUserID__ckMd5=e6a58ccc06aec8f8; SESSDATA=4de5769d%2C1498404903%2Cd86e4dea; bili_jct=5114b3630514ab72df2cb2e7e6fcd2eb"}
        # print(r.text)

        if r.json()['status'] == 'OK':
            cookie = r.json()['cookie']
            self.session.headers["cookie"] = cookie
            self.csrf = re.search('bili_jct=(.*?);', cookie + ';').group(1)
            self.mid = re.search('DedeUserID=(.*?);', cookie + ';').group(1)
            self.session.headers['Accept'] = 'application/json, text/javascript, */*; q=0.01'
            self.session.headers['Referer'] = 'https://space.bilibili.com/{mid}/#!/'.format(mid=self.mid)
            # session.headers['User-Agent'] = 'Mozilla/5.0 (Windows NT 6.1; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/57.0.2987.133 Safari/537.36'
            # session.headers['Content-Type'] = 'application/x-www-form-urlencoded; charset=UTF-8'
            return True
        else:
            return r.json()
    def home_page(self):
        home = self.session.get('https://www.bilibili.com/')
        page = self.session.get('https://member.bilibili.com/v/video/submit.html')
        print(home.headers)
        print(page.cookies._cookies)
        print(self.session.headers)

if __name__ == '__main__':
    # b = Bilibili()
    # b.login('y446970841@163.com', '1122000')
    # b.home_page()
    response = requests.get('https://member.bilibili.com/v/video/submit.html')
    print(response.text)