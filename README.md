<div align="center">
  <img src="https://raw.githubusercontent.com/biliup/biliup/master/public/logo.png" alt="biliup" width="300" height="300"/>
</div>

<div align="center">

[![Python](https://img.shields.io/badge/python-3.9%2B-blue)](https://www.python.org/downloads/)
[![PyPI](https://img.shields.io/pypi/v/biliup)](https://pypi.org/project/biliup)
[![PyPI - Downloads](https://img.shields.io/pypi/dm/biliup)](https://pypi.org/project/biliup)
[![License](https://img.shields.io/github/license/biliup/biliup)](https://github.com/biliup/biliup/blob/master/LICENSE)
[![Telegram](https://img.shields.io/badge/Telegram-Group-blue.svg?logo=telegram)](https://t.me/+IkpIABHqy6U0ZTQ5)

[![GitHub Issues](https://img.shields.io/github/issues/biliup/biliup?label=Issues)](https://github.com/biliup/biliup/issues)
[![GitHub Stars](https://img.shields.io/github/stars/biliup/biliup)](https://github.com/biliup/biliup/stargazers)
[![GitHub Forks](https://img.shields.io/github/forks/biliup/biliup)](https://github.com/biliup/biliup/network)

</div>

## 🛠️ 功能
* 提供 skill，让你的 Agent 成为 up 主: `npx skills add biliup/biliup`
* 开箱即用，多种安装方式，提供可视化 WebUi 界面
* 多主播录制/上传，24X7 无人值守运行，高自定义元信息
* 录制同时抓取弹幕，输出 XML 弹幕文件
* 作为自动化流程中的命令行工具封装使用

论坛：[BBS](https://bbs.biliup.rs)

## 📺 支持平台

内置 19 个直播平台解析，未匹配的地址会交给通用适配器（依赖本机的 yt-dlp / Streamlink）尝试处理。

| 平台 | 站点 | 弹幕 |
| --- | --- | :-: |
| 哔哩哔哩 | `live.bilibili.com`、`b23.tv` | ✅ |
| 抖音 | `douyin.com` | ✅ |
| 斗鱼 | `douyu.com` | ✅ |
| 虎牙 | `huya.com` | ✅ |
| 快手 | `kuaishou.com`、`chenzhongtech.com` | |
| AcFun | `acfun.cn` | |
| 网易 CC | `cc.163.com` | |
| 映客 | `inke.cn` | |
| 猫耳 FM | `missevan.com` | |
| KilaKila / 红豆 FM | `live.kilakila.cn`、`hongdoufm.com` | |
| YY | `yy.com` | |
| TTingLive | `ttinglive.com` | |
| Twitch | `twitch.tv`（直播与录像） | ✅ |
| YouTube | `youtube.com`、`youtu.be` | ✅ |
| TwitCasting | `twitcasting.tv` | ✅ |
| niconico | `nicovideo.jp` | |
| AfreecaTV | `afreecatv.com` | |
| Bigo Live | `bigo.tv` | |
| Picarto | `picarto.tv` | |
| 通用适配器 | 其他 `http(s)` 地址 | |

## 📜 更新日志

> [!IMPORTANT]  
> **Disclaimer / 免责声明**
> - 本项目仅供个人学习研究，不保证稳定性，不提供技术支持
> - 使用本项目产生的一切后果由用户自行承担
> - 禁止商业用途，请遵守版权及平台规定
> - This project is for **personal learning and research purposes only**
> - No stability guarantee or technical support provided
> - Users are solely responsible for any consequences of using this project
> - Commercial use is strictly prohibited
> - Please respect copyright and platform ToS

- **[更新日志 »](https://biliup.github.io/biliup/docs/guide/changelog)**

## 📜 使用文档
B 站命令行投稿工具，支持**短信登录**、**账号密码登录**、**扫码登录**、**浏览器登录**以及**网页Cookie登录**，并将登录后返回的 cookie 和 token 保存在 `cookies.json` 中，可用于其他项目。

- 下载 Release: [biliupR](https://github.com/biliup/biliup/releases/latest)
- 获取命令帮助 `biliup --help`
- 登录信息文件可用 `-u/--user-cookie` 指定，便于多账号切换

**文档地址**：<https://biliup.github.io/biliup-rs>
```shell
Upload video to bilibili.

Usage: biliup [OPTIONS] <COMMAND>

Commands:
  login     登录B站并保存登录信息
  renew     手动验证并刷新登录信息
  upload    上传视频
  append    是否要对某稿件追加视频
  show      打印视频详情
  comments  查看视频评论
  reply     回复视频评论，默认只打印将要回复的内容
  dump-flv  输出flv元数据
  download  下载视频
  server    启动web服务，默认端口19159
  list      列出所有已上传的视频
  help      Print this message or the help of the given subcommand(s)

Options:
  -p, --proxy <PROXY>              配置代理
  -u, --user-cookie <USER_COOKIE>  登录信息文件 [default: cookies.json]
      --rust-log <RUST_LOG>        [default: tower_http=debug,info]
  -h, --help                       Print help
  -V, --version                    Print version
```
启动录制服务
```shell
启动web服务，默认端口19159

Usage: biliup server [OPTIONS]

Options:
  -b, --bind <BIND>            Specify bind address [default: 127.0.0.1]
  -p, --port <PORT>            Port to use [default: 19159]
      --auth                   开启登录密码认证
      --secure-session-cookie  为会话 Cookie 附加 Secure 属性。仅当通过 HTTPS 反向代理访问 Web UI 时开启； 直接通过 HTTP 远程访问时开启会导致浏览器丢弃登录态
  -c, --config <FILE>          使用 biliup 1.0.7 风格配置文件启动录制
  -h, --help                   Print help
```

> [!IMPORTANT]
> 自 [#1660](https://github.com/biliup/biliup/pull/1660) 起，`--bind` 的默认值由 `0.0.0.0` 改为 `127.0.0.1`，即**默认只监听本机**，局域网/公网无法直接访问。
> 如需从其他设备访问，请加上 `--bind 0.0.0.0 --auth`，详见下方「🔓 远程访问」一节。Docker 镜像已内置该参数，不受影响。

单独下载一场直播/视频，无需启动服务：

```shell
biliup download <URL> -o "./video/%Y-%m-%dT%H_%M_%S{title}" --split-time 1h
```

`--split-size` 与 `--split-time` 可按体积或时长自动分段，`-o` 支持 `{title}` 占位符与 strftime 时间格式。

- [使用文档 »](https://biliup.github.io/biliup/docs/guide/introduction/)

## 🚀 快速开始

### Windows
- 下载 Release: [bbup-app](https://github.com/biliup/biliup/releases/latest)

### Linux 或 macOS
1. 安装 [uv](https://docs.astral.sh/uv/getting-started/installation/) 
2. 安装：`uv tool install biliup`
3. 启动：`biliup server --auth`
4. 访问 WebUI：`http://127.0.0.1:19159`（默认只监听本机，远程访问见下方说明）
* 后台运行 
  1. `nohup biliup server --auth &`
  2. [请查看参考](https://biliup.github.io/biliup/docs/guide/introduction/#linuxxia-pei-zhi-kai-ji-zi-qi)
### Termux
- 详见[Wiki](https://github.com/biliup/biliup/wiki/Termux-%E4%B8%AD%E4%BD%BF%E7%94%A8-biliup)

> [!NOTE]
> 默认下载器 `stream-gears` 由 Rust 实现，无需外部依赖。若配置 `ffmpeg` 下载器或使用后处理，需要本机安装 `ffmpeg`；YouTube、niconico 等平台与通用适配器则依赖 `yt-dlp` 或 `streamlink`。Docker 镜像已内置 `ffmpeg`。

### 🔓 远程访问（监听 0.0.0.0）

默认的 `127.0.0.1` 只允许本机访问。需要从其他设备访问时，显式指定 `--bind 0.0.0.0` 并开启 `--auth`：

```shell
biliup server --bind 0.0.0.0 --auth
```

首次打开 `http://your-ip:19159` 会引导设置管理员密码（用户名固定为 `biliup`），之后即可正常登录使用。

> [!NOTE]
> 绑定非回环地址时必须同时开启 `--auth`，否则会拒绝启动，避免无认证的 Web API 被暴露：
>
> ```
> refusing to expose the unauthenticated Web API on 0.0.0.0:19159; use a loopback bind address or enable --auth
> ```

> [!WARNING]
> 将 Web UI 暴露到公网存在风险，建议仅在可信局域网内使用，或置于反向代理之后。
> 首次访问即可设置管理员密码，请在启动后立即完成初始化，避免被他人抢先占用。
> 若通过 **HTTPS** 反向代理访问，请加上 `--secure-session-cookie`；直接以 HTTP 远程访问时**不要**加，否则浏览器会丢弃登录态（见 [#1669](https://github.com/biliup/biliup/pull/1669)）。

#### Docker

镜像已内置 `--bind 0.0.0.0 --auth`，开箱即用，无需额外配置：

```shell
docker compose up -d
```

打开 `http://your-ip:19159` 设置管理员密码即可。录播与配置默认持久化在容器的 `/opt` 卷中。

> [!IMPORTANT]
> 若要自定义 `command`，必须带上 `--bind 0.0.0.0`。容器内若监听 `127.0.0.1`，宿主机的端口映射无法转发进容器，Web UI 将完全无法访问。

---

## 🧑‍💻开发

<details>

### 架构概览

Rust后端 + 精简 Python 包 + Next.js前端的混合架构。

```mermaid
graph TB
    subgraph "🌐 前端层"
        UI[Next.js Web界面<br/>React + TypeScript<br/>Semi UI组件库]
    end

    subgraph "⚡ Rust后端服务"
        CLI[命令行与 Web API<br/>biliup-cli<br/>REST API / WebUI / 配置导入]
        CORE[核心库<br/>biliup<br/>直播解析 / 下载 / 上传]
        DANMAKU[弹幕库<br/>danmaku<br/>多平台协议 / XML输出]
        GEARS[Python绑定<br/>stream-gears<br/>python -m biliup 入口]
    end

    subgraph "🐍 Python包"
        PYENTRY[最小入口<br/>biliup.__main__<br/>调用 stream_gears.main_loop]
        PYUPLOAD[投稿库<br/>bili_webup / bili_webup_sync<br/>供外部项目调用]
    end

    subgraph "🗄️ 数据层"
        DB[(SQLite数据库<br/>配置存储<br/>任务状态 & 日志)]
        FILES[文件系统<br/>视频分段 / 弹幕XML<br/>缓存与临时文件]
    end

    subgraph "🌍 外部服务"
        BILI[Bilibili API<br/>视频上传服务]
        STREAMS[直播平台<br/>B站/斗鱼/虎牙/抖音/Twitch等]
    end

    UI --> CLI
    CLI --> CORE
    CLI --> DANMAKU
    CLI --> DB
    CLI --> FILES
    CORE --> STREAMS
    CORE --> BILI
    DANMAKU --> STREAMS
    DANMAKU --> FILES
    GEARS --> CLI
    PYENTRY --> GEARS
    PYUPLOAD --> BILI

    style UI fill:#e1f5fe
    style CLI fill:#f3e5f5
    style CORE fill:#f3e5f5
    style DANMAKU fill:#f3e5f5
    style GEARS fill:#f3e5f5
    style PYENTRY fill:#e8f5e8
    style PYUPLOAD fill:#e8f5e8
    style DB fill:#fff3e0
    style FILES fill:#fff3e0
    style BILI fill:#ffebee
    style STREAMS fill:#ffebee
```

### 目录结构

| 路径 | 说明 |
| --- | --- |
| `crates/biliup` | 核心库：直播解析、下载器、B 站投稿与凭据管理 |
| `crates/biliup-cli` | 命令行与 Web 服务：REST API、WebUI 托管、录制调度 |
| `crates/danmaku` | 弹幕客户端：多平台协议解析与 XML 输出 |
| `crates/stream-gears` | PyO3 绑定，暴露给 `python -m biliup` |
| `app`、`public` | Next.js WebUI 源码；`npm run build` 产物输出到 `out/` 并由后端内嵌 |
| `biliup` | 精简 Python 包：最小入口与可供外部调用的投稿库 |
| `tauri-app` | 桌面端外壳（实验性） |

</details>

### frontend

1. 确保 Node.js 版本 ≥ 18.17（Next.js 14 要求）
2. 安装依赖：`npm i`
3. 启动开发服务器：`npm run dev`
4. 访问：`http://localhost:3000`

### Python

1. 安装依赖 `maturin dev`
2. `npm run build` 
3. 启动 Biliup：`python3 -m biliup`

### Rust-cli

1. `npm run build`
2. 构建 `cargo build --release --bin biliup`
3. 开发启动 BiliupR：`cargo run`

> [!NOTE]
> `biliup-cli` 通过 `rust-embed` 内嵌前端产物目录 `out/`，因此在 `cargo build` / `cargo test` 之前必须先执行 `npm run build`，否则编译会因 `out/` 不存在而失败。

### 测试

```shell
cargo test -p biliup -p biliup-cli -p danmaku
```

## 🤝Credits
* Thanks `ykdl, youtube-dl, streamlink` provides downloader.
* Thanks `THMonster/danmaku`.


## 💴捐赠
<img src=".github/resource/Image.jpg" width="200" />

[爱发电 »](https://afdian.com/a/biliup)

## ⭐Stars
[![Star History Chart](https://star-history.dera.page/svg?repos=biliup/biliup&type=Date)](https://star-history.dera.page/#biliup/biliup&Date)
