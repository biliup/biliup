use crate::server::common::download::DownloadTask;
use crate::server::common::upload::{execute_postprocessor, process_with_upload};
use crate::server::common::util::Recorder;
use crate::server::core::downloader::{Downloader, SegmentEvent, SegmentInfo};
use crate::server::core::monitor::{Monitor, RoomsHandle};
use crate::server::core::plugin::{DownloadPlugin, StreamInfoExt};
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::{Context, Stage, Worker, WorkerStatus};
use crate::server::infrastructure::models::hook_step::process_video;
use async_channel::{Receiver, Sender, bounded};
use biliup::bilibili::{BiliBili, Studio, Video};
use biliup::client::StatelessClient;
use biliup::credential::login_by_cookies;
use biliup::uploader::line::{Line, Probe};
use biliup::uploader::util::SubmitOption;
use biliup::uploader::{VideoFile, line};
use core::fmt;
use error_stack::ResultExt;
use futures::StreamExt;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use crate::server::infrastructure::connection_pool::ConnectionPool;

/// 下载管理器
/// 负责管理特定平台的下载任务，包括监控器和插件
pub struct DownloadManager {
    /// 监控器实例（可选，使用Mutex保护）
    pub monitor: Mutex<Option<Arc<Monitor>>>,
    /// 下载插件
    plugin: Arc<dyn DownloadPlugin + Send + Sync>,
    /// Actor处理器
    actor_handle: Arc<ActorHandle>,
}

impl DownloadManager {
    /// 创建新的下载管理器实例
    ///
    /// # 参数
    /// * `plugin` - 下载插件实现
    /// * `actor_handle` - Actor处理器
    pub fn new(
        plugin: impl DownloadPlugin + Send + Sync + 'static,
        actor_handle: Arc<ActorHandle>,
    ) -> Self {
        Self {
            monitor: Mutex::new(None),
            plugin: Arc::new(plugin),
            actor_handle,
        }
    }

    /// 确保监控器存在，如果不存在则创建新的
    ///
    /// # 返回
    /// 返回监控器的Arc引用
    pub fn ensure_monitor(&self, pool: ConnectionPool) -> Arc<Monitor> {
        self.monitor
            .lock()
            .unwrap()
            .get_or_insert_with(|| {
                Arc::new(Monitor::new(self.plugin.clone(), self.actor_handle.clone(), pool))
            })
            .clone()
    }

    /// 检查URL是否匹配此下载管理器的插件
    ///
    /// # 参数
    /// * `url` - 要检查的URL
    ///
    /// # 返回
    /// 如果URL匹配返回true，否则返回false
    pub fn matches(&self, url: &str) -> bool {
        self.plugin.matches(url)
    }
}

impl fmt::Debug for DownloadManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DownloadManager [{:?}]", self.monitor)
    }
}

/// 下载Actor
/// 负责处理下载相关的消息和任务
pub struct DActor {
    /// 下载消息接收器
    receiver: Receiver<DownloaderMessage>,
    /// 上传消息发送器
    sender: Sender<UploaderMessage>,
}

impl DActor {
    /// 创建新的下载Actor实例
    pub fn new(receiver: Receiver<DownloaderMessage>, sender: Sender<UploaderMessage>) -> Self {
        Self { receiver, sender }
    }

    /// 运行Actor主循环，处理接收到的消息
    async fn run(&mut self) {
        while let Ok(msg) = self.receiver.recv().await {
            if let Err(e) = self.handle_message(msg).await {
                error!("Error handling message: {}", e);
            }
        }
    }

    /// 处理下载消息
    ///
    /// # 参数
    /// * `msg` - 要处理的下载消息
    async fn handle_message(&mut self, msg: DownloaderMessage) -> AppResult<()> {
        match msg {
            DownloaderMessage::Start(plugin, ctx, rooms_handle) => {
                // 创建下载任务
                let task = DownloadTask::new(plugin, ctx, rooms_handle);

                // 执行下载（使用 Result 链式处理）
                task.execute(&self.sender).await?;

                Ok(())
            }
        }
    }
}
/// 上传Actor
/// 负责处理上传相关的消息和任务
pub struct UActor {
    /// 上传消息接收器
    receiver: Receiver<UploaderMessage>,
}

