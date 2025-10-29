use crate::server::common::download::DownloadTask;
use crate::server::common::util::Recorder;
use crate::server::config::Config;
use crate::server::core::downloader::Downloader;
use crate::server::core::plugin::StreamInfoExt;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::live_streamer::LiveStreamer;
use crate::server::infrastructure::models::upload_streamer::UploadStreamer;
use axum::http::Extensions;
use biliup::client::StatelessClient;
use core::fmt;
use ormlite::Model;
use std::sync::{Arc, RwLock};
use tracing::info;

/// 应用程序上下文，包含工作器和扩展信息
#[derive(Debug, Clone)]
pub struct Context {
    /// 工作器实例
    pub worker: Arc<Worker>,
    pub stream_info: StreamInfoExt,
    /// 扩展数据容器
    pub extension: Extensions,
    pub pool: ConnectionPool,
    pub recorder: Recorder,
}

impl Context {
    /// 创建新的上下文实例
    ///
    /// # 参数
    /// * `worker` - 工作器实例的Arc引用
    pub fn new(worker: Arc<Worker>, pool: ConnectionPool) -> Self {
        Self {
            worker,
            stream_info: Default::default(),
            extension: Default::default(),
            pool,
            recorder: Default::default(),
        }
    }
}

/// 工作器结构体，管理单个主播的录制和上传任务
#[derive(Debug)]
pub struct Worker {
    /// 下载器状态
    pub downloader_status: tokio::sync::RwLock<WorkerStatus>,
    /// 上传器状态
    pub uploader_status: RwLock<WorkerStatus>,
    /// 直播主播信息
    pub live_streamer: LiveStreamer,
    /// 上传配置（可选）
    pub upload_streamer: Option<UploadStreamer>,
    /// 全局配置
    pub config: Arc<RwLock<Config>>,
    /// HTTP客户端
    pub client: StatelessClient,
}

impl Worker {
    /// 创建新的工作器实例
    ///
    /// # 参数
    /// * `live_streamer` - 直播主播信息
    /// * `upload_streamer` - 上传配置（可选）
    /// * `config` - 全局配置的Arc引用
    /// * `client` - HTTP客户端
    pub fn new(
        live_streamer: LiveStreamer,
        upload_streamer: Option<UploadStreamer>,
        config: Arc<RwLock<Config>>,
        client: StatelessClient,
    ) -> Self {
        Self {
            downloader_status: tokio::sync::RwLock::new(Default::default()),
            uploader_status: Default::default(),
            live_streamer,
            upload_streamer,
            config,
            client,
        }
    }

    /// 获取主播信息
    /// 返回当前工作器关联的直播主播信息
    pub fn get_streamer(&self) -> LiveStreamer {
        self.live_streamer.clone()
    }

    /// 获取上传配置
    /// 返回当前工作器的上传配置（如果存在）
    pub fn get_upload_config(&self) -> Option<UploadStreamer> {
        self.upload_streamer.clone()
    }

    /// 获取全局配置
    /// 返回当前的全局配置副本
    pub fn get_config(&self) -> Config {
        self.config.read().unwrap().clone()
    }

    /// 更改工作器状态
    ///
    /// # 参数
    /// * `stage` - 工作阶段（下载或上传）
    /// * `status` - 新的工作状态
    pub async fn change_status(&self, stage: Stage, status: WorkerStatus) {
        match stage {
            Stage::Download => {
                *self.downloader_status.write().await = status;
            }
            Stage::Upload => {
                *self.uploader_status.write().unwrap() = status;
            }
        }
    }
}

pub fn find_worker(workers: &[Arc<Worker>], id: i64) -> Option<&Arc<Worker>> {
    workers.iter().find(|worker| worker.live_streamer.id == id)
}

impl Drop for Worker {
    /// 工作器销毁时的清理逻辑
    fn drop(&mut self) {
        info!("Dropping worker {}", self.live_streamer.id);
    }
}

impl PartialEq for Worker {
    /// 比较两个工作器是否相等（基于主播ID）
    fn eq(&self, other: &Self) -> bool {
        self.live_streamer.id == other.live_streamer.id
    }
}

impl Eq for Worker {}

/// 工作阶段枚举
#[derive(Debug)]
pub enum Stage {
    /// 下载阶段
    Download,
    /// 上传阶段
    Upload,
}

/// 工作器状态枚举
#[derive(Default, Clone)]
pub enum WorkerStatus {
    /// 正在工作
    Working(Arc<DownloadTask>),
    /// 等待中
    Pending,
    /// 空闲状态（默认）
    #[default]
    Idle,
    /// 下载暂停中
    Pause,
}

// 简单 Debug：打印状态名，忽略内部 downloader
impl fmt::Debug for WorkerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            WorkerStatus::Working(_) => "Working",
            WorkerStatus::Pending => "Pending",
            WorkerStatus::Idle => "Idle",
            WorkerStatus::Pause => "Pause",
        };
        f.write_str(name)
    }
}
