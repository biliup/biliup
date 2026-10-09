+++
title = "斗鱼登录、Cookie 续期与画质设置"
description = "配置斗鱼网页版登录凭据和自动续期，检查房间登录来源与实际录制画质"
date = 2026-10-07T00:00:00+00:00
updated = 2026-10-09T00:00:00+00:00
draft = false
weight = 100
sort_by = "weight"
template = "docs/page.html"
[extra]
lead = "配置斗鱼 Web Cookie 与 passport 长期凭据，让 biliup 自动续期登录，并核对房间使用的账号和实际录制画质。"
toc = true
top = false
+++

biliup 使用斗鱼网页版的签名和播放接口取流。将 `douyu_rate` 设为 `0` 表示请求最高画质，实际档位由平台返回；部分房间会将匿名请求降到较低档位。配置有效的网页版登录 Cookie 后，可获取账号有权观看的画质。

“原画”是平台的档位名称，不能统一等同于 1080P、60fps 或固定码率。不同房间的分辨率、帧率和档位编号可能不同。Cookie 也不能保证连接永不中断。

## 获取并保存登录凭据

在本机 GUI 的 **空间配置 → 平台设置 → 斗鱼** 中管理登录。只有 Web Cookie 时可以登录取流；同时提供 `passport.douyu.com` 的长期凭据 `LTP0` 和配套 `dy_did`，才能自动更新 `acf_auth` 等登录 Cookie。三个部分必须来自**同一账号、同一次浏览器登录**。

### Web Cookie

