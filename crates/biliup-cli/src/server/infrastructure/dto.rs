use crate::server::infrastructure::models::live_streamer::LiveStreamer;
use serde::Serialize;

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
}
