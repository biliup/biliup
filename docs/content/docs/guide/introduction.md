+++
title = "Introduction"
description = "AdiDoks is a Zola theme helping you build modern documentation websites, which is a port of the Hugo theme Doks for Zola."
date = 2021-05-01T08:00:00+00:00
updated = 2021-05-01T08:00:00+00:00
draft = false
weight = 10
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ' <a href="https://github.com/biliup/biliup">biliup</a>是一组工具集，旨在降低使用、开发自动化b站投稿的难度，同时提供了b站web端、客户端投稿工具未开放的一些功能，如多p投稿，线路选择，并发数设置，直播录制，视频搬运等.'
toc = true
top = false
+++

## 详细安装教程:
* [快速上手视频教程](https://www.bilibili.com/video/BV1jB4y1p7TK/) by [@milk](https://github.com/by123456by)
* [Ubuntu](https://blog.waitsaber.org/archives/129) 、[CentOS](https://blog.waitsaber.org/archives/163)
、[Windows](https://blog.waitsaber.org/archives/169) 教程 by [@waitsaber](https://github.com/waitsaber)
* [常见问题解决方案](https://blog.waitsaber.org/archives/167) by [@waitsaber](https://github.com/waitsaber)


## INSTALLATION
0. 安装 __Python 3.7+__ 和 __pip__
 > 如需录制 斗鱼(Douyu) 平台，请额外安装至少一个 __JavaScript 解释器__。
 > 支持且不限于以下的  __JavaScript 解释器__，点击名字可跳转至下载页。
 > Please install at least one of the following Javascript interpreter.
 > python packages: [QuickJS](https://pypi.org/project/quickjs/)
 > applications: [Node.js](https://nodejs.org/zh-cn/download)
1. 创建配置文件 **[config.toml](https://github.com/biliup/biliup/tree/master/public/config.toml)**
    ```toml
    # 以下为必填项
    [streamers."1xx直播录像"] # 替换 1xx直播录像 为 主播名
    url = ["https://www.twitch.tv/1xx"]
    tags = ["biliup"]

    # 设置直播间2
    [streamers."2xx直播录像"] # 注意不能与其他 主播名 重复
    url = ["https://www.twitch.tv/2xx"]
    tags = ["biliup"]
    ```
2. 通过 pip 安装 __biliup__：
`pip3 install biliup`
3. 开始使用 __biliup__：
```shell
# 默认监听 0.0.0.0:19159。可使用-H及-P选项配置。
# 考虑到安全性，建议指定本地地址配合web server或者添加验证。
$ biliup start
# 退出
$ biliup stop
# 重启
$ biliup restart
# 查看版本
$ biliup --version
# 显示帮助以查看更多选项
$ biliup -h
# 指定配置文件路径
$ biliup --config ./config.yaml start
```
从 v0.2.15 版本开始，配置文件支持 toml 格式，详见 [config.toml](https://github.com/biliup/biliup/tree/master/public/config.toml) ，
yaml配置文件完整内容可参照 [config.yaml](https://github.com/biliup/biliup/tree/master/public/config.yaml) 。
__FFmpeg__ 作为可选依赖。如果还有问题可以 [加群讨论](https://github.com/ForgQi/biliup/discussions/58#discussioncomment-2388776) 。

> 使用上传功能需要登录B站，通过 [命令行投稿工具](https://github.com/ForgQi/biliup-rs) 获取 cookies.json，并放入启动 biliup 的路径即可

> ARM平台用户，需要使用到stream-gears（默认下载器与上传器）进行下载和上传的，请参考此教程降级stream-gears版本。 https://github.com/biliup/biliup/discussions/407

> Linux下以daemon进程启动，录像和日志文件保存在执行目录下，程序执行过程可查看日志文件。启动之后使用命令`ps -A | grep biliup` 查看进程biliup是否启动成功。


## Docker使用 🔨
### 方式一 拉取镜像
 > 请注意替换 /host/path 为宿主机下载目录
* 从自定义的配置文件启动
```bash
# 在下载目录创建配置文件
vim /host/path/config.toml
# 启动biliup的docker容器
docker run -P --name biliup -v /host/path:/opt -d ghcr.io/biliup/caution:master
```
* 从自定义的配置文件启动
```bash
# 在下载目录创建配置文件
vim /host/path/config.toml
# 启动biliup的docker容器，并启用用户验证。请注意替换 yourpassword 为你的密码。
docker run -P --name biliup -v /host/path:/opt -p 19159:19159 -d --restart always ghcr.io/biliup/caution:latest --password yourpassword
```
 > Web-UI 默认用户名为 biliup。
* 从默认配置文件启动
```bash
docker run -P --name biliup -v /host/path:/opt -p 19159:19159 -d --restart always ghcr.io/biliup/caution:latest --password yourpassword
```
### 方式二 手动构建镜像
```bash
# 进入biliup目录
cd biliup
# 构建镜像
sudo docker build . -t biliup
# 启动镜像
sudo docker run -P -d biliup
```
### 进入容器 📦
1. 查看容器列表，找到你要进入的容器的imageId
```bash
sudo docker ps
```
2. 进入容器
```bash
sudo docker exec -it imageId /bin/bash
```


## 从源码运行biliup
* 下载源码: `git clone https://github.com/ForgQi/bilibiliupload.git`
* 安装: `pip3 install -e .`
* 启动: `python3 -m biliup`
* 构建:
  ```shell
  $ npm install
  $ npm run build
  $ python3 -m build
  ```
* 调试 webUI: `python3 -m biliup --static-dir public`


## yaml配置文件示例
可选项见[完整配置文件](https://github.com/biliup/biliup/tree/master/public/config.yaml),
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
    video.delay_time(dtime) # 设置延后发布（2小时~15天）
    video.cover = bili.cover_up('/cover_path').replace('http:', '')
    ret = bili.submit()  # 提交视频
```
### 下载
```python
from biliup.downloader import download

download('文件名', 'https://www.panda.tv/1150595', suffix='flv')
```


## 使用建议
### 1. VPS上传线路选择
国内VPS网络费用较高，建议使用国外VPS，根据机器的硬盘等资源设置合理并发量, 选择kodo线路较容易跑满带宽。

b站上传目前有两种模式，分别为bup和bupfetch模式。
> bup：国内常用模式，视频直接上传到b站投稿系统。
>
> bupfetch：目前见于国外网络环境，视频首先上传至第三方文件系统，上传结束后通知bilibili投稿系统，再由b站投稿系统从第三方系统拉取视频，以保证某些地区用户的上传体验。

bup模式支持的上传方式为upos，其线路有：
* ws（网宿）
* qn（七牛）
* bda2（百度）

bupfetch模式支持的上传方式及线路有：
* ~~kodo（七牛）已失效~~
* ~~gcs (谷歌）已失效~~
* ~~bos (百度）已失效~~

国内基本选择upos模式的bda2线路。国外多为upos模式的ws和qn线路，也有bupfetch模式的kodo、gcs线路。bilibili采用客户端和服务器端线路探测相结合的方式，服务器会返回可选线路，客户端上传前会先发包测试选择一条延迟最低的线路，保证各个地区的上传质量。

### 2. 登录方案
登录有两种方案：
* 操作浏览器模拟登录
* 通过b站的OAuth2接口
> 对于滑动验证码可进行二值化、灰度处理找缺口计算移动像素，系统会上传分析你的拖动行为，模拟人操作轨迹，提供加速度、抖动等，如直接拖动到目标位置不能通过验证，提示：“拼图被怪物吃了”。滑动验证码系统会学习，需不断更新轨迹策略保证通过验证的成功率。\
> OAuth2接口要提供key，需逆向分析各端

### 3. 推荐biliup配置
线程池限制并发数，减少磁盘占满的可能性。
> 检测到下载情况卡死或者下载超时，biliup会重试三次保证可用性。代码更新后将在空闲时自动重启。

### 4. 关于录制的XML弹幕文件如何使用
使用方法有很多种：
- 使用 [DanmakuFactory](https://github.com/hihkm/DanmakuFactory) 将XML弹幕文件转化为ASS字幕文件，然后使用一般播放器外挂加载字幕
- [AList](https://alist.nn.ci/zh/) 检测到同文件夹下的XML文件会自动挂载弹幕，实现带弹幕的录播效果
- 使用 [弹弹play](https://www.dandanplay.com/) 可直接挂载XML弹幕文件观看


## 自定义插件
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

## LINUX下配置开机自启
开机自启可参照以下模板创建systemd unit:
1. 创建service文件：
```shell
$ nano ~/.config/systemd/user/biliupd.service
```
2. service文件的内容
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
3. 启用service并启动：
```shell
$ systemctl --user enable biliupd
$ systemctl --user start biliupd
```


## Deprecated
* ~~selenium操作浏览器上传两种方式(详见bili_chromeup.py)~~
* ~~Windows图形界面版在release中下载AutoTool.msi进行安装~~[~~AutoTool.msi~~](https://github.com/ForgQi/bilibiliupload/releases/tag/v0.1.0)
* 相关配置示例在[config.yaml](https://github.com/biliup/biliup/tree/master/public/config.yaml)、[config.toml](https://github.com/biliup/biliup/tree/master/public/config.toml)文件中，如直播间地址，b站账号密码等等
* 由于目前使用账号密码登录，大概率触发验证。请使用命令行工具登录，将登录返回的信息填入配置文件，且使用引号括起yaml中cookie的数字代表其为字符串

> 关于B站为什么不能多p上传\
目前bilibili网页端是根据用户权重来限制分p数量的，权重不够的用户切换到客户端的提交接口即可解除这一限制。
> 用户等级大于3，且粉丝数>1000，web端投稿不限制分p数量

