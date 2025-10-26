use crate::server::common::upload::UploaderMessage;
use crate::server::core::downloader::{DownloadStatus, Downloader, SegmentEvent, SegmentInfo};
use crate::server::core::monitor::RoomsHandle;
use crate::server::core::plugin::{DownloadPlugin, StreamStatus};
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::{Context, Worker, WorkerStatus};
use async_channel::{Receiver, Sender};
use error_stack::{ResultExt, bail, Report};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

// Configuration and retry policy
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl RetryPolicy {
    pub fn exponential(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
        }
    }
}

/// 下载任务守卫，确保资源清理
pub struct DownloadGuard {
    worker: Arc<Worker>,
    danmaku_client: Option<Arc<dyn Downloader>>,
    rooms_handle: Arc<RoomsHandle>,
}

impl DownloadGuard {
    fn new(
        worker: Arc<Worker>,
        danmaku_client: Option<Arc<dyn Downloader>>,
        rooms_handle: Arc<RoomsHandle>,
        downloader: Arc<dyn Downloader + Send + Sync>,
    ) -> Self {
        Self {
            worker,
            danmaku_client,
            rooms_handle,
        }
    }

    async fn cleanup(&mut self) {
        // 异步清理任务
        let danmaku = self.danmaku_client.clone();
        let rooms_handle = self.rooms_handle.clone();
        let worker = self.worker.clone();

        if let Some(client) = danmaku
            && let Err(e) = client.stop().await
        {
            error!("Error stopping danmaku client: {}", e);
        }

        // 确保状态更新和资源清理
        rooms_handle
            .toggle(worker.clone(), WorkerStatus::Idle)
            .await;
        info!(
            "{} => {:?}",
            worker.live_streamer.url,
            worker.downloader_status.read().await
        );
    }
}

impl Drop for DownloadGuard {
    fn drop(&mut self) {
        let worker = self.worker.clone();
        info!("{}完成资源清理", worker.live_streamer.url);
    }
}

/// 文件验证配置
#[derive(Clone)]
pub struct FileValidator {
    min_size: u64,
    check_format: bool,
}

impl FileValidator {
    pub fn new(min_size: u64, check_format: bool) -> Self {
        Self {
            min_size,
            check_format,
        }
    }
}

impl Default for FileValidator {
    fn default() -> Self {
        Self {
            min_size: 1024 * 1024 * 100, // 100MB minimum
            check_format: true,
        }
    }
}

impl FileValidator {
    /// 验证文件有效性
    pub fn validate(&self, path: &Path) -> AppResult<()> {
        let metadata = fs::metadata(path).change_context(AppError::Unknown)?;

        let size = metadata.len();

        if size < self.min_size {
            bail!(AppError::Custom(format!(
                "File {} too small: {size} bytes, minimum: {} bytes",
                path.display(),
                self.min_size
            )));
        }

        // 可选：检查文件格式
        if self.check_format {
            self.validate_format(path)?;
        }

        Ok(())
    }

    fn validate_format(&self, path: &Path) -> AppResult<()> {
        // 简单的格式验证 - 检查扩展名
        if let Some(extension) = path.extension() {
            let ext = extension.to_string_lossy().to_lowercase();
            match ext.as_str() {
                "mp4" | "flv" | "ts" | "m3u8" => Ok(()),
                _ => bail!(AppError::Custom(format!("Unsupported format: {}", ext))),
            }
        } else {
            bail!(AppError::Custom("No file extension found".to_string()))
        }
    }
}

/// 分段事件处理器
#[derive(Clone)]
pub struct SegmentEventProcessor {
    tx: Sender<SegmentInfo>,
    rx: Receiver<SegmentInfo>,
    uploader: Sender<UploaderMessage>,
    ctx: Context,
    file_validator: FileValidator,
}

impl SegmentEventProcessor {
    /// 创建处理器
    pub fn new(uploader: Sender<UploaderMessage>, ctx: Context) -> Self {
        let (tx, rx) = async_channel::bounded(32); // Use tokio channel for async

        Self {
            tx,
            rx,
            uploader,
            file_validator: FileValidator::new(
                ctx.worker
                    .clone()
                    .config
                    .read()
                    .unwrap()
                    .filtering_threshold
                    * 1000
                    * 1000,
                true,
            ),
            ctx,
        }
    }

    /// 处理分段事件
    pub fn process(&self, event: SegmentInfo) -> AppResult<()> {
        // 验证文件有效性
        self.file_validator.validate(&event.prev_file_path)?;
        if event.segment_index == 0 {
            // 发送到上传器
            let res = self
                .uploader
                .force_send(UploaderMessage::SegmentEvent(
                    self.rx.clone(),
                    self.ctx.clone(),
                ))
                .change_context(AppError::Custom("Failed to send to uploader".to_string()))?;
            if let Some(prev) = res {
                warn!(SegmentEvent = ?prev, "replace an existing message in the channel");
            }
        }
        // 发送到缓冲区
        let res = self
            .tx
            .force_send(event)
            .change_context(AppError::Custom("Failed to send to buffer".to_string()))?;
        if let Some(prev) = res {
            warn!(SegmentEvent = ?prev, "replace an existing message in the channel");
        }
        Ok(())
    }

