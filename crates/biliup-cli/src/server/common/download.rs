use crate::server::common::upload::UploaderMessage;
use crate::server::config::default_segment_time;
use crate::server::core::downloader::{
    DownloadConfig, DownloadStatus, Downloader, SegmentEvent, SegmentInfo,
};
use crate::server::core::monitor::RoomsHandle;
use crate::server::core::plugin::{DownloadPlugin, StreamStatus};
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::{Context, Worker, WorkerStatus};
use crate::server::infrastructure::models::hook_step::{process, process_video};
use async_channel::{Receiver, Sender};
use error_stack::{Report, ResultExt, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
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
pub struct SegmentEventProcessor {
    channel: Option<(Sender<SegmentInfo>, Receiver<SegmentInfo>)>,
    uploader: Sender<UploaderMessage>,
    ctx: Context,
    file_validator: FileValidator,
}

impl SegmentEventProcessor {
    /// 创建处理器
    pub fn new(uploader: Sender<UploaderMessage>, ctx: Context) -> Self {
        Self {
            channel: None,
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
    pub fn process(&mut self, event: SegmentInfo) -> AppResult<()> {
        // 验证文件有效性
        self.file_validator.validate(&event.prev_file_path)?;
        match &self.channel {
            None => {
                let (tx, rx) = async_channel::bounded(32); // Use tokio channel for async

                // 发送到上传器
                let res = self
                    .uploader
                    .force_send(UploaderMessage::SegmentEvent(rx.clone(), self.ctx.clone()))
                    .change_context(AppError::Custom("Failed to send to uploader".to_string()))?;
                if let Some(prev) = res {
                    warn!(SegmentEvent = ?prev, "replace an existing message in the channel");
                }

                // 发送到缓冲区
                let res = tx
                    .force_send(event)
                    .change_context(AppError::Custom("Failed to send to buffer".to_string()))?;
                if let Some(prev) = res {
                    warn!(SegmentEvent = ?prev, "replace an existing message in the channel");
                }
                self.channel = Some((tx, rx));
            }
            Some((tx, rx)) => {
                // 发送到缓冲区
                let res = tx
                    .force_send(event)
                    .change_context(AppError::Custom("Failed to send to buffer".to_string()))?;
                if let Some(prev) = res {
                    warn!(SegmentEvent = ?prev, "replace an existing message in the channel");
                }
            }
        }

        Ok(())
    }
}

/// 下载任务
pub struct DownloadTask {
    plugin: Arc<dyn DownloadPlugin + Send + Sync>,
    token: CancellationToken,
    done_notify: Notify,
    downloader: Arc<dyn Downloader>,
}

impl DownloadTask {
    pub fn new(
        plugin: Arc<dyn DownloadPlugin + Send + Sync>,
        downloader: Arc<dyn Downloader>,
    ) -> Self {
        Self {
            plugin,
            token: CancellationToken::new(),
            done_notify: Notify::new(),
            downloader,
        }
    }

    pub(self) async fn execute(
        &self,
        mut ctx: Context,
        mut processor: SegmentEventProcessor,
        rooms_handle: &RoomsHandle,
    ) -> AppResult<DownloadStatus> {
        let worker = ctx.worker.clone();
        // 重试配置
        let mut retry_count = 0;
        let max_retries = 5; // 最大重试次数
        let base_delay = Duration::from_secs(0); // 基础延迟时间（2秒）
        let max_delay = Duration::from_secs(worker.get_config().delay); // 最大延迟时间（60秒）
        let result = loop {
            // 创建守卫确保清理
            // 创建事件处理器
            // 执行下载
            let components = self
                .initialize_components(&mut processor, ctx.clone())
                .await;
            info!("Download task completed: {:?}", components);
            ctx = ctx.clone();
            // 检查流状态
            match self.plugin.check_status(&mut ctx).await {
                Ok(StreamStatus::Live { stream_info }) => {
                    ctx.stream_info.raw_stream_url = stream_info.raw_stream_url;
                    info!(
                        "Stream is still live, preparing to retry. Attempt: {}/{}",
                        retry_count + 1,
                        max_retries
                    );

                    // 成功下载后重置计数
                    retry_count = 0;
                }
                Ok(StreamStatus::Offline) => {
                    retry_count += 1;
                    // 继续循环，重新执行下载
                    info!("Stream went offline, stopping download");
                }
                Ok(StreamStatus::Unknown) => {
                    retry_count += 1;
                    // 继续循环，重新执行下载
                    info!("Stream status unknown, stopping download");
                }
                Err(e) => {
                    retry_count += 1;
                    // 继续循环，重新执行下载
                    warn!("Failed to check stream status: {:?}, stopping download", e);
                }
            }

            if self.token.is_cancelled() {
                info!("用户手动停止");
                break components;
            }

            if retry_count >= max_retries {
                warn!("Maximum retry attempts ({}) reached, stopping", max_retries);
                break components;
            }

            // 计算指数退避延迟: delay = base_delay * 2^retry_count
            let delay = if retry_count != 0 {
                base_delay * 2_u32.pow(retry_count)
            } else {
                Duration::ZERO
            };
            let delay = delay.min(max_delay); // 限制最大延迟时间

            info!("Retrying download in {:?}...", delay);
            tokio::time::sleep(delay).await;
        };

        info!("Download task completed: {:?}", result);

        // 清理资源
        // 确保状态更新和资源清理
        rooms_handle
            .toggle(worker.clone(), WorkerStatus::Idle)
            .await;
        self.done_notify.notify_one();
        result
    }

    async fn initialize_components(
        &self,
        processor: &mut SegmentEventProcessor,
        ctx: Context,
    ) -> AppResult<DownloadStatus> {
        // 获取配置和主播信息
        let config = ctx.worker.get_config();
        let streamer = ctx.worker.get_streamer();
        let stream_info = &ctx.stream_info;
        let raw_stream_url = &stream_info.raw_stream_url;
        // 可选的弹幕客户端
        // 初始化弹幕客户端（如果存在）
        let danmaku_client = ctx.extension.get::<Arc<dyn Downloader>>().map(Arc::clone);

        let download_config = DownloadConfig {
            /// 流URL
            url: raw_stream_url.to_string(),
            segment_time: config.segment_time.or_else(default_segment_time),
            file_size: Some(config.file_size), // 2GB
            headers: stream_info.stream_headers.clone(),
            recorder: ctx.recorder.clone(),
            // output_dir: PathBuf::from("./downloads")
            output_dir: PathBuf::from("."),
        };
        // 启动弹幕客户端
        if let Some(ref client) = danmaku_client {
            // 启动弹幕下载逻辑
            info!(
                "Starting danmaku client for stream: {}",
                ctx.stream_info.streamer_info.url
            );
            client
                .download(Box::new(|_| {}), Default::default())
                .await?;
        }

        // 执行下载
        // let hook = processor.create_hook(danmaku_client.clone());
        let hook = |event| {
            match event {
                SegmentEvent::Start { next_file_path } => {
                    unreachable!("应没有任何位置发出此事件");
                    // 开始下载时，获取到的是将要下载的文件名，此时文件还未生成
                    // 触发弹幕滚动保存
                    // if let Some(ref client) = danmaku_client
                    //     && let Err(e) = client
                    //     .rolling(&next_file_path.with_extension("xml").display().to_string())
                    // {
                    //     error!("Danmaku rolling error: {}", e);
                    // }
                }
                SegmentEvent::Segment(event) => {
                    // 分段时，获取到的是已下载的文件名
                    // 触发弹幕滚动保存
                    if let Some(ref client) = danmaku_client
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
                    // let processor = processor.clone();
                    if let Err(e) = processor.process(event) {
                        error!("Failed to process segment event: {}", e);
                    }
                }
            }
        };
        let result = self
            .downloader
            .download(Box::new(hook), download_config)
            .await
            .change_context(AppError::Custom("Failed to download segment".into()))?;

        // 异步清理任务
        if let Some(client) = danmaku_client
            && let Err(e) = client.stop().await
        {
            error!("Error stopping danmaku client: {}", e);
        }
        // 处理结果
        info!(result=?result, "finished downloading");
        Ok(result)
    }

    pub(crate) async fn stop(&self) -> AppResult<()> {
        // 仅发出取消信号并更新状态
        // 如果底层下载函数不支持取消，这里不能真正中断正在进行的下载
        self.token.cancel();
        self.downloader.stop().await?;
        self.done_notify.notified().await;
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
            DownloaderMessage::Start(plugin, ctx, rooms_handle) => {
                let worker = ctx.worker.clone();

                // 创建下载器实例
                let downloader = plugin
                    .create_downloader(ctx.worker.get_config().downloader)
                    .await;
                // 创建下载任务
                let task = Arc::new(DownloadTask::new(plugin.clone(), downloader));
                // 初始化组件
                let processor = SegmentEventProcessor::new(self.sender.clone(), ctx.clone());
                // 更新工作器状态为工作中
                rooms_handle
                    .toggle(worker.clone(), WorkerStatus::Working(task.clone()))
                    .await;

                process(&[], &ctx.worker.get_streamer().preprocessor).await;

                let option = &ctx.worker.get_streamer().downloaded_processor;

                let result = task.execute(ctx, processor, &rooms_handle).await;

                process(&[], option).await;

                info!(
                    "Download workflow completed {} => {:?}",
                    worker.live_streamer.url,
                    worker.downloader_status.read().await
                );
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