impl UActor {
    /// 创建新的上传Actor实例
    pub fn new(receiver: Receiver<UploaderMessage>) -> Self {
        Self { receiver }
    }

    /// 运行Actor主循环，处理接收到的消息
    async fn run(&mut self) {
        while let Ok(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
    }

    /// 处理上传消息
    ///
    /// # 参数
    /// * `msg` - 要处理的上传消息
    async fn handle_message(&mut self, msg: UploaderMessage) {
        match msg {
            UploaderMessage::SegmentEvent(rx, ctx) => {
                ctx.worker
                    .change_status(Stage::Upload, WorkerStatus::Pending);
                let result = match ctx.worker.get_upload_config() {
                    Some(config) => process_with_upload(rx, &ctx, config).await,
                    None => {
                        let mut paths = Vec::new();
                        while let Ok(event) = rx.recv().await {
                            paths.push(event.prev_file_path);
                        }
                        // 无上传配置时，直接执行后处理
                        execute_postprocessor(paths, &ctx).await
                    }
                };

                if let Err(e) = result {
                    error!("Process segment event failed: {}", e);
                    // 可以添加错误通知机制
                }
                ctx.worker.change_status(Stage::Upload, WorkerStatus::Idle);
            }
        }
    }
}

/// Actor处理器
/// 管理下载和上传Actor的生命周期
pub struct ActorHandle {
    /// 下载信号量数量
    download_semaphore: u32,
    /// 上传信号量数量
    update_semaphore: u32,
    /// 上传消息发送器
    pub up_sender: Sender<UploaderMessage>,
    /// 下载消息发送器
    pub down_sender: Sender<DownloaderMessage>,
    /// 下载Actor任务句柄列表
    pub(crate) d_kills: Vec<JoinHandle<()>>,
    /// 上传Actor任务句柄列表
    pub(crate) u_kills: Vec<JoinHandle<()>>,
}

impl ActorHandle {
    /// 创建新的Actor处理器实例
    ///
    /// # 参数
    /// * `download_semaphore` - 下载Actor数量
    /// * `update_semaphore` - 上传Actor数量
    pub fn new(download_semaphore: u32, update_semaphore: u32) -> Self {
        // 创建消息通道
        let (up_tx, up_rx) = bounded(16);
        let (down_tx, down_rx) = bounded(1);
        let mut d_kills = Vec::new();
        let mut u_kills = Vec::new();
        // 创建下载Actor
        for _ in 0..download_semaphore {
            let mut d_actor = DActor::new(down_rx.clone(), up_tx.clone());
            let d_kill = tokio::spawn(async move { d_actor.run().await });
            d_kills.push(d_kill)
        }
        // 创建上传Actor
        for _ in 0..update_semaphore {
            let mut u_actor = UActor::new(up_rx.clone());
            let u_kill = tokio::spawn(async move { u_actor.run().await });
            u_kills.push(u_kill)
        }

        Self {
            download_semaphore,
            update_semaphore,
            up_sender: up_tx,
            down_sender: down_tx,
            d_kills,
            u_kills,
        }
    }
}

/// 上传消息枚举
/// 定义上传Actor可以处理的消息类型
#[derive(Debug)]
pub enum UploaderMessage {
    /// 分段事件消息，包含事件、接收器和工作器
    SegmentEvent(Receiver<SegmentInfo>, Context),
}

/// 下载消息枚举
/// 定义下载Actor可以处理的消息类型
pub enum DownloaderMessage {
    /// 开始下载消息，包含插件、流信息、上下文和房间句柄
    Start(
        Arc<dyn DownloadPlugin + Send + Sync>,
        Context,
        Arc<RoomsHandle>,
    ),
}

impl Drop for ActorHandle {
    fn drop(&mut self) {
        // 发送端随 ActorHandle 一起被 drop，会关闭通道（如果没有其他 sender 克隆）。
        // 为避免 tokio 任务在后台“挂着”，这里直接 abort。
        for h in &self.d_kills {
            h.abort();
        }
        for h in &self.u_kills {
            h.abort();
        }
    }
}
