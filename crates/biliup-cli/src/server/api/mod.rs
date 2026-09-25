/// 业务路由的访问控制（登录 + 权限点）
pub mod access;
/// 认证相关API
pub mod auth;
/// B站API端点
pub mod bilibili_endpoints;
/// 通用API端点
pub mod endpoints;
/// 录制中直播间的封面 / 头像图片代理
pub mod live_media;
/// 直播预览：把正在录制的流旁路给页面内播放器
pub mod live_preview;
/// 录制中直播间的写盘速率（每秒轮询的瘦端点，供码率曲线）
pub mod live_rates;
/// 非超管的配置与主播数据脱敏
pub mod redact;
/// 「保留这场」：改场次的保留期
pub mod session_retention;
/// 单页应用静态文件处理
pub mod spa;
/// Web 用户管理与 /v1/me
pub mod web_users;
pub mod ws;
