mod twitch;
pub mod yy;

use crate::server::core::downloader::{
    DanmakuClient, DownloaderRuntime, DownloaderType, cover_downloader,
};
use crate::server::errors::AppError;
use crate::server::infrastructure::context::{Context, PluginContext};
use crate::server::infrastructure::models::StreamerInfo;
use async_trait::async_trait;
use axum::http::header::USER_AGENT;
use axum::http::{HeaderMap, HeaderValue};
use error_stack::Report;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};

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
}

/// 下载基础trait
/// 定义下载器的基本功能接口
#[async_trait]
pub trait DownloadBase: Send + Sync {
    /// 获取流信息
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>>;
    /// 下载到指定路径
    fn downloader(&self, downloader_type: DownloaderType) -> DownloaderRuntime {
        DownloaderRuntime::from_type(downloader_type)
    }

    /// 初始化弹幕客户端（可选）
    fn danmaku_init(&self) -> Option<Arc<dyn DanmakuClient + Send + Sync>> {
        None
    }
    // /// 获取平台名称
    // fn get_platform_name(&self) -> &'static str;
}

/// 下载插件trait
/// 定义下载插件必须实现的接口
pub trait DownloadPlugin {
    /// 检查URL是否匹配此插件
    fn matches(&self, url: &str) -> bool;

    /// 创建下载器实例
    fn create_downloader(&self, ctx: &mut PluginContext) -> Box<dyn DownloadBase>;

    /// 获取插件名称
    fn name(&self) -> &str;
}
