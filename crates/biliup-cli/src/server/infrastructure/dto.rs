use crate::server::infrastructure::models::live_streamer::LiveStreamer;
use biliup::downloader::preview::PreviewStatus;
use serde::Serialize;

/// 正在录制的直播间能否在页面内预览（复用正在录制的那一路流）。
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct LivePreviewResponse {
    /// 当前下载器能否提供预览。为 `true` 时 `format` 仍可能是 `null`（刚开始拉流、容器未定）
    pub available: bool,
    /// `flv` / `mpegts` / `fmp4`，即 `GET /v1/streamers/{id}/live` 的容器，与其 `Content-Type` 一致
    pub format: Option<&'static str>,
    /// `fmp4` 时从 init segment 解出的 RFC 6381 编码串（如 `avc1.64001f,mp4a.40.2`），
    /// 供前端 `MediaSource.isTypeSupported()` 校验后 `addSourceBuffer`；其它容器为 `null`
    pub codecs: Option<String>,
    /// 不可预览的原因（ffmpeg / streamlink 子进程落盘、HEVC FLV 等）
    pub reason: Option<String>,
    /// 这一路有没有实时弹幕（平台实现了弹幕客户端），有则 `GET /v1/streamers/{id}/danmaku` 可用
    pub danmaku: bool,
    /// 浏览器能否直连 CDN 拉这一路（`preview_transport = direct` 时前端据此选直连或回落中转）
    pub direct: DirectCapability,
}

/// 浏览器直连 CDN 的能力判定（按平台 CDN 的跨域放行与并发策略实测得出）。
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct DirectCapability {
    pub capable: bool,
    /// 不能直连的原因；`capable = true` 时为 `null`
    pub reason: Option<String>,
}

impl LivePreviewResponse {
    pub fn new(status: PreviewStatus, danmaku: bool, direct: DirectCapability) -> Self {
        Self {
            available: status.available,
            format: status.format.map(|f| f.as_str()),
            codecs: status.codecs,
            reason: status.reason,
            danmaku,
            direct,
        }
    }
}

/// 直播主播响应数据传输对象
/// 包含主播信息和当前工作状态
#[derive(Serialize)]
pub struct LiveStreamerResponse {
    /// 主播基本信息（展开到顶层）
    #[serde(flatten)]
    pub inner: LiveStreamer,

    /// 当前工作状态
    pub status: String,
    /// 上传状态
    pub upload_status: String,

    /// 正在录制时最近一个滑动窗口内的写盘速率（字节/秒）。
    /// 未录制、尚无采样、或下载器不经过本进程写盘（边录边传、yt-dlp）时为 `null`。
    pub live_bytes_per_sec: Option<u64>,
    /// 正在录制的直播间封面地址；图片本身请走 `GET /v1/streamers/{id}/cover` 代理。
    pub live_cover_url: Option<String>,
    /// 正在录制的主播头像地址；图片本身请走 `GET /v1/streamers/{id}/avatar` 代理。
    pub live_avatar_url: Option<String>,
    /// 正在录制时的预览能力；未录制为 `null`。视频流走 `GET /v1/streamers/{id}/live`。
    pub preview: Option<LivePreviewResponse>,
    /// 正在录的场次（切片工作台）；未录制或第一个分段还没开写时为 `null`。
    pub session_id: Option<i64>,
}
