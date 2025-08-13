<div align="center">
  <img src="https://docs.biliup.rs/home.png" alt="description" width="300" height="300"/>
</div>

<div align="center">

[![Python](https://img.shields.io/badge/python-3.9%2B-blue)](http://www.python.org/download)
[![PyPI](https://img.shields.io/pypi/v/biliup)](https://pypi.org/project/biliup)
[![PyPI - Downloads](https://img.shields.io/pypi/dm/biliup)](https://pypi.org/project/biliup)
[![License](https://img.shields.io/github/license/biliup/biliup)](https://github.com/biliup/biliup/blob/master/LICENSE)
[![Telegram](https://img.shields.io/badge/Telegram-Group-blue.svg?logo=telegram)](https://t.me/+IkpIABHqy6U0ZTQ5)

[![GitHub Issues](https://img.shields.io/github/issues/biliup/biliup?label=Issues)](https://github.com/biliup/biliup/issues)
[![GitHub Stars](https://img.shields.io/github/stars/biliup/biliup)](https://github.com/biliup/biliup/stargazers)
[![GitHub Forks](https://img.shields.io/github/forks/biliup/biliup)](https://github.com/biliup/biliup/network)

</div>



## 🛠️ 功能
* 开箱即用，多种安装方式，提供可视化WebUi界面
* 多主播录制/上传，24X7无人值守运行，高自定义元信息
* 边录边传不落盘急速上传，节省本地硬盘空间

论坛：[BBS](https://bbs.biliup.rs)

## 📜 更新日志

- **[更新日志 »](https://biliup.github.io/biliup/docs/guide/changelog)**




## 📜 使用文档

- [使用文档 »](https://docs.biliup.rs)

## 🚀 快速开始

### Windows
- 下载 exe: [Release](https://github.com/biliup/biliup/releases/latest)

### Linux 或 macOS
1. 安装 [uv](https://docs.astral.sh/uv/getting-started/installation/) 
2. 安装：`uv tool install biliup`
3. 启动：`biliup start`
4. 访问 WebUI：`http://your-ip:19159`

### Termux
- 详见[Wiki](https://github.com/biliup/biliup/wiki/Termux-%E4%B8%AD%E4%BD%BF%E7%94%A8-biliup)


---

## 🧑‍💻开发

### frontend

1. 确保 Node.js 版本 ≥ 18
2. 安装依赖：`npm i`
3. 启动开发服务器：`npm run dev`
4. 访问：`http://localhost:3000`

### backend

1. 安装依赖 `maturin dev`
2. `npm run build` 确保 `/biliup/web/public` 目录存在构建好的前端静态资源
3. 启动 Biliup：`python3 -m biliup`

## 🤝Credits
* Thanks `ykdl, youtube-dl, streamlink` provides downloader.
* Thanks `THMonster/danmaku`.


## 💴捐赠
[爱发电 »](https://afdian.com/a/biliup)


## ⭐Stars
[![Star History Chart](https://api.star-history.com/svg?repos=biliup/biliup&type=Date)](https://star-history.com/#biliup/biliup&Date)
