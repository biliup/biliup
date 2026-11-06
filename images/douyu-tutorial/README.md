# 斗鱼教程截图

对应页面：`docs/content/docs/tutorials/douyu-cookie-guide.md`。

图片由维护者手动截取并插入正文。此处记录目标文件名，不提供空图片或含真实凭据的示例截图。

| 文件名 | 需要展示的内容 |
| --- | --- |
| `passport-cookie-export.png` | 浏览器在 `www.douyu.com` 下的 Cookie 名称、所属域及导出入口；不展示 Cookie 值 |
| `passport-cookie-export_2.png` | 浏览器在 `passport.douyu.com` 下的 Cookie 名称及所属域，标出 `LTP0` 与配套 `dy_did`；不展示 Cookie 值，遮挡登录二维码 |
| `douyu-login-settings.png` | biliup 斗鱼设置中的 Cookie、LTP0、续期设备号和自动续期开关，以及登录状态和手动续期入口；所有凭据、账号 UID 和设备标识遮挡 |
| `douyu-login-status.png`（可选） | 单独展示登录状态、上次校验、上次续期及下次时间、手动续期入口；账号 UID 和设备标识遮挡 |

保存位置为本目录，例如 `docs/static/images/douyu-tutorial/douyu-login-status.png`。当前 `/biliup` 部署前缀下的引用路径是 `/biliup/images/douyu-tutorial/douyu-login-status.png`，不要把 `static/` 写入图片 URL。

使用 PNG，裁掉无关窗口区域，确保文字清晰。凭据必须在截图前隐藏或在导出后用不透明色块彻底遮盖；检查 Cookie、LTP0、设备号、账号 ID、登录二维码、签名 URL 以及浏览器其他标签页内容。教程中的文字占位可以在插图后删除。