    /// 创建事件钩子
    pub fn create_hook(
        &self,
        danmaku: Option<Arc<dyn Downloader>>,
    ) -> impl Fn(SegmentEvent) + Clone + use<> {
        let processor = self.clone();

        move |event| {
            match event {
                SegmentEvent::Start { next_file_path } => {
                    unreachable!("应没有任何位置发出此事件");
                    // 开始下载时，获取到的是将要下载的文件名，此时文件还未生成
                    // 触发弹幕滚动保存
                    if let Some(ref client) = danmaku
                        && let Err(e) = client
                            .rolling(&next_file_path.with_extension("xml").display().to_string())
                    {
                        error!("Danmaku rolling error: {}", e);
                    }
                }
                SegmentEvent::Segment(event) => {
                    // 分段时，获取到的是已下载的文件名
                    // 触发弹幕滚动保存
                    if let Some(ref client) = danmaku
                        && let Err(e) = client.rolling(
                            &event
                                .prev_file_path
                                .with_extension("xml")
                                .display()
                                .to_string(),
                        )
                    {
                        error!("Danmaku rolling error: {}", e);
                    }
                    // 异步处理事件
                    let processor = processor.clone();
                    if let Err(e) = processor.process(event) {
                        error!("Failed to process segment event: {}", e);
                    }
                }
            }
        }
    }
}

/// 下载任务
pub struct DownloadTask {
    plugin: Arc<dyn DownloadPlugin + Send + Sync>,
    ctx: Context,
    rooms_handle: Arc<RoomsHandle>,
}

struct DownloadComponents {
    downloader: Arc<dyn Downloader + Send + Sync>,
    danmaku_client: Option<Arc<dyn Downloader>>,
    uploader: Sender<UploaderMessage>,
}

impl DownloadTask {
    pub fn new(
        plugin: Arc<dyn DownloadPlugin + Send + Sync>,
        ctx: Context,
        rooms_handle: Arc<RoomsHandle>,
    ) -> Self {
        Self {
            plugin,
            ctx,
            rooms_handle,
        }
    }

    pub(self) async fn execute(self, components: DownloadComponents) -> AppResult<DownloadStatus> {
        // 创建事件处理器
        let processor = SegmentEventProcessor::new(components.uploader.clone(), self.ctx.clone());

        // 启动弹幕客户端
        if let Some(ref client) = components.danmaku_client {
            self.start_danmaku(client).await?;
        }

        // 执行下载
        let hook = processor.create_hook(components.danmaku_client.clone());
        let result = components
            .downloader
            .download(Box::new(hook))
            .await
            .change_context(AppError::Custom("Failed to download segment".into()))?;

        // 处理结果
        info!(result=?result, "finished downloading");
        Ok(result)
    }

    async fn initialize_components(&self, uploader: Sender<UploaderMessage>) -> DownloadComponents {
        // 获取配置和主播信息
        let config = self.ctx.worker.get_config();
        let streamer = self.ctx.worker.get_streamer();
        let stream_info = &self.ctx.stream_info;

        // 可选的弹幕客户端
        // 初始化弹幕客户端（如果存在）
        let danmaku_client = self
            .ctx
            .extension
            .get::<Arc<dyn Downloader>>()
            .map(Arc::clone);

        // 创建下载器实例
        let downloader = self
            .plugin
            .create_downloader(stream_info, config, self.ctx.recorder.clone())
            .await;

        DownloadComponents {
            downloader,
            danmaku_client,
            uploader,
        }
    }

    async fn start_danmaku(&self, client: &Arc<dyn Downloader>) -> AppResult<()> {
        // 启动弹幕下载逻辑
        info!(
            "Starting danmaku client for stream: {}",
            self.ctx.stream_info.streamer_info.url
        );
        client.download(Box::new(|_| {})).await?;
        Ok(())
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
    pub(crate) async fn run(&mut self) {
        while let Ok(msg) = self.receiver.recv().await {
            self.handle_message(msg).await
        }
    }

    /// 处理下载消息
    ///
    /// # 参数
    /// * `msg` - 要处理的下载消息
    async fn handle_message(&mut self, msg: DownloaderMessage) {
        match msg {
            DownloaderMessage::Start(plugin, mut ctx, rooms_handle) => {
                let worker = ctx.worker.clone();
                // 创建下载任务
                let task = DownloadTask::new(plugin.clone(), ctx.clone(), rooms_handle.clone());

                // 初始化组件
                let components = task.initialize_components(self.sender.clone()).await;
                // 创建守卫确保清理
                let mut guard = DownloadGuard::new(
                    worker.clone(),
                    components.danmaku_client.clone(),
                    rooms_handle.clone(),
                    components.downloader.clone(),
                );

                // 更新工作器状态为工作中
                rooms_handle
                    .toggle(worker, WorkerStatus::Working(components.downloader.clone()))
                    .await;

                // 执行下载
                let result = task.execute(components).await;

                match plugin.check_status(&mut ctx).await {
                    Ok(StreamStatus::Live {stream_info}) => {
                        ctx.stream_info.raw_stream_url = stream_info.raw_stream_url;
                        println!("stream_info=?stream_info");
                    }
                    Ok(StreamStatus::Offline) => {}
                    Ok(StreamStatus::Unknown) => {}
                    Err(_) => {}
                }

                guard.cleanup().await;
                info!("Handling message: {:?} done", result);
            }
        }
    }
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
