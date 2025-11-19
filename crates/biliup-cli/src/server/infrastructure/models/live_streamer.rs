use crate::server::infrastructure::models::hook_step::HookStep;
use ormlite::{Insert, Model};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::server::config::{Config, ConfigPatch};

/// 直播主播模型
/// 存储直播主播的配置信息和录制参数
#[derive(Model, Debug, Clone, Serialize, Deserialize)]
#[ormlite(table = "livestreamers")]
pub struct LiveStreamer {
    /// 主键ID
    #[ormlite(primary_key)]
    pub id: i64,
    /// 直播间URL
    pub url: String,
    /// 备注名称
    pub remark: String,
    /// 文件名前缀
    pub filename_prefix: Option<String>,
    /// 录制时间范围
    pub time_range: Option<String>,
    /// 关联的上传配置ID（外键，可空）
    pub upload_streamers_id: Option<i64>,
    /// 录制格式
    pub format: Option<String>,
    /// 覆盖配置（"override"为关键字，字段名避让）
    #[ormlite(column = "override", json)]
    #[serde(rename = "override")]
    pub override_cfg: Option<ConfigPatch>,

    /// 预处理器列表（JSON格式）
    /// 注意：数据库空与json空有区别，所以这里不能用#[ormlite(json)]
    /// 只能使用 sqlx::types::Json
    #[ormlite(json)]
    pub preprocessor: Option<Vec<HookStep>>,
    /// 分段处理器列表（JSON格式）
    #[ormlite(json)]
    pub segment_processor: Option<Vec<HookStep>>,
    /// 下载完成处理器列表（JSON格式）
    #[ormlite(json)]
    pub downloaded_processor: Option<Vec<HookStep>>,
    /// 后处理器列表（JSON格式）
    #[ormlite(json)]
    pub postprocessor: Option<Vec<HookStep>>,
    /// 可选参数
    pub opt_args: Option<Value>,
    /// 排除关键词
    pub excluded_keywords: Option<Value>,
}

/// 插入直播主播的数据结构
/// 用于创建新的直播主播记录
#[derive(Insert, Debug, Serialize, Deserialize)]
#[ormlite(returns = "LiveStreamer")]
pub struct InsertLiveStreamer {
    pub url: String,
    pub remark: String,
    pub filename_prefix: Option<String>,
    pub time_range: Option<String>,
    pub upload_streamers_id: Option<i64>, // FK，可空
    pub format: Option<String>,
    #[ormlite(column = "override", json)]
    // “override” 是字段名，这里改为 override_cfg 避免与保留字混淆
    #[serde(rename = "override")]
    pub override_cfg: Option<ConfigPatch>, // "override" 为关键字，字段名避让

    // #[ormlite(json)] 数据库空与json空有区别所以这里不能用
    // pub preprocessor: Option<Vec<String>>,
    // 只能使用 sqlx::types::Json
    #[ormlite(json)]
    pub preprocessor: Option<Vec<HookStep>>,
    #[ormlite(json)]
    pub segment_processor: Option<Vec<HookStep>>,
    #[ormlite(json)]
    pub downloaded_processor: Option<Vec<HookStep>>,
    #[ormlite(json)]
    pub postprocessor: Option<Vec<HookStep>>,
    pub opt_args: Option<Value>,
    pub excluded_keywords: Option<Value>,
}
