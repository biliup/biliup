# 斗鱼 Web 登录 Cookie 自动续期

更新日期：2026-10-09。此功能使用斗鱼 passport 长期凭据 `LTP0` 与配套的 `dy_did` 换取新的 Web 登录 Cookie，并为运行中的请求提供更新后的 `acf_auth`。来源配置、运行时续期结果和续期状态分别维护，不要求重启程序。

## 设置与 GUI

在本机「空间配置 → 平台设置 → 斗鱼」中保存：

| 配置字段 | 用途 |
| --- | --- |
| `douyu_cookie` | 斗鱼 Web Cookie 字符串或浏览器导出的 Cookie JSON 数组，通常包含 `acf_uid` 与 `acf_auth` |
| `douyu_ltp0` | passport 长期凭据 `LTP0`；完整导出中已有时可留空 |
| `douyu_refresh_device_id` | 与该 LTP0 配套的 passport `dy_did`；完整导出中已有时可留空 |
| `douyu_auto_refresh` | 自动续期开关，未设置时默认开启；缺少凭据对时仅能校验已有 Cookie |
| `douyu_deviceId` | 原有取流设备 ID，仍按现有取流逻辑使用；不是续期凭据字段 |

使用同一账号、同一次浏览器登录取得的 Web Cookie、LTP0 和 dy_did。Cookie JSON 可包含 `www.douyu.com` / `.douyu.com` 与 `passport.douyu.com` 的条目；程序识别作用域并分离长期凭据。格式错误、冲突的同名字段及非法请求头字符会被拒绝，不要把不同账号的导出合并。

状态面板展示登录状态、续期状态、账号 UID、上次登录校验、上次成功续期和下次续期/重试时间。凭据失效时提示重新登录；没有配套长期凭据时提示补充。

「手动续期」只使用已经保存的配置。表单有未保存修改时按钮不可用，应先保存；「测试 Cookie」校验当前输入框的 Cookie，不会保存输入值。手动续期即使自动开关关闭也可调用，但同一来源最短 60 秒一次，重复点击和正在续期时不会启动第二次交换。

续期管理界面只挂载在本机全局配置页，不在 Fleet 配置页或主播覆写抽屉操作控制面的本机账号。后端状态与手动接口也支持 `streamer_id` 指定主播范围；主播配置中的 Cookie 发生变化时会隔离全局长期凭据，避免继承其他账号的票据和设备号。

## 来源配置与运行时凭据

| 数据 | 存放与行为 |
| --- | --- |
| 用户保存的来源凭据 | 保留在已有全局配置或主播覆写中，GUI 输入框继续显示导入的来源值 |
| 最新 Web Cookie 与 passport 凭据 | 存放在 SQLite 的 `douyu_cookie_keeper` 表，与来源凭据的规范化哈希绑定 |
| 续期状态 | 同表保存校验/成功/尝试时间、下次时间、失败次数、固定错误及租约 |
| 运行中的取流请求 | 每次构造新请求配置时按匹配来源解析最新 Cookie；长期 LTP0 不作为普通 Web API Cookie 发送 |

续期不会把新的 Cookie 反写进来源表单。保持已保存的来源值不变即可使用表内最新结果；改变 Cookie、LTP0 或续期设备号后产生新的来源绑定，旧账号的结果不会套到新配置上，已不再使用的来源状态会清理。

「无需重启」是指后续 Web API 请求立即能取得最新 Cookie。已经建立的 CDN 视频连接无需也不会在续期时被强制重连，已有连接是否继续可用仍由平台决定。

## 周期、退避与恢复

- 自动调度默认每分钟检查一次到期状态，续期成功后的下一次为 3 天后。
- 启动时加载已有续期结果，之后安排登录校验；保存的下次续期/失败退避不会因为重启变成无限重复交换。
- 临时网络、接口或协议错误从 5 分钟开始指数退避，上限 6 小时。临时失败保留已有可用 Cookie。
- 明确登录失效或账号不匹配会进入「凭据已失效」，安排 24 小时后重试，并提示重新登录。已确认失效的 Cookie 不再用于取流请求。
- 本进程使用来源级互斥，并通过同一 SQLite 表的 5 分钟租约防止重复续期。整个交换最多 240 秒，每个 HTTP 请求有超时；重启后的陈旧租约到期后可恢复。
- 交换成功但状态写盘失败时，先保留本进程中的结果并重试保存，避免立刻再次交换票据；此时 GUI 提示等待重试，落库前重启仍可能丢失尚未保存的新结果。

时间字段为 Unix 秒，GUI 按浏览器所在时区显示。系统时间回退与失败退避的恢复逻辑有针对性测试，但修改系统时钟仍可能改变实际执行时间。

## 已核实的协议

实现先阅读实际源码，使用以下不可变版本作为参考：

