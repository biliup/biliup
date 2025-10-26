pub mod yy;

use crate::server::common::construct_headers;
use crate::server::common::util::{Recorder, parse_time};
use crate::server::config::{Config, default_segment_time};
use crate::server::core::downloader::ffmpeg_downloader::FfmpegDownloader;
use crate::server::core::downloader::stream_gears::StreamGears;
use crate::server::core::downloader::{DownloadConfig, Downloader, DownloaderType};
use crate::server::errors::AppError;
use crate::server::infrastructure::context::Context;
use crate::server::infrastructure::models::StreamerInfo;
use async_trait::async_trait;
use biliup::downloader::util::Segmentable;
use error_stack::Report;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// 流信息结构
/// 包含直播流的详细信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StreamInfoExt {
    pub streamer_info: StreamerInfo,
    /// 原始流URL
    pub raw_stream_url: String,
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
    Live { stream_info: Box<StreamInfoExt> },
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
    async fn get_stream_info(&self) -> Result<StreamInfoExt, Report<AppError>>;
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
        stream_info: &StreamInfoExt,
        config: Config,
        recorder: Recorder,
    ) -> Arc<dyn Downloader> {
        let raw_stream_url = &stream_info.raw_stream_url;
        match config.downloader {
            Some(DownloaderType::Ffmpeg) => {
                let config = DownloadConfig {
                    segment_time: config.segment_time.or_else(default_segment_time),
                    file_size: Some(config.file_size), // 2GB
                    headers: stream_info.stream_headers.clone(),
                    recorder,
                    // output_dir: PathBuf::from("./downloads")
                    output_dir: PathBuf::from("."),
                };

                Arc::new(FfmpegDownloader::new(
                    raw_stream_url,
                    config,
                    Vec::new(),
                    DownloaderType::FfmpegExternal,
                ))
            }
            // Some(DownloaderType::StreamGears) => {
            //
            // },
            _ => Arc::new(StreamGears::new(
                raw_stream_url,
                construct_headers(&stream_info.stream_headers),
                recorder.filename_template(),
                Segmentable::new(
                    config.segment_time.as_deref().map(parse_time),
                    Some(config.file_size),
                ),
                None,
            )),
        }
    }

    /// 初始化弹幕客户端（可选）
    fn danmaku_init(&self) -> Option<Box<dyn Downloader>>;

    /// 获取插件名称
    fn name(&self) -> &str;
}
