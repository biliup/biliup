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
}
