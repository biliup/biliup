/// 钩子步骤模块
pub mod hook_step;

use chrono::NaiveDateTime;
use hook_step::HookStep;
use ormlite::{Insert, Model};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 上传配置模型
/// 存储视频上传到B站时的各种配置参数
#[derive(Model, Debug, Clone, Serialize, Deserialize)]
#[ormlite(table = "uploadstreamers")]
pub struct UploadStreamer {
    /// 主键ID
    pub id: i64,
    /// 模板名称
    pub template_name: String,
    /// 视频标题
    pub title: Option<String>,
    /// 分区ID
    pub tid: Option<u16>,
    /// 版权类型（1-自制，2-转载）
    pub copyright: Option<u8>,
    /// 转载来源
    pub copyright_source: Option<String>,
    /// 封面路径
    pub cover_path: Option<String>,
    /// 视频简介
    pub description: Option<String>,
    /// 动态内容
    pub dynamic: Option<String>,
    /// 定时发布时间
    pub dtime: Option<u32>,
    /// 杜比音效
    pub dolby: Option<u8>,
    /// Hi-Res音质
    pub hires: Option<u8>,
    /// 充电专属
    pub charging_pay: Option<u8>,
    /// 禁止转载
    pub no_reprint: Option<u8>,
    /// 上传者
    pub uploader: Option<String>,
    /// 用户Cookie
    pub user_cookie: Option<String>,
    /// 标签列表（JSON格式）
    #[ormlite(json)]
    pub tags: Vec<String>, // not null
    /// 制作人员信息
    pub credits: Option<Value>,
    /// 开启精选评论
    pub up_selection_reply: Option<bool>,
    /// 关闭评论
    pub up_close_reply: Option<bool>,
    /// 关闭弹幕
    pub up_close_danmu: Option<bool>,
    /// 额外字段
    pub extra_fields: Option<String>,
    /// 仅自己可见
    pub is_only_self: Option<i64>,
}

/// 主播信息模型
/// 存储主播的基本信息和直播状态
#[derive(Model, Debug, Clone, Serialize, Deserialize)]
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
    /// 直播时间
    pub date: NaiveDateTime,
    /// 直播封面路径
    pub live_cover_path: String,
}

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
    #[ormlite(column = "override")]
    pub override_: Option<Value>,

    /// 预处理器列表（JSON格式）
    /// 注意：数据库空与json空有区别，所以这里不能用#[ormlite(json)]
    /// 只能使用 sqlx::types::Json
    #[ormlite(json)]
    pub preprocessor: Option<Vec<String>>,
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

/// 文件列表模型
/// 存储录制文件的信息
#[derive(Model, Debug, Clone, Serialize, Deserialize)]
#[ormlite(table = "filelist")]
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
    #[ormlite(column = "override")]
    pub override_: Option<Value>, // "override" 为关键字，字段名避让

    // #[ormlite(json)] 数据库空与json空有区别所以这里不能用
    // pub preprocessor: Option<Vec<String>>,
    // 只能使用 sqlx::types::Json
    #[ormlite(json)]
    pub preprocessor: Option<Vec<String>>,
    #[ormlite(json)]
    pub segment_processor: Option<Vec<HookStep>>,
    #[ormlite(json)]
    pub downloaded_processor: Option<Vec<HookStep>>,
    #[ormlite(json)]
    pub postprocessor: Option<Vec<HookStep>>,
    pub opt_args: Option<Value>,
    pub excluded_keywords: Option<Value>,
}

/// 插入上传配置的数据结构
/// 用于创建新的上传配置记录
#[derive(Model, Insert, Debug, Clone, Serialize, Deserialize)]
#[ormlite(table = "uploadstreamers", returns = "UploadStreamer")]
pub struct InsertUploadStreamer {
    pub id: Option<i64>,
    pub template_name: String,
    pub title: Option<String>,
    pub tid: Option<u16>,
    pub copyright: Option<u8>,
    pub copyright_source: Option<String>,
    pub cover_path: Option<String>,
    pub description: Option<String>,
    pub dynamic: Option<String>,
    pub dtime: Option<u32>,
    pub dolby: Option<u8>,
    pub hires: Option<u8>,
    pub charging_pay: Option<u8>,
    pub no_reprint: Option<u8>,
    pub uploader: Option<String>,
    pub user_cookie: Option<String>,
    #[ormlite(json)]
    pub tags: Vec<String>, // not null
    pub credits: Option<Value>,
    pub up_selection_reply: Option<u8>,
    pub up_close_reply: Option<u8>,
    pub up_close_danmu: Option<u8>,
    pub extra_fields: Option<String>,
    pub is_only_self: Option<i64>,
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
