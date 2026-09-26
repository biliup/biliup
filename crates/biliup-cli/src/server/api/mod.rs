/// 业务路由的访问控制（登录 + 权限点）
pub mod access;
/// 认证相关API
pub mod auth;
/// 自动切片的连通性测试与状态
pub mod auto_clip;
/// B站API端点
pub mod bilibili_endpoints;
/// 切片工作台的切片发布（上传投稿队列、取帧、封面）
pub mod clip_publish;
/// 切片工作台的切片与导出
pub mod clips;
/// 通用API端点
pub mod endpoints;
/// 控制面的节点与加入票据（只在 `--controller` 时注册）
pub mod fleet;
pub mod fleet_config;
pub mod fleet_rooms;
/// 录制中直播间的封面 / 头像图片代理
pub mod live_media;
/// 直播预览：把正在录制的流旁路给页面内播放器
pub mod live_preview;
/// 录制中直播间的写盘速率（每秒轮询的瘦端点，供码率曲线）
pub mod live_rates;
/// 切片工作台的标记
pub mod markers;
/// 非超管的配置与主播数据脱敏
pub mod redact;
/// 「保留这场」：改场次的保留期
pub mod session_retention;
pub mod sessions;
/// 单页应用静态文件处理
pub mod spa;
/// 控制台首页的系统状态（CPU / 内存 / 磁盘 / 网速）
pub mod system_stats;
/// Web 用户管理与 /v1/me
pub mod web_users;
pub mod ws;
