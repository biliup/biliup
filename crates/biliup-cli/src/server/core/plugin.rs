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

/// 流信息结构
/// 包含直播流的详细信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    /// 主播名称
    pub name: String,
    /// 直播间URL
    pub url: String,
    /// 原始流URL
    pub raw_stream_url: String,
    /// 直播标题
    pub title: String,
    /// 直播开始时间
    #[serde(with = "time::serde::rfc3339")]
    pub date: OffsetDateTime,
    /// 直播封面路径（可选）
    pub live_cover_path: Option<String>,
    /// 平台名称
    pub platform: String,
    /// 流请求头
    pub stream_headers: HashMap<String, String>,
    /// 文件后缀
    pub suffix: String,
}

/// 流状态枚举
/// 表示直播流的当前状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamStatus {
    /// 正在直播，包含流信息
    Live { stream_info: StreamInfo },
    /// 离线状态
    Offline,
    /// 未知状态
    Unknown,
}

/// 下载基础trait
/// 定义下载器的基本功能接口
#[async_trait]
pub trait DownloadBase: Send + Sync {
    /// 检查流是否可用
    async fn check_stream(&self) -> Result<bool, Report<AppError>>;
    /// 获取流信息
    async fn get_stream_info(&self) -> Result<StreamInfo, Report<AppError>>;
    /// 下载到指定路径
    async fn download(&self, output_path: impl AsRef<Path>) -> Result<(), Report<AppError>>;
    /// 判断是否应该录制
    fn should_record(&self, room_title: &str) -> bool;
    /// 获取平台名称
    fn get_platform_name(&self) -> &'static str;
}

/// 下载插件trait
/// 定义下载插件必须实现的接口
#[async_trait]
pub trait DownloadPlugin {
    /// 检查URL是否匹配此插件
    fn matches(&self, url: &str) -> bool;
    /// 检查流状态
    async fn check_status(&self, ctx: &mut Context) -> Result<StreamStatus, Report<AppError>>;
    /// 创建下载器实例
    async fn create_downloader(
        &self,
        stream_info: &StreamInfo,
        config: Config,
        recorder: Recorder,
    ) -> Box<dyn Downloader>;

    /// 初始化弹幕客户端（可选）
    fn danmaku_init(&self) -> Option<Box<dyn Downloader>>;

    /// 获取插件名称
    fn name(&self) -> &str;
}
