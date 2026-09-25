/// 钩子步骤模块
pub mod hook_step;
pub mod live_streamer;
pub mod upload_streamer;

use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use ormlite::{Insert, Model};
use serde::{Deserialize, Serialize};
/// 一场直播（`stream_sessions` 表）：开播时的主播名、直播间、标题与开播时间。
/// 表里还有切片工作台的列（主播外键、时间轴零点、结束时间等），由
/// [`crate::server::workbench::store`] 读写，不在这个模型里，`/v1/streamer-info` 的
/// JSON 因此保持原样。
#[derive(Model, Debug, Clone, Serialize, Deserialize, Default)]
#[ormlite(table = "stream_sessions")]
pub struct StreamerInfo {
    /// 主键ID
    pub id: i64,
    /// 主播名称
    pub name: String,
    /// 直播间URL
    pub url: String,
    /// 直播标题
    pub title: String,
    #[serde(with = "ts_seconds")]
    /// 直播开始时间
    pub date: DateTime<Utc>,
    /// 直播封面路径（可选）
    pub live_cover_path: String,
}

impl StreamerInfo {
    pub fn new(
        name: &str,
        url: &str,
        title: &str,
        date: DateTime<Utc>,
        live_cover_path: &str,
    ) -> Self {
        Self {
            id: -1,
            name: name.to_string(),
            url: url.to_string(),
            title: title.to_string(),
            date,
            live_cover_path: live_cover_path.to_string(),
        }
    }
}

/// 文件列表模型
/// 存储录制文件的信息
#[derive(Model, Debug, Clone, Serialize, Deserialize)]
#[ormlite(table = "filelist", insert = "InsertFileItem")]
pub struct FileItem {
    /// 主键ID
    pub id: i64,
    /// 文件路径
    pub file: String,
    /// 所属场次（`stream_sessions.id`，非空）。JSON 仍叫 `streamer_info_id`。
    #[serde(rename = "streamer_info_id")]
    pub session_id: i64,
}

/// 配置模型
/// 存储应用程序的配置信息
#[derive(Model, Debug, Clone, Serialize, Deserialize)]
#[ormlite(table = "configuration")]
pub struct Configuration {
    /// 主键ID
    pub id: i64,
    /// 配置键
    pub key: String,
    /// 配置值（TEXT类型）
    pub value: String,
}

/// 插入配置的数据结构
/// 用于创建新的配置记录
#[derive(Insert, Debug, Clone, Serialize, Deserialize)]
#[ormlite(returns = "Configuration")]
pub struct InsertConfiguration {
    /// 配置键
    pub key: String,
    /// 配置值（TEXT类型）
    pub value: String,
}
