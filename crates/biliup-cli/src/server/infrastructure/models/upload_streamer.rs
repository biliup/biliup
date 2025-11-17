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
    /// 仅自己可见
    pub is_only_self: Option<u8>,
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
    pub is_only_self: Option<u8>,
}
