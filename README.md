<p align="center">
    <img src="https://image.biliup.me/2024-06-26/1719388842-365149-logo.png" width="400" alt="logo">
</p>

<div align="center">

[![Python](https://img.shields.io/badge/python-3.7%2B-blue)](http://www.python.org/download)
[![PyPI](https://img.shields.io/pypi/v/biliup)](https://pypi.org/project/biliup)
[![PyPI - Downloads](https://img.shields.io/pypi/dm/biliup)](https://pypi.org/project/biliup)
[![License](https://img.shields.io/github/license/biliup/biliup)](https://github.com/biliup/biliup/blob/master/LICENSE)
[![Telegram](https://img.shields.io/badge/Telegram-Group-blue.svg?logo=telegram)](https://t.me/+IkpIABHqy6U0ZTQ5)

[![GitHub Issues](https://img.shields.io/github/issues/biliup/biliup?label=Issues)](https://github.com/biliup/biliup/issues)
[![GitHub Stars](https://img.shields.io/github/stars/biliup/biliup)](https://github.com/biliup/biliup/stargazers)
[![GitHub Forks](https://img.shields.io/github/forks/biliup/biliup)](https://github.com/biliup/biliup/network)

</div>

---

## 🛠️ 核心功能

- 📥 **多平台支持**：录制主流直播平台内容并上传至 B 站/本地存储
- 🚄 **智能上传**：自动选择最优上传线路，支持手动调整并发
- ⚙️ **线路配置**：手动配置平台下载线路，避免画面断流
- 🔐 **多账号管理**：支持多账号切换上传，同时上传多账号
- 🏷️ **元数据定制**：自定义视频标题、标签、简介等信息

---

## 📜 更新日志

- **[更新日志 »](https://biliup.github.io/biliup/docs/guide/changelog)**

---

## 💬 交流与工具

- 💬 [交流社区](https://biliup.me/)
- 🛠️ [Windows 投稿工具](https://github.com/biliup/biliup-app)

---

## 📜 使用文档

- `编写中`

## 🚀 快速开始

### Windows
- 下载 exe: [Release](https://github.com/biliup/biliup/releases/latest)

### Linux 或 macOS
1. 确保 Python 版本 ≥ 3.8
2. 安装：`pip3 install biliup`
3. 启动：`biliup start`
4. 访问 WebUI：`http://your-ip:19159`

---

### 🐋
```sh
docker run -d \
  --name biliup \
  --restart unless-stopped \
  -p 0.0.0.0:19159:19159 \
  -v /path/to/save_folder:/opt \
  ghcr.io/biliup/caution:latest \
  --password password123
```
* 用户名`biliup`
* 公网暴露很危险，`password123`为密码，录制文件/日志存储在`/opt`。
* 根据需求进行修改，只作参考。

## 界面预览

![Light Theme](.github/resource/light.png)
![Dark Theme](.github/resource/dark.png)

---

## 🤝 开发

1. 确保 Node.js 版本 ≥ 18
2. 安装依赖：`npm i`
3. 启动开发服务器：`npm run dev`
4. 启动 Biliup：`python3 -m biliup`
5. 访问：`http://localhost:3000`

### 直播平台信息

| 直播平台     | 支持类型       | 链接示例                                                                                     | 特殊注释                                                                 |
|--------------|----------------|----------------------------------------------------------------------------------------------|--------------------------------------------------------------------------|
| 虎牙         | 直播           | [`https://www.huya.com/123456`](https://www.huya.com/123456)                                 | 可录制弹幕                                                               |
| 斗鱼         | 直播           | [`https://www.douyu.com/123456`](https://www.douyu.com/123456)                               | 可录制弹幕                                                               |
| YY语音       | 直播           | [`https://www.yy.com/123456`](https://www.yy.com/123456)                                     |                                                                          |
| 哔哩哔哩     | 直播           | [`https://live.bilibili.com/123456`](https://live.bilibili.com/123456)                       | 特殊分区hls流需要单独配置/可录制弹幕                                     |
| acfun        | 直播           | [`https://live.acfun.cn/live/123456`](https://live.acfun.cn/live/123456)                     |                                                                          |
| afreecaTV    | 直播           | [`https://play.afreecatv.com/biliup123/123456`](https://play.afreecatv.com/biliup123/123456) | 录制部分直播时需要登陆                                                   |
| bigo         | 直播           | [`https://www.bigo.tv/123456`](https://www.bigo.tv/123456)                                   |                                                                          |
| 抖音         | 直播           | 直播：[`https://live.douyin.com/123456`](https://live.douyin.com/123456)<br>直播：[`https://live.douyin.com/tiktok`](https://live.douyin.com/tiktok)<br>主页：[`https://www.douyin.com/user/456789`](https://www.douyin.com/user/456789) | 使用主页链接或被风控需配置cookies                                        |
| 快手         | 直播           | [`https://live.kuaishou.com/u/biliup123`](https://live.kuaishou.com/u/biliup123)             | 监控开播需使用中国大陆IPv4家宽，且24小时内单直播间最多120次请求          |
| 网易CC       | 直播           | [`https://cc.163.com/123456`](https://cc.163.com/123456)                                     |                                                                          |
| flextv       | 直播           | [`https://www.flextv.co.kr/channels/123456/live`](https://www.flextv.co.kr/channels/123456/live) |                                                                          |
| 映客         | 直播           | [`https://www.inke.cn/liveroom/index.html?uid=123456`](https://www.inke.cn/liveroom/index.html?uid=123456) |                                                                          |
| 猫耳FM       | 直播           | [`https://fm.missevan.com/live/123456`](https://fm.missevan.com/live/123456)                 | 猫耳为纯音频流                                                           |
| nico         | 直播           | [`https://live.nicovideo.jp/watch/lv123456`](https://live.nicovideo.jp/watch/lv123456)       | 可配置登录信息                                                           |
| twitch       | 直播<br>回放   | 直播：[`https://www.twitch.tv/biliup123`](https://www.twitch.tv/biliup123)<br>回放：[`https://www.twitch.tv/biliup123/videos?filter=archives&sort=time`](https://www.twitch.tv/biliup123/videos?filter=archives&sort=time) | 可配置登录信息/尽量录制回放/可录制弹幕                                   |
| youtube      | 直播<br>回放   | 直播：[`https://www.youtube.com/watch?v=biliup123`](https://www.youtube.com/watch?v=biliup123)<br>直播：[`https://www.youtube.com/@biliup123/live`](https://www.youtube.com/@biliup123/live)<br>回放：[`https://www.youtube.com/@biliup123/videos`](https://www.youtube.com/@biliup123/videos) | 可配置登录信息/尽量录制回放/可配置回放下载日期                           |
| 克拉克拉      |直播           | 直播: [`http://www.hongdoufm.com/room/123456`](http://www.hongdoufm.com/room/123456)<br>直播：[`https://live.kilakila.cn/PcLive/index/detail?id=123456`](https://live.kilakila.cn/PcLive/index/detail?id=123456) | hls/flv

* 理论上streamlink与yt-dlp支持的都可以下载，但不保证可以正常使用，详见:[streamlink支持列表](https://streamlink.github.io/plugins.html)，[yt-dlp支持列表](https://github.com/yt-dlp/yt-dlp/tree/master/yt_dlp/extractor).


## Credits
* Thanks `ykdl, youtube-dl, streamlink` provides downloader.
* Thanks `THMonster/danmaku`.


## 捐赠
* 爱发电 :`https://afdian.com/a/biliup`


## Stars
[![Star History Chart](https://api.star-history.com/svg?repos=biliup/biliup&type=Date)](https://star-history.com/#biliup/biliup&Date)
