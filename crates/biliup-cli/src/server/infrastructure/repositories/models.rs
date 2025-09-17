use chrono::NaiveDateTime;
use ormlite::Model;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub tags: Value, // not null
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
    pub id: i64,
    pub url: String,
    pub remark: String,
    pub filename_prefix: Option<String>,
    pub time_range: Option<String>,
    pub upload_streamers_id: Option<i64>, // FK，可空
    pub format: Option<String>,
    #[ormlite(column = "override")]
    pub override_: Option<Value>, // "override" 为关键字，字段名避让
    pub preprocessor: Option<Value>,
    pub segment_processor: Option<Value>,
    pub downloaded_processor: Option<Value>,
    pub postprocessor: Option<Value>,
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
#[ormlite(table = "configuration")]
pub struct Configuration {
    pub id: i64,
    pub key: String,
    pub value: String, // TEXT
}