1. 在浏览器打开 [斗鱼](https://www.douyu.com/) 并登录。
2. 打开开发者工具的 **Network（网络）** 面板，刷新直播间。
3. 选择发送到 `www.douyu.com` 的请求，在 **Request Headers（请求头）** 中复制 `Cookie` 的值。
4. 将内容粘贴到 biliup 的 **登录 Cookie（douyu_cookie）**，保存配置。
5. 点击 **测试 Cookie** 检查账号登录状态。

示例格式（下面的值仅为占位）：

```
acf_uid=123456; acf_auth=your_auth_value; acf_did=your_device_id; ...
```

也支持 [EditThisCookie (V3)](https://chromewebstore.google.com/detail/editthiscookie-v3/ojfebgpkimhlhcblbalbfjblapadhbol?pli=1) 等浏览器插件导出的 JSON 数组，可直接粘贴：

```json
[
  { "domain": ".douyu.com", "name": "acf_uid", "value": "123456" },
  { "domain": ".douyu.com", "name": "acf_auth", "value": "your_auth_value" },
  { "domain": ".douyu.com", "name": "acf_did", "value": "your_device_id" }
]
```

请粘贴 JSON **内容**，这个字段不是文件路径。程序识别 `.douyu.com`、`douyu.com` 和 `www.douyu.com` 的 Web Cookie；导出中的 `passport.douyu.com` 条目用于提取续期凭据，其他网站的条目会被忽略。普通 Web API 请求不会携带 `LTP0`。

`acf_uid` 和 `acf_auth` 需要非空。复制完整来源，不要手动解码 `%xx` 或修改值中的 `=`。格式错误、冲突的同名字段和非法请求头字符会被拒绝；不要合并不同账号的导出。

### 长期凭据与自动续期

在同一次登录的浏览器会话中打开斗鱼 passport 登录页面，通过开发者工具的 **Application（应用）→ Storage（存储）→ Cookies** 查看 `passport.douyu.com` 以及 `.douyu.com` 作用域，取得 `LTP0` 和配套 `dy_did`。不要将另一台设备、另一个账号或另一次登录的设备号与 `LTP0` 拼在一起。

可以选择一种导入方式：

- 将包含 Web Cookie 与 passport 凭据的完整浏览器 JSON 数组粘贴到 **登录 Cookie**，保留原有 `domain` 字段。程序会按作用域分离凭据；导出已包含 `LTP0` 和配套 `dy_did` 时，两个单独字段可以留空。
- 在 **登录 Cookie** 中保存 Web Cookie，并把 passport 的值分别填入 **长期续期凭据 LTP0** 和 **续期设备标识 dy_did**。

不要只复制 passport Cookie 代替 Web Cookie；passport 的长期票据与网页的 `acf_auth` 用途不同。浏览器中没有 `LTP0` 时，重新完成登录并检查；已有 Web Cookie 仍可用于登录取流，但无法自动续期。

| GUI 字段 / 配置键                               | 用途                                             |
| ----------------------------------------------- | ------------------------------------------------ |
| 登录 Cookie / `douyu_cookie`                    | Web 请求头字符串或浏览器 Cookie JSON 数组        |
| 长期续期凭据 LTP0 / `douyu_ltp0`                | passport 长期票据；完整 JSON 已包含时可留空      |
| 续期设备标识 dy_did / `douyu_refresh_device_id` | 与该票据配套的设备标识；完整 JSON 已包含时可留空 |
| 自动续期登录 Cookie / `douyu_auto_refresh`      | 默认开启，保存有效凭据对后自动调度               |
| 设备 ID / `douyu_deviceId`                      | 取流设备 ID，与续期设备字段分开；通常可留空      |

网页版取流设备 ID 留空时，优先使用 Web Cookie 中的 `dy_did`，其次是 `acf_did`，再使用默认值；App 回退使用 `acf_did` 或默认值。不要为了续期把 passport 设备号填到取流设备字段。

![浏览器中 Web Cookie 与 passport 长期凭据的作用域](/biliup/images/douyu-tutorial/passport-cookie-export.png)
![浏览器中 Web Cookie 与 passport 长期凭据的作用域](/biliup/images/douyu-tutorial/passport-cookie-export_2.png)

## 查看登录状态和手动续期

保存配置后，**本机已保存的斗鱼登录状态** 面板显示登录状态、账号、续期状态、上次校验、上次成功续期，以及下次续期或重试时间。时间按浏览器时区显示。

**测试 Cookie** 校验当前输入框的内容，既不保存修改，也不触发续期。它调用需要登录的 `https://www.douyu.com/wgapi/livenc/liveweb/follow/top3` 接口，并检查预期 JSON 登录结果。HTTP 200、HTML 页面或公开房间信息接口成功都不足以证明已登录。

**手动续期** 使用已经保存的配置。凭据或续期开关有未保存修改时，先保存再点击；自动续期关闭时仍可手动续期。同一来源最短 60 秒操作一次，正在续期和重复点击不会再启动一份交换。需要 `config.edit` 权限才能查看状态和操作这些按钮。

| 状态                        | 处理方式                                                   |
| --------------------------- | ---------------------------------------------------------- |
| 等待下次续期                | 已安排自动续期；可以查看下次时间                           |
| 正在续期                    | 等待状态更新，不要重复发起交换                             |
| 缺少续期凭据                | 补充同一次登录的 `LTP0` 与 `dy_did`，再保存                |
| 续期失败，等待重试          | 临时失败已安排退避；检查网络，避免连续点击                 |
| 续期凭据已失效 / 登录已失效 | 重新登录斗鱼，导出同一会话的 Web Cookie 与长期凭据，再保存 |
| 自动续期已关闭              | 按需开启；已有 Cookie 仍会做登录校验                       |

![本机斗鱼登录与自动续期配置](/biliup/images/douyu-tutorial/douyu-login-settings.png)

### 自动调度、持久化与重启

- 程序每分钟检查到期状态，成功后下一次续期为 **3 天后**。
- 临时网络、接口或协议错误从 **5 分钟**开始指数退避，最多 **6 小时**，保留已有 Cookie。明确凭据失效或账号不匹配时退避 **24 小时**，并提示重新登录；已确认失效的 Cookie 不再用于取流。
- 最新 Cookie、轮换后的长期凭据、状态和下次时间保存在本机 SQLite 的 `douyu_cookie_keeper` 表。重启恢复已有日程和结果，并安排登录校验。
- 来源级互斥和同一数据库的 **5 分钟租约**防止重复交换；交换最多 240 秒。异常退出后陈旧租约到期即可恢复。

输入框继续保留你导入的**来源凭据**；续期后的最新凭据由程序单独保存。来源值不变时，后续 Web API 请求会取得最新 Cookie，无需重启。已有 CDN 视频连接不会因为续期强制重连。

修改来源 Cookie、`LTP0` 或续期设备号会建立新的来源绑定，旧账号的续期结果不会套用到新配置。交换成功但写盘失败时，程序保留内存结果并重试保存；落库前重启仍可能丢失该结果，应检查状态中的重试提示。

## 房间继承与独立账号

进入 **直播管理 → 对应房间 → 配置覆写 → 斗鱼**，选择该房间的登录来源：

- **继承空间配置**：使用本机全局 Cookie 和续期结果，无需重复填写；房间 Cookie 输入框留空并不表示没有登录。
- **此房间独立 Cookie**：使用这里填写的 Cookie。独立来源不借用全局账号的长期凭据；要自动续期，需要提供这个来源自己的完整凭据导出。选择独立但未填 Cookie，保存后仍继承全局配置。
- **此房间匿名取流**：明确不使用全局登录信息，原画可能受平台限制。

房间面板展示**已保存配置实际使用的登录状态**，包含全局继承结果。房间表单上的“测试 Cookie”只校验当前独立输入，不代表另一个来源的状态。修改后保存，再查看状态面板确认使用的账号。

## 核对实际录制画质

登录状态有效，表示斗鱼接受了该账号的登录凭据；这并不保证每个直播间都提供原画或 HEVC。取流日志会报告请求档位和平台实际返回的档位，发生降档时会提示。遇到不符合预期的画质，请同时检查：

- Cookie 验证是否通过，同一账号在网页上是否能选择目标画质。
- `douyu_rate` 是否为目标档位。`0` 请求原画；常见编号还包括 `8`（蓝光 8M）、`4`（蓝光 4M）、`3`（超清）、`2`（高清），以具体房间为准。
- 实际文件的分辨率、帧率和编码。不能仅凭 URL 的 `1024h.flv` 等后缀或请求参数判断。
- 房间是否正在循环播放。biliup 目前将 `videoLoop=1` 的房间视为非实时直播并跳过录制。

新网页版接口出错时，biliup 可回退到 App 播放接口。网页版 Cookie 不等于 App 登录 token，回退取流的画质可能受限，日志会说明回退原因。

浏览器完整导出中的部分无关字段可能导致网页播放接口返回 403。当前实现为播放接口单独构造 Cookie Header，只携带需要的 `acf_*` 和 Web `dy_did`；完整来源仍用于保存和续期。因此无需手动删除导出中的其他字段。若仍看到 403 或 App 回退，结合日志、登录状态和媒体检查定位问题。

检查已经关闭的录制分段：

```bash
ffprobe -v error -select_streams v:0 \
  -show_entries stream=codec_name,width,height,avg_frame_rate \
  -of default=noprint_wrappers=1 '录制分段.flv'
```

例如 `width=3840`、`height=2160`、`avg_frame_rate=60/1` 才能确认该文件为 4K60。不同房间提供的媒体参数不同，这个例子不代表平台固定提供该档位。

录制分段、自定义时长、在直播截帧上圈选遮挡和快速合并文件，见[录制分段、画面遮挡与录像合并](@/docs/tutorials/recording-and-masking.md)。

## AVC 与 HEVC

默认编码为 AVC（H.264）。选择 HEVC（H.265）时，biliup 请求平台提供 HEVC，并使用返回的对应播放地址；房间未提供 HEVC 地址时会回退到 AVC 并提示。

HEVC 录制应使用 **mesio** 或支持实际 FLV 格式的新版 **FFmpeg**。**stream-gears** 的 FLV 解析器不支持 HEVC 和 AV1，会报错停止，以避免生成只有音频的文件。不同房间和 CDN 可能使用传统 codec id 12 或 Enhanced FLV，旧版 FFmpeg 不一定兼容。

## 开发者验证

协议、Cookie 和接口回退的离线回归测试不需要真实账号或网络：

```bash
cargo test --locked -p biliup downloader::live::douyu
```

仓库提供 `douyu_probe` 示例程序，调用实际插件取流，最多采样 8 MiB，再用本机的 `ffprobe` 检查编码、分辨率、帧率，并用 `ffmpeg` 解码两秒视频。请选当前正在直播的房间；循环播放或未开播会明确显示 `offline`，这不算成功验证了取流。

```bash
# 匿名 AVC
cargo run --locked -p biliup --example douyu_probe -- --room 288016 --codec AVC

# 使用本地 Cookie 文件验证 HEVC；文件可包含请求头字符串或浏览器 JSON 数组
cargo run --locked -p biliup --example douyu_probe -- --room 288016 --codec HEVC --cookie-file /path/to/douyu-cookie.json
```

探测输出包含实际媒体信息，不输出 Cookie、签名或完整播放 URL。不要将含凭据的文件提交到仓库。

线上探测在本机手动运行，受房间状态、地域、账号和 CDN 可达性影响。GitHub Actions 只执行不需要斗鱼账号的离线回归测试。

## 凭据保管与多节点部署

完整 Cookie、`LTP0` 和设备号都按登录凭据保管，不要放入截图、工单、日志、仓库或分享的配置。截图应遮住所有凭据值和账号 ID；图名和说明也不要包含真实账号信息。泄露后应退出相关登录会话并重新取得凭据。

凭据保存在本机 SQLite，**没有加密为保险库**。Unix 上数据库和 WAL/SHM 文件以 `0600` 权限打开，配置导出、备份和部署目录仍需自行保护。账号管理遵循项目现有权限及 `--auth` 模式。

Fleet / HA 不下发本机长期凭据，房间配置传输时也会剥离内嵌 passport 凭据。每个节点在本机管理自己的 `LTP0` 与 `dy_did`；不同数据库之间没有续期互斥，不要把同一张长期票据复制到多个节点同时续期。应分别登录取得各节点使用的凭据。远程 Fleet 配置页不会操作控制面的本机账号。

状态和手动续期接口只返回安全状态，不返回 Cookie、票据、设备号或来源哈希。交换会核对账号一致性，并限制到经过检查的 HTTPS passport 接口和斗鱼一次性登录桥；协议变化或无法确认响应时会失败并退避。

## 实现参考

续期流程使用 `LTP0 + dy_did → safeAuth → 主站登录桥 → 新 acf_* Cookie → 登录校验`。实现核对了以下参考源码；斗鱼接口仍可能改变，长期票据也可能被平台撤销：

- [bililive-go：refresh.go](https://github.com/bililive-go/bililive-go/blob/c5a96e2818ca25d6046ea48812d5e193ecc394fc/src/live/douyu/refresh.go)
- [bililive-go：douyu_keeper.go](https://github.com/bililive-go/bililive-go/blob/c5a96e2818ca25d6046ea48812d5e193ecc394fc/src/servers/douyu_keeper.go)
- [douyu-keep-just-works：douyu-passport.ts](https://github.com/tophtab/douyu-keep-just-works/blob/e1290ed111768d068f02c07ed616f42e4562c3a4/src/core/douyu-passport.ts)

自动续期的本地回归入口：

```bash
cargo test --locked -p biliup --lib douyu
cargo test --locked -p biliup-cli --lib douyu_keeper
node --test app/lib/douyu-auth.test.cjs
```
