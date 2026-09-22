use crate::server::infrastructure::models::live_streamer::LiveStreamer;
use biliup::downloader::preview::PreviewStatus;
use serde::Serialize;

/// 正在录制的直播间能否在页面内预览（复用正在录制的那一路流）。
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct LivePreviewResponse {
    /// 当前下载器能否提供预览。为 `true` 时 `format` 仍可能是 `null`（刚开始拉流、容器未定）
    pub available: bool,
    /// `flv` / `mpegts`，即 `GET /v1/streamers/{id}/live` 的容器，与其 `Content-Type` 一致
    pub format: Option<&'static str>,
    /// 不可预览的原因（ffmpeg / streamlink 子进程落盘、fMP4、HEVC 等）
    pub reason: Option<String>,
}

impl From<PreviewStatus> for LivePreviewResponse {
    fn from(status: PreviewStatus) -> Self {
        Self {
            available: status.available,
            format: status.format.map(|f| f.as_str()),
            reason: status.reason,
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
}
