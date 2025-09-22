use crate::server::common::util::Recorder;
use crate::server::config::Config;
use crate::server::core::downloader::Downloader;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::{Context, Worker};
use async_trait::async_trait;
use error_stack::Report;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use time::OffsetDateTime;

// Stream information structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub name: String,
    pub url: String,
    pub raw_stream_url: String,
    pub title: String,
    #[serde(with = "time::serde::rfc3339")]
    pub date: OffsetDateTime, // 保存 Python 的时间对象（如 time.struct_time）
    // pub end_time: PyObject,   // 同上
    pub live_cover_path: Option<String>,
    pub platform: String,
    pub stream_headers: HashMap<String, String>,
    pub suffix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamStatus {
    Live { stream_info: StreamInfo },
    Offline,
    Unknown,
}

#[async_trait]
pub trait DownloadBase: Send + Sync {
    async fn check_stream(&self) -> Result<bool, Report<AppError>>;
    async fn get_stream_info(&self) -> Result<StreamInfo, Report<AppError>>;
    async fn download(&self, output_path: impl AsRef<Path>) -> Result<(), Report<AppError>>;
    fn should_record(&self, room_title: &str) -> bool;
    fn get_platform_name(&self) -> &'static str;
}

#[async_trait]
pub trait DownloadPlugin {
    fn matches(&self, url: &str) -> bool;
    async fn check_status(&self, ctx: &mut Context) -> Result<StreamStatus, Report<AppError>>;
    async fn create_downloader(
        &self,
        stream_info: &StreamInfo,
        config: Config,
        recorder: Recorder,
    ) -> Box<dyn Downloader>;

    fn danmaku_init(&self) -> Option<Box<dyn Downloader>>;

    fn name(&self) -> &str;
}
