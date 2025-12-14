use crate::server::common::upload::UploaderMessage;
use crate::server::common::util::FileValidator;
use crate::server::config::default_segment_time;
use crate::server::core::downloader::cover_downloader;
use crate::server::core::downloader::{
    DanmakuClient, DownloadConfig, DownloadStatus, DownloaderRuntime, DownloaderType, SegmentEvent,
    SegmentInfo,
};
use crate::server::core::monitor::Monitor;
use crate::server::core::plugin::{DownloadBase, StreamInfoExt, StreamStatus};
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::{Context, Stage, WorkerStatus};
use crate::server::infrastructure::models::hook_step::process;
use async_channel::{Receiver, Sender};
use error_stack::{ResultExt, bail};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
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

/// 分段事件处理器
pub struct SegmentEventProcessor {
    channel: Option<Sender<SegmentInfo>>,
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
                ctx.config().filtering_threshold * 1000 * 1000,
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
                self.channel = Some(tx);
            }
            Some(tx) => {
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
    token: CancellationToken,
    done_notify: Notify,
    downloader: DownloaderRuntime,
}

impl DownloadTask {
    pub fn new(downloader: DownloaderRuntime) -> Self {
        Self {
            token: CancellationToken::new(),
            done_notify: Notify::new(),
            downloader,
        }
    }

    pub(self) async fn execute(
        &self,
        ctx: &Context,
        sender: Sender<UploaderMessage>,
        mut plugin: Box<dyn DownloadBase>,
        rooms_handle: Arc<Monitor>,
    ) -> AppResult<()> {
        // 重试配置
        let mut retry_count = 0;
        let max_retries = 3; // 最大重试次数
        let base_delay = Duration::from_secs(2); // 基础延迟时间（2秒）
        let max_delay = Duration::from_secs(ctx.config().delay); // 最大延迟时间（60秒）
        let url = ctx.live_streamer().url.clone();
        let mut stream_info_ext = ctx.stream_info_ext().clone();
        // 可选的弹幕客户端
        // 初始化弹幕客户端（如果存在）
        let danmaku_client = plugin.danmaku_init();
        // 启动弹幕客户端
        if let Some(ref client) = danmaku_client {
            // 启动弹幕下载逻辑
            info!("Starting danmaku client for stream: {}", url);
            client.download().await?;
        }

        // 初始化组件
        let mut processor = SegmentEventProcessor::new(sender, ctx.clone());
        let result = loop {
            // 创建守卫确保清理
            // 创建事件处理器
            // 执行下载
            let components = self
                .download(
                    &mut processor,
                    ctx.clone(),
                    danmaku_client.clone(),
                    &stream_info_ext,
                )
                .await;

            info!("initialize_components completed: {url}");

            if self.token.is_cancelled() {
                info!(url = url, "task is cancelled");
                break components;
            }
            // 检查流状态
            match plugin.check_stream().await {
                Ok(StreamStatus::Live { stream_info }) => {
                    stream_info_ext = *stream_info;
                    info!(
                        url = url,
                        "Stream is still live, preparing to retry. attempt: {}", retry_count
                    );
                    // 成功下载后重置计数
                    retry_count = 0;
                }
                Ok(StreamStatus::Offline) => {
                    retry_count += 1;
                    // 继续循环，重新执行下载
                    info!(url = url, "Stream went offline, stopping download");
                }
                Err(e) => {
                    retry_count += 1;
                    // 继续循环，重新执行下载
                    warn!(
                        url = url,
                        "Failed to check stream status: {:?}, stopping download", e
                    );
                }
            }

            if retry_count >= max_retries {
                warn!(
                    url = url,
                    "Maximum retry attempts ({}) reached, stopping", max_retries
                );
                break components;
            }

            info!(
                url = url,
                "preparing to retry. Attempt: {}/{}",
                retry_count + 1,
                max_retries
            );

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
        // 异步清理任务
        if let Some(client) = danmaku_client.clone()
            && let Err(e) = client.stop().await
        {
            error!("Error stopping danmaku client: {}", e);
        }
        // 清理资源
        // 确保状态更新和资源清理
        rooms_handle.wake_waker(ctx.worker_id()).await;
        info!("Download task completed: {:?}", result);
        self.done_notify.notify_one();
        Ok(())
    }

    async fn download(
        &self,
        processor: &mut SegmentEventProcessor,
        ctx: Context,
        danmaku_client: Option<Arc<dyn DanmakuClient + Send + Sync>>,
        ext: &StreamInfoExt,
    ) -> AppResult<DownloadStatus> {
        // 获取配置和主播信息
        let streamer = ctx.live_streamer();

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
            .download(Box::new(hook), ctx.download_config(ext))
            .await
            .change_context(AppError::Custom("Failed to download segment".into()))?;

        // 处理结果
        info!(url=streamer.url,result=?result, "finished downloading");
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

    rooms_handle: Arc<Monitor>,
}

impl DActor {
    /// 创建新的下载Actor实例
    pub fn new(
        receiver: Receiver<DownloaderMessage>,
        sender: Sender<UploaderMessage>,
        rooms_handle: Arc<Monitor>,
    ) -> Self {
        Self {
            receiver,
            sender,
            rooms_handle,
        }
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
            DownloaderMessage::Start(downloader, ctx) => {
                // 创建下载任务
                let task = Arc::new(DownloadTask::new(
                    downloader.downloader(
                        ctx.config()
                            .downloader
                            .unwrap_or(DownloaderType::StreamGears),
                    ),
                ));
                // 更新工作器状态为工作中
                ctx.change_status(Stage::Download, WorkerStatus::Working(task.clone()))
                    .await;

                tokio::spawn({
                    let streamer_info = &ctx.stream_info_ext().streamer_info;
                    let live_cover_url = streamer_info.live_cover_path.clone();
                    let format_filename = ctx.recorder(streamer_info.clone()).format_filename();
                    let client = ctx.stateless_client().client.clone();
                    let enabled = ctx
                        .config()
                        .use_live_cover
                        .map(|u| u && !live_cover_url.is_empty())
                        .unwrap_or(false);
                    async move {
                        cover_downloader::download_cover_with(
                            &live_cover_url,
                            enabled,
                            &format_filename,
                            client,
                        )
                        .await
                    }
                });

                process(&[], &ctx.live_streamer().preprocessor).await;

                let _ = task
                    .execute(
                        &ctx,
                        self.sender.clone(),
                        downloader,
                        self.rooms_handle.clone(),
                    )
                    .await;

                process(&[], &ctx.live_streamer().downloaded_processor).await;

                info!(
                    "Download workflow completed {} => {:?}",
                    ctx.live_streamer().url,
                    ctx.status(Stage::Download)
                );
            }
        }
    }
}

/// 下载消息枚举
/// 定义下载Actor可以处理的消息类型
pub enum DownloaderMessage {
    /// 开始下载消息，包含插件、流信息、上下文和房间句柄
    Start(Box<dyn DownloadBase>, Context),
}
