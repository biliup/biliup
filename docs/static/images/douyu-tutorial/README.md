# 斗鱼教程截图

对应页面：`docs/content/docs/tutorials/douyu-cookie-guide.md`。

图片由维护者手动截取并插入正文。此处记录目标文件名，不提供空图片或含真实凭据的示例截图。

| 文件名 | 需要展示的内容 |
| --- | --- |
| `passport-cookie-export.png` | 浏览器在 `passport.douyu.com` 下的 Cookie 名称及所属域，标出 `LTP0` 与配套 `dy_did`；所有值遮挡 |
| `douyu-login-settings.png` | biliup 斗鱼设置中 Cookie、LTP0、续期设备号和自动续期开关的位置；所有凭据内容遮挡 |
| `douyu-login-status.png` | 登录状态、上次校验、上次续期及下次时间、手动续期入口；账号 UID 和设备标识遮挡 |

保存位置为本目录，例如 `docs/static/images/douyu-tutorial/douyu-login-status.png`。当前 `/biliup` 部署前缀下的引用路径是 `/biliup/images/douyu-tutorial/douyu-login-status.png`，不要把 `static/` 写入图片 URL。

使用 PNG，裁掉无关窗口区域，确保文字清晰。凭据必须在截图前隐藏或在导出后彻底遮盖，不能只模糊后仍可辨认；检查 Cookie、LTP0、设备号、账号 ID、签名 URL 以及浏览器其他标签页内容。教程中的文字占位可以在插图后删除。
