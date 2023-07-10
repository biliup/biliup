# biliup

![Python](https://img.shields.io/badge/Python-3.7%2B-blue)
![License](https://img.shields.io/badge/License-MIT-green)
[![Telegram](https://img.shields.io/badge/Telegram-Group-blue.svg?logo=telegram)](https://t.me/+IkpIABHqy6U0ZTQ5)
[![Discord](https://img.shields.io/discord/1015494098481852447.svg?logo=discord)](https://discord.gg/shZmdxDFB7)
[![QQ群](https://img.shields.io/badge/QQ群-biliup-blue)](http://qm.qq.com/cgi-bin/qm/qr?_wv=1027&k=YzQksrDAhOk5EnCynXFD1BiJrwq9XzEb&authKey=B9As%2FNDWGWOgEc0Pz9MVbZRihPq1m%2BPeOV4NASMX4Uky1MZLV59eXvYMVNHtQE9W&noverify=0&group_code=130816738)

biliup是一款自动化直播录制工具，并能将录播投稿至哔哩哔哩。


# 功能

* 支持录制各种直播平台，包括但不限于AcFun、AfreecaTV、哔哩哔哩、斗鱼、抖音、虎牙、网易CC、Niconico、猫耳FM、
Twitch、YY直播等，并能在录制结束后投稿至哔哩哔哩
* 支持将YouTube、Twitch直播播放列表搬运至哔哩哔哩
* 支持录制哔哩哔哩、斗鱼、虎牙、Twitch的直播弹幕，生成B站标准格式的XML弹幕文件，可被各种常见的弹幕挂载程序使用处理
* 自动选择上传线路，保证国内外VPS上传质量和速度
* 可分别控制下载与上传并发量
* ~~支持 cos-internal，腾讯云上海内网上传、免流和大幅提速~~
* 实验性功能：
    - 防止录制花屏（仅当使用stream-gears下载器时）
    - 启动时加入`--http`选项并访问 localhost:19159 可使用webUI

**更新日志**：[CHANGELOG.md](https://github.com/biliup/biliup/blob/master/CHANGELOG.md)

**文档地址**：<https://biliup.github.io/biliup>


# 安装教程
* [快速上手视频教程](https://www.bilibili.com/video/BV1jB4y1p7TK/) by [@milk](https://github.com/by123456by)
* [Ubuntu](https://blog.waitsaber.org/archives/129) 、[CentOS](https://blog.waitsaber.org/archives/163)
、[Windows](https://blog.waitsaber.org/archives/169) 教程 by [@waitsaber](https://github.com/waitsaber)
* [常见问题解决方案](https://blog.waitsaber.org/archives/167) by [@waitsaber](https://github.com/waitsaber)


# 安装
通过pip安装biliup：

```shell
$ pip3 install biliup
```


# 使用

直接运行：

```shell
# 在当前目录启动biliup
$ biliup start
# 退出
$ biliup stop
# 重启
$ biliup restart
# 查看版本
$ biliup --version
# 显示帮助以查看更多选项
$ biliup -h
# 启动webUI, 默认 0.0.0.0:19159。可使用-H及-P选项配置。考虑到安全性，建议指定本地地址配合web server或添加验证
$ biliup --http start
# 指定配置文件
$ biliup --config ./config.yaml start
```
biliup需要配置文件才能启动，支持yaml和toml两种配置文件，以下为最简配置示例：

toml配置文件：

```toml
[streamers."xxx直播录像"]
url = ["https://www.twitch.tv/xxx"]
```

yaml配置文件：

```yaml
streamers:
    xxx直播录像:
        url:
            - https://www.twitch.tv/xxx
```

更多可配置内容请参阅[选项](#选项)，示例文件请参考[config.toml](https://github.com/biliup/biliup/tree/master/public/config.toml)、[config.yaml](https://github.com/biliup/biliup/tree/master/public/config.yaml)

> 需要上传到B站时，请通过[命令行投稿工具](https://github.com/biliup/biliup-rs)获取cookies.json，并放入启动biliup的目录

> ARM平台用户如果需要使用stream-gears（默认下载器与上传器）进行下载和上传，请参考[此教程](https://github.com/biliup/biliup/discussions/407)降级stream-gears

> Linux下以daemon进程启动，直播和日志文件保存在执行目录下，程序执行时可查看日志文件。启动后可使用命令`ps -A | grep biliup`查看biliup是否启动成功


# 选项

## 全局选项

### 下载选项

    downloader                  下载器，可选stream-gears（默认），ffmpeg（需手动安装
                                ffmpeg），streamlink（需手动安装streamlink和ffmpeg）
    segment_time                录播单文件时间限制，格式HH:MM:SS，超过此时长分段下载
    file_size                   录播单文件大小限制，单位Byte，超过此大小分段下载，与
                                segment_time同时使用时将被忽略
    filtering_threshold         录播文件过滤，小于此大小的文件将被删除，单位MB
    filename_prefix             自定义录播文件名模板，支持变量{streamer}：配置文件中设置的
                                直播间名，strftime（%Y-%m-%d %H-%M-%S）：文件创建时间，
                                {title}：直播间标题

### B站上传选项

    uploader                    上传器，可选biliup-rs（默认），bili_web，Noop（不上传）
    submit_api                  上传提交接口，默认自动选择，可选web，client
    lines                       上传线路，可选AUTO（默认），bda2，kodo，ws，qn，cos，
                                cos-internal
    threads                     单文件上传线程数
    uploading_record            上传时检测到开播也进行录制（实验性功能），可选false
                                （默认），true
    use_live_cover              使用直播间封面作为投稿封面，优先级低于主播选项指定的
                                cover_path，目前仅支持哔哩哔哩。直播封面将会保存于cover目
                                录下可选false（默认），true

### 杂项

    delay                       主播下播后延迟再次检测时间，避免提早上传后导致漏录，单位
                                秒，可选范围0~1800
    event_loop_interval         检测时间间隔，单位秒
    checker_sleep               相同平台检测时间间隔，单位秒
    pool1_size                  线程池1大小，负责下载事件
    pool2_size                  线程池2大小，处理下载事件以外的所有事件
    check_sourcecode            检测源码文件变化间隔，单位秒，检测到源码变化后，程序会在空
                                闲时自动重启

## 平台选项

### 斗鱼

    douyucdn                    斗鱼CDN线路，可选tctc-h5（备用线路4），tct-h5（备用线路
                                5），ali-h5（备用线路6），hw-h5（备用线路7），hs-h5（备用
                                线路13）
    douyu_danmaku               斗鱼弹幕录制，目前暂不支持对按时长分段的录播的弹幕文件进行
                                自动分段，且只有选择ffmpeg或streamlink作为下载器时才支持，
                                可选false（默认），true

### 虎牙

    huyacdn                     虎牙CDN线路，可选AL（阿里云），HW（华为云），TX（腾讯云），
                                WS（网宿），HS（火山引擎），AL13（阿里云），HW16（华为云）
    huya_danmaku                虎牙弹幕录制，目前暂不支持对按时长分段的录播的弹幕文件进行
                                自动分段，且只有选择ffmpeg或streamlink作为下载器时才支持，
                                可选false（默认），true

### 哔哩哔哩
    bilibili_danmaku            哔哩哔哩弹幕录制，目前暂不支持对按时长分段的录播的弹幕文件
                                进行自动分段，且只有选择ffmpeg或streamlink作为下载器时才支
                                持，可选false（默认），true
    bili_protocol               哔哩哔哩直播流协议，可选stream（flv流，默认），hls_fmp4
                                （fmp4流，仅大陆可解析），hls_ts（ts流）。目前录制fmp4流需
                                要downloader为streamlink
    bili_perfCDN                哔哩哔哩直播优选CDN，可同时填入多个CDN节点
    bili_force_source           哔哩哔哩直播强制原画，可选false（默认），true
    bili_liveapi                自定义哔哩哔哩直播API
    bili_fallback_api           自定义哔哩哔哩直播回退API
    bili_cdn_fallback           哔哩哔哩CDN线路自动回退，可选true（默认），false
    bili_force_ov05_ip          强制指定ov-gotcha05的IP
    bili_force_cn01             强制指定cn-gotcha01，可选false（默认），true
    bili_force_cn01_domains     强制替换cn-gotcha01的域名，可同时填入多个域名

### YouTube

    youtube_prefer_vcodec       偏好视频编码，可选avc，av01，vp9
    youtube_prefer_acodec       偏好音频编码，可选mp4a，opus
    youtube_max_videosize       视频最大大小，如500M，2G。若所有画质都不符合将下载最低画质
    youtube_max_resolution      视频最大高度，如1080，1440，优先级低于
                                youtube_max_videosize
    use_youtube_cover           转载YouTube视频时自动获取视频封面并用于作投稿封面，封面将保
                                存在cover/youtube目录下，可选true（默认），false
    youtube_after_date          仅下载该日期后的视频，格式YYYYMMDD
    youtube_before_date         仅下载该日期前的视频，格式YYYYMMDD
    use_new_ytb_downloader      切换到streamlink下载模式，将只获取直播，仅当下载器为ffmpeg
                                时生效，开启后其他所有YouTube选项失效，可选false（默认），
                                true

### Twitch

    twitch_danmaku              Twitch弹幕录制，仅当下载器为ffmpeg时生效，可选false
                                （默认），true
    twitch_disable_ads          Twitch跳过广告片段，会导致录播分段，可选true（默认），
                                false

## 主播选项

    url                         直播链接、播放列表等
    title                       自定义稿件名称模板，支持变量{streamer}：配置文件中设置的直
                                播间名，{title}：直播间标题，strftime
                                （%Y-%m-%d %H-%M-%S）：文件创建时间，{url}：主播的第一条直
                                播间链接
    tid                         投稿分区码
    copyright                   投稿类型，可选1（自制，默认），2（转载）
    cover_path                  指定哔哩哔哩投稿封面文件
    use_live_cover              使用直播间封面作为投稿封面，优先级低于cover_path，目前仅支
                                持哔哩哔哩。直播封面将会保存于cover目录下
    description                 投稿简介，支持变量{streamer}，{title}，{url}，strftime
                                （%Y-%m-%d %H-%M-%S）
    dynamic                     哔哩哔哩投稿动态
    dtime                       延时发布时间戳，需距离投稿时间2小时~15天
    uploader                    上传器，将覆盖全局选项，可选biliup-rs（默认），
                                bili_web，Noop（不上传）
    filename_prefix             自定义录播文件名模板，将覆盖全局选项，支持变量{streamer}：
                                配置文件中设置的直播间名，strftime（%Y-%m-%d %H-%M-%S）：
                                文件创建时间，{title}：直播间标题
    user_cookie                 指定投稿的哔哩哔哩账号cookie文件
    tags                        哔哩哔哩投稿tag（如果要投稿到B站为必选项）
    preprocessor                直播开始时按自定义顺序执行命令，仅支持shell指令
    preprocessor                上传完成后（若不上传则为直播结束时）按自定义顺序执行命令，
                                仅支持shell指令。不使用该选项时，默认删除视频文件
    format                      视频保存格式，默认为flv，使用其他格式时需要下载器为ffmpeg或
                                streamlink
    opt_args                    ffmpeg参数

## 用户cookie

    # 哔哩哔哩
    bili_cookie                 用于获取直播流的cookie，至少需要SESSDATA，bili_jct，
                                DedeUserID__ckMd5，DedeUserID，access_token五个键值对（填
                                写时请用";"分开每个键值对）
    customAPI_use_cookie        向自定义直播API传递cookie，向第三方API传递cookie有风险，可
                                选false（默认），true
    
    # 抖音
    douyin_cookie               用于获取直播流的cookie，至少需要__ac_nonce，__ac_signature
                                两个键值对（填写时请用";"分开每个键值对）
    
    # Twitch
    twitch_cookie               用于获取直播流的cookie，直播间内的会员可免除该直播间的广
                                告，Twitch Turbo用户可大量减少广告，需填入auth-token的值。
                                仅当下载器为ffmpeg时生效
    
    # YouTube
    youtube_cookie              指定用于获取会限等内容的Netscape格式cookie文件
    
    # Niconico
    niconico-user-session       用于获取会限、高质量流等的cookie，需填入user_session的值
    niconico-email              Niconico用户名
    niconico-password           Niconico密码
    niconico-purge-credentials  清除缓存的Niconico凭证并重新认证


# Docker使用

## 方式一 拉取镜像
* 从配置文件启动
```bash
# 在指定目录创建配置文件
$ vim /host/path/config.toml
# 启动biliup的docker容器
$ docker run -P --name biliup -v /host/path:/opt -d ghcr.io/biliup/caution:latest
```
* 从配置文件启动，并启动webUI
```bash
# 在指定目录创建配置文件
$ vim /host/path/config.toml
# 启动biliup的docker容器，webUI的用户名为biliup，密码为password
$ docker run -P --name biliup -v /host/path:/opt -p 19159:19159 -d ghcr.io/biliup/caution:latest --http --password password
```
* 直接启动webUI并自动生成配置文件
```bash
$ docker run -P --name biliup -v /host/path:/opt -p 19159:19159 -d ghcr.io/biliup/caution:latest --http --password password
```
## 方式二 手动构建镜像
```bash
# 进入biliup目录
$ cd biliup
# 构建镜像
$ sudo docker build . -t biliup
# 启动镜像
$ sudo docker run -P -d biliup
```
## 进入容器
1. 查看容器列表，找到你要进入的容器的image ID
```bash
$ sudo docker ps
```
2. 进入容器
```bash
$ sudo docker exec -it imageID /bin/bash
```


# 从源码运行biliup
* 下载源码：`git clone https://github.com/biliup/biliup.git`
* 安装：`pip3 install -e .` 
* 启动：`python3 -m biliup`
* 构建：
  ```shell
  $ npm install
  $ npm run build
  $ python3 -m build
  ```
* 调试webUI：`python3 -m biliup --http --static-dir public`


# 嵌入biliup
如果你不想使用完全自动托管的功能，而仅仅只是想嵌入biliup作为一个库来使用，这里有两个例子可以参考：
## 上传
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
        video_part = bili.upload_file(file, lines=lines, tasks=tasks)  # 上传视频，默认线路AUTO自动选择，线程数量3
        video.append(video_part)  # 添加已经上传的视频
    video.delay_time(dtime) # 设置延后发布（2小时~15天）
    video.cover = bili.cover_up('/cover_path').replace('http:', '')
    ret = bili.submit()  # 提交视频
```
## 下载
```python
from biliup.downloader import download

download('文件名', 'https://www.panda.tv/1150595', suffix='flv')
```


# 使用建议
## VPS上传线路选择
国内VPS网络费用较高，建议使用国外VPS，根据VPS的CPU、硬盘等资源设置合理并发量。

B站上传目前有两种模式，分别为bup和bupfetch模式。

> bup：国内常用模式，视频直接上传到b站投稿系统。\
> bupfetch：目前见于国外网络环境，视频首先上传至第三方文件系统，上传结束后通知bilibili投稿系统，再由B站投稿系统从第三方拉取视频，以保证国外用户的上传体验。

bup模式支持的上传方式为upos，其线路有：
* ws（网宿）
* qn（七牛）
* bda2（百度）

bupfetch模式支持的上传方式及线路有：
* kodo（七牛）
* ~~gcs (谷歌）已失效~~
* ~~bos (百度）已失效~~

国内基本选择bup模式的bda2线路。国外多为bup模式的ws和qn线路，也有bupfetch模式的kodo线路。哔哩哔哩采用客户端和服务器端线路探测相结合的方式，服务器会返回可选线路，客户端上传前会先发包测试选择一条延迟最低的线路，保证上传质量。


## 登录方案
登录有两种方案：
* 操作浏览器模拟登录
* 通过B站的OAuth2接口
> 对于滑动验证码可进行二值化、灰度处理找缺口计算移动像素，系统会上传分析你的拖动行为，模拟人操作轨迹，提供加速度、抖动等，如直接拖动到目标位置不能通过验证，提示：”拼图被怪物吃了“。滑动验证码系统会学习，需不断更新轨迹策略保证通过验证的成功率。\
> OAuth2接口要提供key，需逆向分析各端。


## XML弹幕文件的使用
以下为几种使用方式：
- 使用 [DanmakuFactory](https://github.com/hihkm/DanmakuFactory) 将XML弹幕文件转换为ASS字幕文件，然后使用播放器加载外挂字幕
- [AList](https://alist.nn.ci/zh/) 检测到同目录下的XML弹幕文件会自动挂载，实现带弹幕的播放效果
- 使用 [弹弹play](https://www.dandanplay.com/) 可直接挂载XML弹幕文件观看


# 自定义插件
下载整合了 [ykdl](https://github.com/SeaHOH/ykdl)、[youtube-dl](https://github.com/ytdl-org/youtube-dl)、[streamlink](https://streamlink.github.io/)，不支持或者支持的不够好的网站可自行拓展。
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


# Linux配置开机自启
1. 创建service文件：
```shell
$ vim ~/.config/systemd/user/biliupd.service
```
2. service文件的内容：
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


# 弃用
* ~~selenium操作浏览器上传两种方式（详见bili_chromeup.py）~~
* ~~Windows图形界面版在release中下载[AutoTool.msi](https://github.com/biliup/biliup/releases/tag/v0.1.0)进行安装~~


# 感谢
* ykdl: https://github.com/SeaHOH/ykdl
* youtube-dl: https://github.com/ytdl-org/youtube-dl
* streamlink: https://streamlink.github.io/
* danmaku: https://github.com/THMonster/danmaku

> GUI：[B站投稿客户端 biliup-app](https://github.com/biliup/biliup-app)

