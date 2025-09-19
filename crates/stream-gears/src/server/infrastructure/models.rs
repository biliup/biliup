use chrono::NaiveDateTime;
use ormlite::{Insert, Model};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Model, Debug, Clone, Serialize, Deserialize)]
#[ormlite(table = "uploadstreamers")]
pub struct UploadStreamer {
    pub id: i64,
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
    pub up_selection_reply: Option<bool>,
    pub up_close_reply: Option<bool>,
    pub up_close_danmu: Option<bool>,
    pub extra_fields: Option<String>,
    pub is_only_self: Option<i64>,
}

#[derive(Model, Debug, Clone, Serialize, Deserialize)]
#[ormlite(table = "streamerinfo")]
pub struct StreamerInfo {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub title: String,
    pub date: NaiveDateTime,
    pub live_cover_path: String,
}

#[derive(Model, Debug, Clone, Serialize, Deserialize)]
#[ormlite(table = "livestreamers")]
pub struct LiveStreamer {
    #[ormlite(primary_key)]
    pub id: i64,
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
    pub preprocessor: sqlx::types::Json<Option<Vec<String>>>,

    pub segment_processor: sqlx::types::Json<Option<Vec<HookStep>>>,

    pub downloaded_processor: sqlx::types::Json<Option<Vec<HookStep>>>,
    pub postprocessor: sqlx::types::Json<Option<Vec<HookStep>>>,
    pub opt_args: Option<Value>,
    pub excluded_keywords: Option<Value>,
}

#[derive(Model, Debug, Clone, Serialize, Deserialize)]
#[ormlite(table = "filelist")]
pub struct FileItem {
    pub id: i64,
    pub file: String,
    pub streamer_info_id: i64, // FK not null
}

#[derive(Model, Debug, Clone, Serialize, Deserialize)]
#[ormlite(table = "configuration", insert = "InsertConfiguration")]
pub struct Configuration {
    pub id: i64,
    pub key: String,
    pub value: String, // TEXT
}

// 钩子步骤：既支持 key-value 形式（如 {run: "..."}），也支持纯字符串（如 "rm"）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookStep {
    Map(HashMap<String, String>),
    Symbol(String),
}

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
    pub preprocessor: sqlx::types::Json<Option<Vec<String>>>,

    pub segment_processor: sqlx::types::Json<Option<Vec<HookStep>>>,

    pub downloaded_processor: sqlx::types::Json<Option<Vec<HookStep>>>,
    pub postprocessor: sqlx::types::Json<Option<Vec<HookStep>>>,
    pub opt_args: Option<Value>,
    pub excluded_keywords: Option<Value>,
}

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
