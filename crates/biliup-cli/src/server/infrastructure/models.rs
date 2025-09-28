/// 钩子步骤模块
pub mod hook_step;
pub mod live_streamer;
pub mod upload_streamer;

use chrono::{DateTime, Utc};
use ormlite::{Insert, Model};
use serde::{Deserialize, Serialize};
/// 主播信息模型
/// 存储主播的基本信息和直播状态
#[derive(Model, Debug, Clone, Serialize, Deserialize, Default)]
#[ormlite(table = "streamerinfo")]
pub struct StreamerInfo {
    /// 主键ID
    pub id: i64,
    /// 主播名称
    pub name: String,
    /// 直播间URL
    pub url: String,
    /// 直播标题
    pub title: String,
    /// 直播开始时间
    pub date: DateTime<Utc>,
    /// 直播封面路径（可选）
    pub live_cover_path: String,
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
    /// 关联的主播信息ID（外键，非空）
    pub streamer_info_id: i64,
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
