+++
title = "斗鱼 Cookie 与画质设置"
description = "配置斗鱼网页版 Cookie，检查登录状态与实际录制画质"
date = 2026-10-07T00:00:00+00:00
updated = 2026-10-08T00:00:00+00:00
draft = false
weight = 100
sort_by = "weight"
template = "docs/page.html"
[extra]
lead = "部分斗鱼直播间限制匿名用户的原画档位。本文说明如何配置登录 Cookie，以及如何核对实际返回的画质和编码。"
toc = true
top = false
+++

biliup 使用斗鱼当前网页版的签名和播放接口取流。将 `douyu_rate` 设为 `0` 表示请求最高画质，实际档位由平台返回；部分房间会将匿名请求降到较低档位。配置有效的网页版登录 Cookie 后，可获取账号有权观看的画质。

“原画”是平台的档位名称，不能统一等同于 1080P、60fps 或固定码率。不同房间的分辨率、帧率和档位编号可能不同。Cookie 也不能保证连接永不中断。

## 获取并配置 Cookie

1. 在浏览器打开 [斗鱼](https://www.douyu.com/) 并登录。
2. 打开开发者工具的 **Network（网络）** 面板，刷新直播间。
3. 选择发送到 `www.douyu.com` 的请求，在 **Request Headers（请求头）** 中复制 `Cookie` 的值。
4. 在 biliup 的斗鱼平台设置里，将其粘贴到 **登录 Cookie（douyu_cookie）**，保存配置。
5. 点击 **测试 Cookie** 检查账号登录状态。

示例格式（下面的值仅为占位）：

```
acf_uid=123456; acf_auth=your_auth_value; acf_did=your_device_id; ...
```

也支持 [EditThisCookie (V3)](https://chromewebstore.google.com/detail/editthiscookie-v3/ojfebgpkimhlhcblbalbfjblapadhbol?pli=1) 等浏览器插件导出的 JSON 数组，可直接粘贴：

```json
[
  {"domain": ".douyu.com", "name": "acf_uid", "value": "123456"},
  {"domain": ".douyu.com", "name": "acf_auth", "value": "your_auth_value"},
  {"domain": ".douyu.com", "name": "acf_did", "value": "your_device_id"}
]
```

JSON 导入只使用 `.douyu.com`、`douyu.com` 或 `www.douyu.com` 的 Cookie。其他网站和 `passport.douyu.com` 的 Cookie 不会被发送到播放接口。请将 JSON 内容粘贴到配置字段；该字段不是文件路径。

`acf_uid` 和 `acf_auth` 需要非空。建议复制完整 Cookie，不要对其中的 `%xx` 编码或 `=` 字符手动解码。设备 ID 留空时，会先使用 Cookie 中的设备标识，再使用默认值；通常无需另外填写。

## 检查登录状态和画质

**测试 Cookie** 会访问需要登录的个人中心页面。未登录或 Cookie 过期时，斗鱼会将请求重定向到登录页，验证结果为无效。公开的直播间信息接口即使未登录也能成功，因此不能用它判断 Cookie 是否有效。

登录状态有效，表示斗鱼接受了该账号的登录凭据；这并不保证每个直播间都提供原画或 HEVC。取流日志会报告请求档位和平台实际返回的档位，发生降档时会提示。遇到不符合预期的画质，请同时检查：

- Cookie 验证是否通过，同一账号在网页上是否能选择目标画质。
- `douyu_rate` 是否为目标档位。`0` 请求原画；常见编号还包括 `8`（蓝光 8M）、`4`（蓝光 4M）、`3`（超清）、`2`（高清），以具体房间为准。
- 实际文件的分辨率、帧率和编码。可以用媒体播放器的属性窗口或 `ffprobe` 检查，不能仅凭文件名或请求参数判断。
- 房间是否正在循环播放。biliup 目前将 `videoLoop=1` 的房间视为非实时直播并跳过录制。

新网页版接口出错时，biliup 可回退到 App 播放接口。网页版 Cookie 不等于 App 登录 token，回退取流的画质可能受限，日志会说明回退原因。

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

GitHub Actions 的 **Douyu live stream smoke test** 工作流支持手动运行，输入房间号后分别检查 AVC 和 HEVC 的匿名播放。仓库维护者可自行配置 `DOUYU_COOKIE` Secret，以额外检查登录播放；没有 Secret 时该步骤会跳过。工作流不会上传 Cookie 或媒体样本。线上检查受房间状态、地域、账号和 CDN 可达性影响，独立于每次 PR 执行的离线测试。

## Cookie 过期与保管

验证失效时，重新登录斗鱼并复制新的 Cookie。完整 Cookie 是登录凭据，不要将它放入公开截图、工单、日志、仓库或分享的配置文件。如果已泄露，请退出相关登录会话并更新凭据。
