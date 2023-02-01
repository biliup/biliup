# biliup
![](https://img.shields.io/badge/python-v3.7%2B-blue)
![GitHub](https://img.shields.io/github/license/ForgQi/bilibiliupload)
[![Telegram](https://img.shields.io/badge/Telegram-Group-blue.svg?logo=telegram)](https://t.me/+IkpIABHqy6U0ZTQ5)
[![Discord chat][discord-badge]][discord-url]

[discord-badge]: https://img.shields.io/discord/1015494098481852447.svg?logo=discord
[discord-url]: https://discord.gg/shZmdxDFB7

* 支持自动录制各大直播平台实时流，上传到bilibili
* 支持YouTube，twitch直播回放列表自动搬运至b站，如链接https://www.twitch.tv/xxxx/videos?filter=archives&sort=time
* 自动选择上传线路，保证国内外vps上传质量和速度
* 可分别控制下载与上传并发量
* 支持 cos-internal，腾讯云上海内网上传，免流 + 大幅提速
* 实验性功能：
    - 防止录制花屏
    - 启动时加入`--http`选项并访问localhost:19159可使用webUI (建议使用toml配置文件)

> 详细安装教程:
> * [快速上手视频](https://www.bilibili.com/video/BV1jB4y1p7TK/)教程 by [@milk](https://github.com/by123456by)
> * [Ubuntu](https://blog.waitsaber.org/archives/129) 、[CentOS](https://blog.waitsaber.org/archives/163)
、[Windows](https://blog.waitsaber.org/archives/169) 教程
与 [常见问题](https://blog.waitsaber.org/archives/167) 解决方案 by [@waitsaber](https://github.com/waitsaber)

**文档地址**：<https://biliup.github.io/biliup>
## INSTALLATION
1. 创建配置文件 [**config.toml**](#最小配置文件示例)
    ```toml
    # 以下为必填项
    [streamers."1xx直播录像"] # 设置直播间1
    url = ["https://www.twitch.tv/1xx"]
    tags = ["biliup"]
   
    # 设置直播间2
    [streamers."2xx直播录像"]
    url = ["https://www.twitch.tv/2xx"]
    tags = ["biliup"]            
    ```
2. 安装 __pip__ 并通过 pip 安装 __biliup__：
`pip3 install biliup`
```shell
# 在创建配置文件的目录启动 biliup
$ biliup start
# 退出
$ biliup stop
# 重启
$ biliup restart
# 查看版本
$ biliup --version
# 显示帮助以查看更多选项
$ biliup -h
# 启动 web ui, 默认 0.0.0.0:19159。 可使用-H及-P选项配置。考虑到安全性，建议指定本地地址配合web server或者添加验证。
$ biliup --http start
# 指定配置文件路径
$ biliup --config ./config.yaml start
```
从 v0.2.15 版本开始，配置文件支持 toml 格式，详见 [config.toml](https://github.com/biliup/biliup/blob/master/public/config.toml)，
yaml配置文件完整内容可参照 [config.yaml](https://github.com/ForgQi/bilibiliupload/blob/master/public/config.yaml)。
__FFmpeg__ 作为可选依赖。如果还有问题可以 [加群讨论](https://github.com/ForgQi/biliup/discussions/58#discussioncomment-2388776) 。

> 使用上传功能需要登录B站，通过 [命令行投稿工具](https://github.com/ForgQi/biliup-rs) 获取 cookies.json，并放入启动 biliup 的路径即可

Linux下以daemon进程启动，录像和日志文件保存在执行目录下，程序执行过程可查看日志文件。
`ps -A | grep biliup` 查看进程是否启动成功。

## Docker使用 🔨
### 方式一 拉取镜像
#### 从配置文件启动
```bash
vim /host/path/config.toml
docker run -P --name biliup -v /host/path:/opt -d ghcr.io/biliup/caution:master
```
#### 从配置文件启动，并启动web-ui
```bash
vim /host/path/config.toml
docker run -P --name biliup -v /host/path:/opt -p 19159:19159 -d --restart always ghcr.io/biliup/caution:latest --http --password yourpassword
```
yourpassword为web-ui的密码，用户名为biliup

#### 直接启动web-ui 自动生成配置文件
```bash
docker run -P --name biliup -v /host/path:/opt -p 19159:19159 -d --restart always ghcr.io/biliup/caution --http --password yourpassword
```


### 方式二 手动构建镜像
```bash
cd biliup
sudo docker build . -t biliup
sudo docker run -P -d biliup
```
### 进入容器 📦
```bash
sudo docker ps (找到你的imageId)
sudo docker exec -it imageId /bin/bash
```

## 调试源码
* 下载源码: git clone https://github.com/ForgQi/bilibiliupload.git
* 安装: `pip3 install -e .` 
* 启动: `python3 -m biliup`
* 构建: 
  ```shell
  $ npm install
  $ npm run build
  $ python3 -m build
  ```
* 调试 webUI: `python3 -m biliup --http --static-dir public`
## yaml配置文件示例
可选项见完整配置文件,
tid投稿分区见[Wiki](https://github.com/ForgQi/biliup/wiki)
```yaml
streamers:
    xxx直播录像:
        url:
            - https://www.twitch.tv/xxx
        tags: biliup
```
## EMBEDDING BILIUP
如果你不想使用完全自动托管的功能，而仅仅只是想嵌入biliup作为一个库来使用这里有两个例子可以作为参考
### 上传
```python
from biliup.plugins.bili_webup import BiliBili, Data

video = Data()
video.title = '视频标题'
video.desc = '视频简介'
video.source = '添加转载地址说明'
# 设置视频分区,默认为122 野生技能协会
video.tid = 171
video.set_tag(['星际争霸2', '电子竞技'])
video.dynamic = '动态内容'
lines = 'AUTO'
tasks = 3
dtime = 7200 # 延后时间，单位秒
with BiliBili(video) as bili:
    bili.login("bili.cookie", {
        'cookies':{
            'SESSDATA': 'your SESSDATA',
            'bili_jct': 'your bili_jct',
            'DedeUserID__ckMd5': 'your ckMd5',
            'DedeUserID': 'your DedeUserID'
        },'access_token': 'your access_key'})
    # bili.login_by_password("username", "password")
    for file in file_list:
        video_part = bili.upload_file(file, lines=lines, tasks=tasks)  # 上传视频，默认线路AUTO自动选择，线程数量3。
        video.append(video_part)  # 添加已经上传的视频
    video.dtime = dtime # 设置延后发布（2小时~15天）
    video.cover = bili.cover_up('/cover_path').replace('http:', '')
    ret = bili.submit()  # 提交视频
```
### 下载
```python
from biliup.downloader import download

download('文件名', 'https://www.panda.tv/1150595', suffix='flv')
```
## 使用建议
国内VPS网络费用较高，建议使用国外VPS，根据机器的硬盘等资源设置合理并发量, 选择kodo线路较容易跑满带宽。

b站上传目前有两种模式，分别为bup和bupfetch模式。
>* bup：国内常用模式，视频直接上传到b站投稿系统。
>* bupfetch：目前见于国外网络环境，视频首先上传至第三方文件系统，上传结束后通知bilibili投稿系统，再由b站投稿系统从第三方系统拉取视频，以保证某些地区用户的上传体验。

bup模式支持的上传方式为upos，其线路有：
* ws（网宿）
* qn（七牛）
* bda2（百度）

bupfetch模式支持的上传方式及线路有：
1. kodo（七牛）
2. gcs（谷歌）
3. bos（百度）

国内基本选择upos模式的bda2线路。国外多为upos模式的ws和qn线路，也有bupfetch模式的kodo、gcs线路。bilibili采用客户端和服务器端线路探测相结合的方式，服务器会返回可选线路，客户端上传前会先发包测试选择一条延迟最低的线路，保证各个地区的上传质量。
***
登录有两种方案：

* 操作浏览器模拟登录

* 通过b站的OAuth2接口

>对于滑动验证码可进行二值化、灰度处理找缺口计算移动像素，系统会上传分析你的拖动行为，模拟人操作轨迹，提供加速度、抖动等，如直接拖动到目标位置不能通过验证，提示：“拼图被怪物吃了”。滑动验证码系统会学习，需不断更新轨迹策略保证通过验证的成功率。\
>OAuth2接口要提供key，需逆向分析各端

线程池限制并发数，减少磁盘占满的可能性。检测下载情况卡死或者下载超时，重试三次保证可用性。代码更新后将在空闲时自动重启。


下载整合了ykdl、youtube-dl、streamlink，不支持或者支持的不够好的网站可自行拓展。
下载和上传模块插件化，如果有上传或下载目前不支持平台的需求便于拓展。

下载基类在`engine/plugins/base_adapter.py`中，拓展其他网站，需要继承下载模块的基类，加装饰器`@Plugin.download`。

拓展上传平台，继承`engine/plugins/upload/__init__.py`文件中上传基类，加装饰器`@Plugin.upload`。

实现了一套基于装饰器的事件驱动框架。增加其他功能监听对应事件即可，比如下载后转码：
```python
# e.p.给函数注册事件
# 如果操作耗时请指定block=True, 否则会卡住事件循环
@event_manager.register("download_finish", block=True)
def transcoding(data):
    pass
```
'''
        Please install at least one of the following Javascript interpreter.'
        python packages: PyChakra, quickjs
        applications: Gjs, CJS, QuickJS, JavaScriptCore, Node.js, etc.'''

开机自启可参照以下模板创建systemd unit:

创建service文件：
> $ nano ~/.config/systemd/user/biliupd.service
```
[Unit]
Description=Biliup Startup
Documentation="https://biliup.github.io/biliup"
Wants=network-online.target
After=network-online.target

[Service]
Type=simple
WorkingDirectory=[在此填入你的config所在目录]
ExecStart=/usr/bin/biliup -v
ExecReload=/usr/bin/biliup restart
ExecStop=/usr/bin/biliup stop

[Install]
WantedBy=default.target
```
启用service并启动：
```shell
$ systemctl --user enable biliupd
$ systemctl --user start biliupd
```

## Deprecated
* ~~selenium操作浏览器上传两种方式~~(详见bili_chromeup.py)
* ~~Windows图形界面版在release中下载AutoTool.msi进行安装~~[AutoTool.msi](https://github.com/ForgQi/bilibiliupload/releases/tag/v0.1.0)
* 相关配置示例在config.yaml文件中，如直播间地址，b站账号密码\
由于目前使用账号密码登录，大概率触发验证。请使用命令行工具登录，将登录返回的信息填入配置文件，
且使用引号括起yaml中cookie的数字代表其为字符串

>关于B站为什么不能多p上传\
目前bilibili网页端是根据用户权重来限制分p数量的，权重不够的用户切换到客户端的提交接口即可解除这一限制。
>用户等级大于3，且粉丝数>1000，web端投稿不限制分p数量

## Credits
* Thanks `ykdl, youtube-dl, streamlink` provides downloader.
>GUI：[B站投稿客户端 biliup-app](https://github.com/ForgQi/Caution)
