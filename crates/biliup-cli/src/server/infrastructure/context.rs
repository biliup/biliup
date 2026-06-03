use crate::server::common::download::DownloadTask;
use crate::server::common::util::Recorder;
use crate::server::config::Config;
use crate::server::core::downloader::DownloadConfig;
use crate::server::core::live::streamer_info;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::StreamerInfo;
use crate::server::infrastructure::models::live_streamer::LiveStreamer;
use crate::server::infrastructure::models::upload_streamer::UploadStreamer;
use biliup::client::StatelessClient;
use biliup::downloader::live::LiveStream;
use core::fmt;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use struct_patch::Patch;
use tracing::{error, info};

/// 应用程序上下文，包含工作器和扩展信息
#[derive(Debug, Clone)]
pub struct Context {
    id: i64,
    /// 工作器实例
    worker: Arc<Worker>,
    stream: LiveStream,
    streamer_info: StreamerInfo,
    pool: ConnectionPool,
}

impl Context {
    /// 创建新的上下文实例
    ///
    /// # 参数
    /// * `worker` - 工作器实例的Arc引用
    pub fn new(id: i64, worker: Arc<Worker>, pool: ConnectionPool, stream: LiveStream) -> Self {
        let mut streamer_info = streamer_info(&stream);
        streamer_info.id = id;
        Self {
            id,
            worker,
            stream,
            streamer_info,
            pool,
        }
    }

    pub fn worker_id(&self) -> i64 {
        self.worker.id()
    }

    pub fn id(&self) -> i64 {
        self.id
    }

    pub(crate) fn worker(&self) -> &Arc<Worker> {
        &self.worker
    }

    pub fn live_streamer(&self) -> &LiveStreamer {
        &self.worker.get_streamer()
    }

    pub fn stateless_client(&self) -> &StatelessClient {
        &self.worker.client
    }

    pub fn config(&self) -> Config {
        self.worker.get_config()
    }

    pub fn pool(&self) -> &ConnectionPool {
        &self.pool
    }

    pub async fn change_status(&self, stage: Stage, status: WorkerStatus) {
        self.worker.change_status(stage, status).await;
    }

    pub fn status(&self, stage: Stage) -> WorkerStatus {
        match stage {
            Stage::Download => self.worker.downloader_status.read().unwrap().clone(),
            Stage::Upload => self.worker.uploader_status.read().unwrap().clone(),
        }
    }

    pub fn upload_config(&self) -> &Option<UploadStreamer> {
        self.worker.get_upload_config()
    }

    pub fn recorder(&self, streamer_info: StreamerInfo) -> Recorder {
        // 创建录制器
        Recorder::new(
            self.live_streamer()
                .filename_prefix
                .clone()
                .or(self.config().filename_prefix.clone()),
            streamer_info,
        )
    }

    pub fn live_stream(&self) -> &LiveStream {
        &self.stream
    }

    pub fn streamer_info(&self) -> &StreamerInfo {
        &self.streamer_info
    }

    pub fn download_config(&self, stream: &LiveStream) -> DownloadConfig {
        let config = self.config();
        // 确定文件格式后缀
        let suffix = self
            .live_streamer()
            .format
            .clone()
            .unwrap_or_else(|| stream.suffix.to_string());
        let mut stream_info = streamer_info(stream);
        if stream.url == self.stream.url {
            stream_info.id = self.streamer_info.id;
        }
        DownloadConfig {
            // 流URL
            url: stream.raw_stream_url.to_string(),
            segment_time: config.segment_time,
            file_size: config.file_size,
            headers: stream.stream_headers.clone(),
            recorder: self.recorder(stream_info),
            // output_dir: PathBuf::from("./downloads")
            output_dir: PathBuf::from("."),
            suffix,
        }
    }
}

/// 工作器结构体，管理单个主播的录制和上传任务
#[derive(Debug)]
pub struct Worker {
    /// 下载器状态
    pub downloader_status: RwLock<WorkerStatus>,
    /// 上传器状态
    pub uploader_status: RwLock<WorkerStatus>,
    /// 直播主播信息
    pub live_streamer: LiveStreamer,
    /// 上传配置（可选）
    pub upload_streamer: Option<UploadStreamer>,
    /// 全局配置
    config: Arc<RwLock<Config>>,
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
            downloader_status: RwLock::new(Default::default()),
            uploader_status: Default::default(),
            live_streamer,
            upload_streamer,
            config,
            client,
        }
    }

    pub fn id(&self) -> i64 {
        self.live_streamer.id
    }

    /// 获取主播信息
    /// 返回当前工作器关联的直播主播信息
    pub fn get_streamer(&self) -> &LiveStreamer {
        &self.live_streamer
    }

    /// 获取上传配置
    /// 返回当前工作器的上传配置（如果存在）
    pub fn get_upload_config(&self) -> &Option<UploadStreamer> {
        &self.upload_streamer
    }

    /// 获取覆写配置
    /// 返回当前的配置副本
    pub fn get_config(&self) -> Config {
        let mut cfg = self.config.read().unwrap().clone();

        if let Some(cfg_p) = self.live_streamer.override_cfg.clone() {
            cfg.apply(cfg_p)
        }
        cfg
    }

    /// 更改工作器状态
    ///
    /// # 参数
    /// * `stage` - 工作阶段（下载或上传）
    /// * `status` - 新的工作状态
    pub async fn change_status(&self, stage: Stage, status: WorkerStatus) {
        match stage {
            Stage::Download => {
                let task = if let WorkerStatus::Working(task) =
                    &*self.downloader_status.read().unwrap()
                    && !matches!(status, WorkerStatus::Working(_))
                {
                    Some(task.clone())
                } else {
                    None
                };

                *self.downloader_status.write().unwrap() = status;

                if let Some(task) = task
                    && let Err(e) = task.stop().await
                {
                    error!(error = ?e, "Failed to stop downloader");
                }
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