- bililive-go [refresh.go（c5a96e2818ca25d6046ea48812d5e193ecc394fc）](https://github.com/bililive-go/bililive-go/blob/c5a96e2818ca25d6046ea48812d5e193ecc394fc/src/live/douyu/refresh.go)。
- bililive-go [douyu_keeper.go（同一版本）](https://github.com/bililive-go/bililive-go/blob/c5a96e2818ca25d6046ea48812d5e193ecc394fc/src/servers/douyu_keeper.go)，并核对同项目的登录校验代码。
- douyu-keep-just-works [douyu-passport.ts（e1290ed111768d068f02c07ed616f42e4562c3a4）](https://github.com/tophtab/douyu-keep-just-works/blob/e1290ed111768d068f02c07ed616f42e4562c3a4/src/core/douyu-passport.ts)。

流程调用 `https://passport.douyu.com/lapi/passport/iframe/safeAuth`，携带 LTP0 / dy_did 与参考实现中的 client、时间和回调参数。关闭 reqwest 自动重定向，只允许经过检查的 HTTPS passport safeAuth 和斗鱼 `/api/passport/login` 一次性登录桥，最多 8 跳；其他主机、路径、HTTP、用户名/密码、非默认端口或片段被拒绝。

LTP0 仅用于允许的续期交换与一次性登录桥，不进入普通取流 Web API 请求。收集响应中的 Cookie 时检查作用域和路径，更新 passport 凭据与 Web 凭据；要求新的 `acf_uid`、`acf_auth`、`acf_stk`、`acf_ltkid`、`acf_biz`、`acf_ct` 齐全，核对账号 UID，保留已有 Web 设备号与其他正常 Cookie。

最后调用登录校验接口 `https://www.douyu.com/wgapi/livenc/liveweb/follow/top3`。只有预期的 JSON 登录结果才能确认有效；HTTP 200、HTML 页面、缺失 error 字段或格式错误不算成功。续期结果必须通过该校验才发布给运行中的请求。

## 凭据保护与部署限制

状态 API 为 `GET /v1/douyu/auth/status`、`POST /v1/douyu/auth/refresh`，均要求现有 `config.edit` 权限，返回状态并设置 `Cache-Control: no-store`，不返回 Cookie、LTP0、设备号或来源哈希。测试接口为 `POST /v1/douyu/validate-cookie`。错误消息使用固定文本，界面只接收安全状态字段。

新字段默认不在非管理员配置可见白名单内，也不作为 Fleet 全局配置下发。房间共享配置如携带 Web Cookie，传输时也会剥离其中嵌入的 passport 长期凭据；LTP0 与续期 dy_did 按节点本地管理。新增凭据结构的调试输出经过脱敏，主播删除日志不再输出完整覆写配置。管理员配置接口仍可以读写来源凭据，访问控制遵循项目已有权限与 `--auth` 模式。

凭据保存在本机 SQLite 中，**并非加密保险库**。保护配置文件、数据库、备份和日志访问权限。Unix 上新建及已存在的数据库文件、SQLite 的 WAL/SHM 文件在打开前设为 0600；备份和配置导出仍需由部署者保护。不要在 issue、截图、共享配置或日志中粘贴真实 Cookie。LTP0 也不应写入命令行参数或仓库文件。

续期状态和互斥范围以本机进程及其 SQLite 数据库为单位，不跨 Fleet 节点同步。每个节点需要本地保存自己的登录凭据对。多个节点使用同一账号的长期票据时，不同数据库之间无法协调票据轮换，应分别管理各节点的凭据；没有实现跨机器去重或集群级账号续期服务。异步账号切换、旧请求完成和多来源共享同一账号的防串用逻辑有单元测试，但不等同于保证平台允许同一账号无限多端并发。

## 验证证据

本次执行了本地模拟 HTTP 协议测试、Cookie 作用域/冲突/重定向/账号一致性测试、续期调度和重启恢复测试、GUI 安全状态解析与脏表单测试；前端 TypeScript 检查和生产构建通过。凭据 Debug 脱敏的针对性 Rust 测试通过。

在用户授权的真实凭据验证中，safeAuth 交换成功执行一次，新的 Cookie 通过登录校验，账号一致性得到确认。真实凭据和输出没有进入仓库：更新后的凭据仅保存在仓库外的受保护临时文件中，输出采用安全状态摘要。该单次成功证明本次协议可用，不能保证斗鱼以后不更改接口或凭据永不过期。

常用本地回归入口：

```bash
cargo test -p biliup --lib douyu
cargo test -p biliup-cli --lib douyu_keeper
cargo test -p biliup credential_debug --lib
node --test app/lib/douyu-auth.test.cjs
```

实际接口和实现分别位于 `crates/biliup/src/downloader/live/douyu_refresh.rs`、`crates/biliup-cli/src/server/services/douyu_keeper.rs`、`server/api/douyu_validation.rs` 与 `app/ui/plugins/DouyuAuthPanel.tsx`。
