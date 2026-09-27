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

biliup 是一个把直播录制、弹幕、切片和 B 站投稿串起来的工具：Rust 写的后端，自带 Web 界面。
配好主播和投稿模板后可以 24×7 无人值守，从开播检测、分段录制到稿件发布都不用人手参与。
单机一条命令 `biliup server` 就能跑；要多台机器一起录，控制面加 `--controller`，其余机器凭票据加入。

论坛：[BBS](https://bbs.biliup.rs)

## 🛠️ 功能

* **录制**：19 个直播平台内置解析，其余地址交给通用适配器；默认下载器 `mesio` 与可选的 `stream-gears` 都由 Rust 实现、编进 biliup，无外部依赖，也可选 `ffmpeg` / `streamlink` / `yt-dlp`；按时长或体积分段，同一主播下播后 10 分钟内（`live_merge_minutes`）再开播记在同一场
* **弹幕**：7 个平台录制的同时抓弹幕，输出 XML 弹幕文件
* **预览与回看**：录制中的直播间可在 Web 界面直接预览，复用正在写盘的那一路流（不另外拉流），带封面、实时弹幕与写盘码率曲线，支持多路同屏监视；一场直播一条时间轴，可拖动回看已落盘的部分，看直播时随手打标记
* **切片与发布**：回看页选段即剪，关键帧快速剪（不转码）或 FFmpeg 精确剪，可下载源格式或 MP4；切片进发布队列按投稿模板投 B 站，支持多选集中发布和取消
* **投稿**：多账号、多投稿模板；简介里的 `@credit` 生成真正的 @；可对已有稿件追加分 P；边录边传 0 落盘；上传池大小在 Web 里改完立即生效
* **多机（Fleet）**：一台控制面 + 多台节点，节点凭票据加入，只需出站连接、不需要公网 IP；房间分派到节点或按负载自动选；分层配置、节点标签、告警、控制面本机当节点（这几项在 master，未发版）
* **用户与权限**：`--auth` 下三种角色（超级管理员 / 操作员 / 只读观察者）、15 个权限点，`biliup user` 命令行管理
* **其他**：命令行可单独下载、投稿、管理合集，方便嵌进自动化流程；提供 skill，让你的 Agent 成为 up 主：`npx skills add biliup/biliup`；自动切片，让大模型从一场直播里找高光候选、人工确认后再切（master，未发版）；Windows 桌面安装包（Tauri，内置 FFmpeg）

### 功能与版本

从 PyPI / Release 安装得到的是最新发布版本。README 描述的是 master，下表列出各功能从哪个版本开始提供：

| 功能 | 最低版本 |
| --- | --- |
| 录制 / 弹幕 / 投稿 / 边录边传 / 直播预览 | v1.2.8 及更早 |
| 投稿：简介 `@credit` 生成真 @、上传池大小运行时可调 | v1.2.9 |
| 用户与权限（三角色） | v1.2.8（`clip.edit` 打标记 / 剪切片与 `node.manage` 节点管理两个权限点 v1.2.9 加入） |
| 切片工作台全套（场次、回看、标记、切片、发布队列、剪辑台导航） | v1.2.9 |
| Fleet：控制面、节点加入、心跳、房间分派、按负载自动选节点 | v1.2.9 |
| Fleet：分层配置、节点标签、界面告警、控制台汇总、本机当节点 | master（未发版） |
| 自动切片（A1–A4b） | master（未发版） |
| Windows 桌面安装包（新版，进程内运行 biliup、内置 FFmpeg） | 下个版本（v1.2.9 的构建失败，已由 [#1767](https://github.com/biliup/biliup/pull/1767) 修复；v1.2.8 附带的是旧版 `bbup-app` 安装包） |

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

## ⚠️ 免责声明

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

## 🚀 快速开始

### Windows
- **桌面安装包**：Release 中的 [biliup_*_x64-setup.exe](https://github.com/biliup/biliup/releases/latest)，双击安装，不需要另装 Python 或 FFmpeg。v1.2.9 的 Release 没有附带安装包，下个版本恢复；v1.2.9 用户请用下面的 uv 方式。
- **uv**：安装 [uv](https://docs.astral.sh/uv/getting-started/installation/) 后执行 `uv tool install biliup`，之后与 Linux / macOS 相同。

### Linux 或 macOS
1. 安装 [uv](https://docs.astral.sh/uv/getting-started/installation/) 
2. 安装：`uv tool install biliup`（也可以用 `pipx install biliup`）
3. 启动：`biliup server --auth`
4. 访问 WebUI：`http://127.0.0.1:19159`（默认只监听本机，远程访问见下方说明）
* 后台运行 
  1. `nohup biliup server --auth &`
  2. [请查看参考](https://biliup.github.io/biliup/docs/guide/introduction/#linuxxia-pei-zhi-kai-ji-zi-qi)
### Termux
- 详见[Wiki](https://github.com/biliup/biliup/wiki/Termux-%E4%B8%AD%E4%BD%BF%E7%94%A8-biliup)
- Release 附带 Android（aarch64）wheel `biliup-*-android_24_arm64_v8a.whl`

> [!NOTE]
> 默认下载器 `mesio`（rust-srec 引擎）和可选的 `stream-gears` 都由 Rust 实现、编进 biliup，无需外部依赖。若配置 `ffmpeg` 下载器或使用后处理，需要本机安装 `ffmpeg`；YouTube、niconico 等平台与通用适配器则依赖 `yt-dlp` 或 `streamlink`。Docker 镜像已内置 `ffmpeg`。

### 第一次使用

1. 打开 Web 界面，在「投稿管理」页右侧「新增账号」，用扫码登录添加 B 站账号，再新建一个投稿模板。
2. 在「直播管理」页新增直播间，填主播地址并选上一步的投稿模板。
3. 等主播开播。主播卡片会显示录制状态，录到的文件在剪辑台的「录制文件」里。

录像默认写在启动 biliup 的工作目录下。**请备份 `data/` 目录**：配置、主播与模板都在 `data/data.sqlite3`，网页扫码登录得到的 B 站凭据是 `data/<mid>.json`（命令行 `biliup login` 默认写到工作目录的 `cookies.json`）。

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

打开 `http://your-ip:19159` 设置管理员密码即可。录播与配置默认持久化在容器的 `/opt` 卷中（`docker-compose.yml` 默认把当前目录下的 `data/` 挂进去）。

> [!IMPORTANT]
> 若要自定义 `command`，必须带上 `--bind 0.0.0.0`。容器内若监听 `127.0.0.1`，宿主机的端口映射无法转发进容器，Web UI 将完全无法访问。

## 🎬 预览、回看与切片

biliup 把一场直播记成一个「场次」：断流重连、分段都挂在同一条时间轴上。

- **看直播、打标记**：录制中点主播卡片进直播预览页（`/live`），看到的就是正在写盘的那一路流；看到精彩处按标记，或直接「剪下刚才 30 秒 / 1 分钟 / 2 分钟」。多路同屏在剪辑台的「实时监视」里。
- **回看**：侧栏「剪辑台」（`/workbench`）的「直播场次」每场一行，点进去是录像回看页（`/replay`）。直播中也能回看已落盘的部分，标记会显示在时间轴上。
- **剪**：在回看页选入点、出点。快速剪在进程内按关键帧切、不转码，入点取不晚于它的最近关键帧，产物与源格式相同（FLV / TS / fMP4）；精确剪交给 FFmpeg 逐帧裁剪并重新编码成 MP4。切片可下载源格式或 MP4（由 FFmpeg 转封装）。**没装 FFmpeg 时只能快速剪、不能下载 MP4。**
- **发布**：切片卡片点「发布」按投稿模板投 B 站，也可以多选后集中发布。发布队列单并发，B 站返回 601（上传太频繁）时整个队列暂停，等你点「继续」。切片一律按转载投稿。
- **素材保留**：被标记、被切片引用的分段，后处理 `rm`、过滤删除和边录边传投稿后的删除都会推迟到引用释放后再删。在「空间配置」里可设「投稿后保留录像」（`retention_hours`，默认 0 即立即删）与「磁盘最低可用空间」（`min_free_space`，默认不启用）；可用空间低于阈值时，先删没被引用的最旧分段，仍不够再删被引用的，正在录的不删。

## 🖧 多机录制 Fleet

一台机器当控制面，其他机器作为节点加入；房间在控制面统一分派，节点各自录制和投稿。不用 Fleet 的单机用户升级后行为不变（不监听 19160）；各项对应的版本见上方「功能与版本」表。

1. **控制面**：`biliup server --controller`。需要一个节点能连到的 **TCP 端口，默认 19160**（内嵌 relay，`--relay-listen` 可改）；控制面网卡上没有公网地址时，用 `--relay-url` 声明外部地址（域名、端口转发、反向代理）。UDP 不是必需的。
2. **节点**：在控制面侧栏「节点」页点「添加节点」生成票据，在节点机上执行 `biliup node join <票据>`，然后重启 `biliup server`。Docker 部署的节点改设环境变量 `BILIUP_JOIN_TICKET=<票据>`。票据里带着控制面的 relay 地址（默认是控制面各网卡的地址，`--relay-url` 可改），节点只需出站连接，传输自带加密，不用配证书。
3. **分派**：在「节点 › 房间」把房间分给指定节点，或让控制面按负载自动选。改派时新节点等原节点确认释放后再接手，避免两台同时录（「强制迁移」除外）。以下在 master、未发版：节点可以打标签、房间可以要求标签；控制面统一下发配置（Cookie、密码等密钥留在节点本地）；「告警」显示节点离线、磁盘不足、投稿失败等；控制面自己也可以启用「本机」当节点。
4. **数据与备份**：控制面数据在独立的 `data/fleet.sqlite3`，**里面有控制面私钥，务必备份**，丢了所有节点都要重新加入。节点的凭据在 `data/node.json`（权限 0600）。已加入的节点不能再用 `--config` 启动，要回到单机先执行 `biliup node leave`。

## 🤖 自动切片（master，未发版）

下播后（过了断流合并窗口）按场次自动跑一遍：FFmpeg 抽音频、跳过静音 → 送语音转写 → 连同弹幕密度（模型能看图时再加关键帧缩图）交给大模型挑出候选区间 → 候选出现在回看页，**人工接受后才建切片草稿，不会自动导出或投稿**。

- **配置**：「空间配置」里的「自动切片（实验）」区块，对应全局配置 `[auto_clip]`。只支持 OpenAI 兼容接口（`/chat/completions` 与 `/audio/transcriptions`），想用本地模型就把地址指向自建服务。chat 填 `base_url`、`api_key`、`chat_model`（`api_key` 也可用环境变量 `BILIUP_AUTO_CLIP_API_KEY` 提供，界面只回显掩码）；转写填 `asr_base_url`、`asr_api_key`（留空沿用 chat 的）和 `asr_model`，可选 `asr_language`、`asr_prompt`（热词）。
- 填好后先点「测试连接」，再打开 `enabled` 总开关；首次开启会提示音频、文字和截图将发给你填的接口。
- **触发**：在主播编辑里打开「下播后自动生成候选」（`auto_clip_after_live`，默认关，改它需要超级管理员）；或在剪辑台「直播场次」里对某一场手动点「生成候选」，会先给出用量预估。
- **上限**：每场默认最多转写 300 分钟（`max_asr_minutes`）、chat 30 万 token（`max_chat_tokens`）。预估只是估算，费用以服务商账单为准。候选 72 小时没处理会过期，并释放占住的素材。
- **不配置时零行为变化**：没有 `[auto_clip]` 或 `enabled = false` 时什么都不跑，不调用任何 API。

## 📤 边录边传（sync-downloader）

把 `downloader` 设为 `sync-downloader` 后，ffmpeg 会把直播流 remux 成 Matroska 写到 stdout，按 UPOS 分片一边录一边上传，每录满 `file_size`（默认 2.5 GB，向上对齐到 10 MiB）就作为一 P 追加到同一稿件。前提：

- 本机 `PATH` 中有 `ffmpeg`；HLS 流（B 站 `bili_protocol = "hls_fmp4"`）若装有 `streamlink` 会用它拉流再交给 ffmpeg，没有则由 ffmpeg 直拉。
- 主播必须绑定上传模板，且模板对应的 cookies 文件可用；`uploader = "Noop"` 或没有模板时不会录制，日志会给出 `边录边传需要先为主播设定上传模板`。
- 上传并发固定 3 线程，不受 `threads`、`segment_time` 控制。

```toml
downloader = "sync-downloader"
uploader = "bili_web"
file_size = 2621440000          # 每 P 大小，可按上传带宽调小
# sync_save_dir = "/data/sync"  # 可选：额外保留每 P 的本地副本

[streamers."某主播"]
url = ["https://live.bilibili.com/1234"]
title = "{streamer}%Y-%m-%d 直播录像"
tid = 171
user_cookie = "cookies.json"
```

行为说明：

- 录制期间每 P 会同时写入系统临时目录（`$TMPDIR/biliup-sync/worker-<id>/`）作为兜底；预传完整且校验一致时直接 complete，否则（直播提前结束、上传落后）按实际长度从临时文件重传。投稿确认后临时文件删除，设置了 `sync_save_dir` 的副本保留并交给 `postprocessor`。
- 停止/暂停/编辑主播只会结束当前分段，已录内容仍会上传并投稿；上传或投稿失败时保留有内容的分段文件并在日志中打印路径，可手动补传。
- B 站直链约 1 小时过期。分段结束后拉不到数据时会立刻交回监控循环重新解析直链，并在同一稿件上继续追加分 P。
- 排查问题时留意 `ERROR ... 下载流程出错` 日志：会带上具体原因（cookies 文件路径、preupload 拒绝等），而不是静默退回落盘录制。

## 🔐 用户与权限

启动时加 `--auth` 后，首次访问会引导设置管理员密码（用户名固定为 `biliup`），这个账号是超级管理员。之后在侧栏「用户管理」里添加其他用户，角色三选一：

- **超级管理员**：全部 15 个权限点。
- **操作员**：可以查看与预览、暂停 / 恢复录制、增删改直播间、改投稿模板、投稿与发布切片、打标记与剪切片、看日志和文件；不能改空间配置、配置处理器钩子、管理 B 站账号、管理用户与 Fleet 节点。
- **只读观察者**：只能看直播间、预览、配置（敏感字段脱敏）、日志和文件。

忘记密码或需要排查时，在 biliup 服务的工作目录下执行 `biliup user list` / `biliup user reset-password <用户名>`，它们直接读写 `data/data.sqlite3`。

## 📜 命令行

B 站命令行投稿工具，支持**短信登录**、**账号密码登录**、**扫码登录**、**浏览器登录**以及**网页Cookie登录**，并将登录后返回的 cookie 和 token 保存在 `cookies.json` 中，可用于其他项目。

- 下载 Release: [biliupR](https://github.com/biliup/biliup/releases/latest)
- 获取命令帮助 `biliup --help`
- 登录信息文件可用 `-u/--user-cookie` 指定，便于多账号切换

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
  season    管理自己的合集：列合集、查小节、加入 / 移出稿件、排序
  user      管理 Web 界面的登录用户（在 biliup 服务的工作目录下执行，直接读写 data/data.sqlite3）
  node      把这台机器加入 Fleet 控制面或退出（在 biliup 服务的工作目录下执行，读写 data/node.json）
  list      列出所有已上传的视频
  help      Print this message or the help of the given subcommand(s)

Options:
  -p, --proxy <PROXY>              配置代理
  -u, --user-cookie <USER_COOKIE>  登录信息文件 [default: cookies.json]
      --rust-log <RUST_LOG>        日志过滤规则，如 debug；不指定时读取环境变量 RUST_LOG，都没有则为 tower_http=debug,info
  -h, --help                       Print help
  -V, --version                    Print version
```
启动录制服务
```shell
启动web服务，默认端口19159

Usage: biliup server [OPTIONS]

Options:
  -b, --bind <BIND>              Specify bind address [default: 127.0.0.1]
  -p, --port <PORT>              Port to use [default: 19159]
      --auth                     开启登录密码认证
      --secure-session-cookie    为会话 Cookie 附加 Secure 属性。仅当通过 HTTPS 反向代理访问 Web UI 时开启； 直接通过 HTTP 远程访问时开启会导致浏览器丢弃登录态
  -c, --config <FILE>            使用 biliup 1.0.7 风格配置文件启动录制
      --controller               以 Fleet 控制面运行：接受其他 biliup 节点加入并显示它们的状态（数据在 data/fleet.sqlite3）
      --relay-listen <ADDR|off>  控制面内嵌 relay 的 TCP 监听地址（默认 0.0.0.0:19160），节点必须能连到它； off 表示不起内嵌 relay，此时必须用 --relay-url 指定外部 relay。只在 --controller 时生效
      --relay-url <URL>          写进加入票据的 relay 地址，可重复；不给时自动列出本机网卡地址。 用于域名、端口转发、反向代理或外部 relay。只在 --controller 时生效
  -h, --help                     Print help
```

> [!IMPORTANT]
> 自 [#1660](https://github.com/biliup/biliup/pull/1660) 起，`--bind` 的默认值由 `0.0.0.0` 改为 `127.0.0.1`，即**默认只监听本机**，局域网/公网无法直接访问。
> 如需从其他设备访问，请加上 `--bind 0.0.0.0 --auth`，详见上方「🔓 远程访问」一节。Docker 镜像已内置该参数，不受影响。

Fleet 节点管理
```shell
把这台机器加入 Fleet 控制面或退出（在 biliup 服务的工作目录下执行，读写 data/node.json）

Usage: biliup node <COMMAND>

Commands:
  join    用控制面「添加节点」给出的票据加入；成功后重启 biliup server 生效
  leave   通知控制面移除本节点，并删除本地凭据 data/node.json
  status  查看本机的节点凭据（不连控制面）
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

单独下载一场直播/视频，无需启动服务：

```shell
biliup download <URL> -o "./video/%Y-%m-%dT%H_%M_%S{title}" --split-time 1h
```

管理自己的合集（需要先 `biliup login`，所有子命令都支持 `--json`）：

```shell
biliup season list                                # 列出合集和小节 ID
biliup season sections <合集ID>                   # 查看各小节里的稿件及其 episode ID
biliup season add <小节ID> --vid BV1xx411c7mD     # 加入稿件，cid 和标题自动获取；--vid 可重复
biliup season remove <episode ID>                 # 移出稿件
biliup season sort <小节ID> <episode ID>...       # 列出的稿件按顺序排到最前，其余保持原顺序；或用 --reverse 整体倒序
```

`--split-size` 与 `--split-time` 可按体积或时长自动分段，`-o` 支持 `{title}` 占位符与 strftime 时间格式。

## 📚 文档与更新日志

- [使用文档 »](https://biliup.github.io/biliup/docs/guide/introduction/)
- [命令行文档 »](https://biliup.github.io/biliup-rs)
- [更新日志 »](https://biliup.github.io/biliup/docs/guide/changelog)

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
        CLI[命令行与 Web API<br/>biliup-cli<br/>REST API / WebUI / 录制调度 / 切片]
        FLEET[多机 Fleet<br/>biliup-cli fleet<br/>控制面 / 节点 / iroh QUIC + 内嵌 relay]
        AUTOCLIP[自动切片<br/>biliup-cli auto_clip<br/>OpenAI 兼容客户端]
        CORE[核心库<br/>biliup<br/>直播解析 / 下载 / 上传]
        DANMAKU[弹幕库<br/>danmaku<br/>多平台协议 / XML输出]
        GEARS[Python绑定<br/>stream-gears<br/>python -m biliup 入口]
    end

    subgraph "🐍 Python包"
        PYENTRY[最小入口<br/>biliup.__main__<br/>调用 stream_gears.main_loop]
        PYUPLOAD[投稿库<br/>bili_webup / bili_webup_sync<br/>供外部项目调用]
    end

    subgraph "🗄️ 数据层"
        DB[(主库 data.sqlite3<br/>配置 / 主播 / 场次与切片<br/>任务状态 & 日志)]
        FLEETDB[(fleet.sqlite3<br/>控制面：节点 / 房间 / 私钥)]
        FILES[文件系统<br/>视频分段 / 弹幕XML<br/>关键帧索引 / 切片]
    end

    subgraph "🌍 外部服务"
        BILI[Bilibili API<br/>视频上传服务]
        STREAMS[直播平台<br/>B站/斗鱼/虎牙/抖音/Twitch等]
        LLM[OpenAI 兼容 API<br/>可选]
    end

    UI --> CLI
    CLI --> CORE & DANMAKU & FLEET & AUTOCLIP
    CLI --> DB & FILES
    FLEET --> FLEETDB
    AUTOCLIP --> LLM
    CORE --> STREAMS & BILI
    DANMAKU --> STREAMS & FILES
    GEARS --> CLI
    PYENTRY --> GEARS
    PYUPLOAD --> BILI

    classDef ui fill:#e1f5fe
    classDef rust fill:#f3e5f5
    classDef py fill:#e8f5e8
    classDef data fill:#fff3e0
    classDef ext fill:#ffebee
    class UI ui
    class CLI,FLEET,AUTOCLIP,CORE,DANMAKU,GEARS rust
    class PYENTRY,PYUPLOAD py
    class DB,FLEETDB,FILES data
    class BILI,STREAMS,LLM ext
```

### 目录结构

| 路径 | 说明 |
| --- | --- |
| `crates/biliup` | 核心库：直播解析、下载器、B 站投稿与凭据管理 |
| `crates/biliup-cli` | 命令行与 Web 服务：REST API、WebUI 托管、录制调度 |
| `crates/biliup-cli/src/server/{api,fleet,auto_clip}` | 分别是 Web API 路由、Fleet 控制面与节点、自动切片 |
| `crates/biliup-cli/migrations` | 主库 `data.sqlite3` 迁移（1–14） |
| `crates/biliup-cli/fleet_migrations` | Fleet 库 `fleet.sqlite3` 迁移（1–4） |
| `crates/danmaku` | 弹幕客户端：多平台协议解析与 XML 输出 |
| `crates/stream-gears` | PyO3 绑定，暴露给 `python -m biliup` |
| `app`、`public` | Next.js WebUI 源码；`npm run build` 产物输出到 `out/` 并由后端内嵌 |
| `crates/stream-gears/biliup` | 精简 Python 包：最小入口与可供外部调用的投稿库 |
| `tauri-app` | Windows 桌面端（Tauri，进程内运行 biliup 服务） |

</details>

### frontend

1. 确保 Node.js 版本 ≥ 20.9（Next.js 16 要求）；前端为 Next.js 16 / React 19 / TypeScript 6 / Semi UI
2. 安装依赖：`npm i`
3. 启动开发服务器：`npm run dev`
4. 访问：`http://localhost:3000`

### Python

1. 安装依赖 `maturin develop -m crates/stream-gears/Cargo.toml`
2. `npm run build` 
3. 启动 Biliup：`python3 -m biliup`

### Rust-cli

1. 准备 Rust ≥ 1.95（依赖 `mesio-engine` 的要求，CI 使用 stable），然后 `npm run build`
2. 构建 `cargo build --release --bin biliup`
3. 开发启动 BiliupR：`cargo run`

> [!NOTE]
> `biliup-cli` 通过 `rust-embed` 内嵌前端产物目录 `out/`，因此在 `cargo build` / `cargo test` 之前必须先执行 `npm run build`，否则编译会因 `out/` 不存在而失败。

### 测试

```shell
cargo test -p biliup -p biliup-cli -p danmaku
```

- 自动切片抽音频、取帧等用例会真的调用 `ffmpeg`，本机没装时这些用例会跳过；CI 的 Rust tests 已安装 ffmpeg。
- clippy 只对改动的 crate 跑：`cargo clippy -p <crate> --all-targets -- --no-deps`。用 `--workspace` 可能长时间不结束。
- 桌面端 `tauri-app/src-tauri` 是独立的 Cargo workspace，有自己的 `Cargo.lock`。改了 `biliup-cli` 的依赖（或根 `Cargo.lock` 变了）必须同步它，并用 `cargo metadata --locked --manifest-path tauri-app/src-tauri/Cargo.toml` 验证，CI 有同样的检查。

### 数据库迁移

主库 `data/data.sqlite3` 与 Fleet 库 `data/fleet.sqlite3` 各有一套迁移、独立编号，启动时自动执行（Fleet 库只在 `--controller` 时打开）。迁移不能回滚，升级前请备份 `data/`。

## 🤝Credits
* Thanks `ykdl, youtube-dl, streamlink` provides downloader.
* Thanks `THMonster/danmaku`.


## 💴捐赠
<img src=".github/resource/Image.jpg" width="200" />

[爱发电 »](https://afdian.com/a/biliup)

## ⭐Stars
[![Star History Chart](https://star-history.dera.page/svg?repos=biliup/biliup&type=Date)](https://star-history.dera.page/#biliup/biliup&Date)
