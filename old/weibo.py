from selenium import webdriver



driver = webdriver.Chrome(executable_path=r'D:\bilibiliupload\chromedriver.exe')

driver.get("https://api.weibo.com/oauth2/authorize?client_id=2841902482&redirect_uri=https://passport.bilibili.com/login/snsback?sns=weibo&state=2a4055b02e5411e881f60242ac123e5a&scope=email")

cookie = {'name': 'JSESSIONID', 'value': 'E9A43CDCEEE9204A5D67180C9E22CF67'}

driver.add_cookie(cookie)

driver.get('https://api.weibo.com/oauth2/authorize')


