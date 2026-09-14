use crate::server::common::recording_policy;
use crate::server::common::sync::SyncSession;
use crate::server::common::upload::UploaderMessage;
use crate::server::common::util::FileValidator;
use crate::server::core::downloader::cover_downloader;
use crate::server::core::downloader::{
    DanmakuClient, DownloadStatus, DownloaderRuntime, SegmentEvent, SegmentInfo,
};
use crate::server::core::live::{danmaku_client, downloader_runtime, live_request};
use crate::server::core::monitor::Monitor;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::{Context, Stage, WorkerStatus};
use crate::server::infrastructure::models::hook_step::process;
use async_channel::Sender;
use biliup::downloader::live::{LivePlugin, LiveStatus, LiveStream};
use error_stack::ResultExt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Notify};
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

        // 上一轮 process_with_upload 可能因上传失败提前返回，UActor 已 drop rx，
        // 这里挂着的 tx 是死的；丢弃后下面会重建一条新的管道。
        if let Some(tx) = &self.channel
            && tx.is_closed()
        {
            warn!(
                url = self.ctx.live_streamer().url,
                "upload channel closed by uploader, reopening"
            );
            self.channel = None;
        }

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
    sync_session: Option<Arc<Mutex<SyncSession>>>,
}

impl DownloadTask {
    pub fn new(downloader: DownloaderRuntime) -> Self {
        let sync_session = matches!(&downloader, DownloaderRuntime::Sync(_))
            .then(|| Arc::new(Mutex::new(SyncSession::default())));
        Self {
            token: CancellationToken::new(),
            done_notify: Notify::new(),
            downloader,
            sync_session,
        }
    }

    pub(self) async fn execute(
        &self,
        ctx: &Context,
        sender: Sender<UploaderMessage>,
        plugin: Arc<dyn LivePlugin + Send + Sync>,
        rooms_handle: Arc<Monitor>,
    ) -> AppResult<()> {
        // 重试配置
        let mut retry_count = 0;
        let max_retries = 3; // 最大重试次数
        let base_delay = Duration::from_secs(2); // 基础延迟时间（2秒）
        let max_delay = Duration::from_secs(ctx.config().delay); // 最大延迟时间（60秒）
        let url = ctx.live_streamer().url.clone();
        let mut stream = ctx.live_stream().clone();
        let filename_prefix = ctx
            .live_streamer()
            .filename_prefix
            .clone()
            .or_else(|| ctx.config().filename_prefix.clone());
        let danmaku_client = danmaku_client(
            stream.danmaku.as_ref(),
            filename_prefix.as_deref(),
            &stream.name,
        );
        // 启动弹幕客户端
        if let Some(ref client) = danmaku_client {
            // 启动弹幕下载逻辑
            info!("Starting danmaku client for stream: {}", url);
            client.download().await?;
        }

        // 初始化组件
        let mut processor = SegmentEventProcessor::new(sender, ctx.clone());
        // 边录边传：记录已确认分 P 数，只有真正推进投稿才算“有进展”。
        // 否则持续性错误（如 cookie 失效、preupload 被拒）会在直播中零延迟热循环。
        let mut last_committed = match &self.sync_session {
            Some(session) => session.lock().await.committed_parts(),
            None => 0,
        };
        let result = loop {
            // 创建守卫确保清理
            // 创建事件处理器
            // 执行下载
            let components = self
                .download(&mut processor, ctx.clone(), danmaku_client.clone(), &stream)
                .await;

            // 失败原因只藏在结束时的 Debug 输出里会让用户以为下载器“什么都没做”
            // （典型：边录边传缺少上传模板、cookie 失效）。
            if let Err(e) = &components {
                error!(url = url, error = ?e, "下载流程出错");
            }
            info!("initialize_components completed: {url}");

            if self.token.is_cancelled() {
                info!(url = url, "task is cancelled");
                break components;
            }
            // 检查流状态
            match plugin.check_stream(live_request(ctx.worker())).await {
                Ok(LiveStatus::Live {
                    stream: next_stream,
                }) => {
                    stream = *next_stream;
                    info!(
                        url = url,
                        "Stream is still live, preparing to retry. attempt: {}", retry_count
                    );
                    // 边录边传只有分 P 有实际推进才重置退避。管线的多数失败路径
                    // （空流、分段过小）返回 Ok(StreamEnded)，不能以 Ok/Err 判断；
                    // 否则拉流持续失败会零延迟重跑登录、preupload。
                    // 非边录边传：仅在分段成功或干净结束时立即续录；DownloadStatus::Error
                    // 必须走指数退避，避免仍在直播时 0ns 热循环（issue #1682）。
                    let progressed = match &self.sync_session {
                        Some(session) => {
                            let committed = session.lock().await.committed_parts();
                            let progressed = committed > last_committed;
                            last_committed = committed;
                            progressed
                        }
                        None => download_attempt_progressed(&components),
                    };
                    if progressed {
                        retry_count = 0;
                    } else {
                        retry_count += 1;
                    }
                }
                Ok(LiveStatus::Offline) => {
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

            // 录制策略：条件已不成立就不再续录，把房间交回监控循环。
            // 少了这一步，下载器按边界收尾后循环会立刻重开一段，等于策略形同虚设。
            // 每轮都重新判定，对齐 Python 版每轮 `run()` 前重新调用 `should_record()`。
            //
            // 放在 check_stream 之后有两个原因：用得到刚刷新的房间标题；且能同时覆盖
            // 探测失败的分支——那条路径只加重试计数，否则会带着已失效的策略绕回去重录。
            if let Some(rejection) =
                recording_policy::reject_before_record(ctx.live_streamer(), &stream.title)
            {
                info!(url = url, reason = %rejection, "停止录制");
                break components;
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

            // 计算指数退避延迟: delay = base_delay * 2^retry_count（成功分段时为 0）
            let delay = retry_delay(retry_count, base_delay, max_delay);

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
        stream: &LiveStream,
    ) -> AppResult<DownloadStatus> {
        // 获取配置和主播信息
        let streamer = ctx.live_streamer();
        let download_config = ctx.download_config(stream);
        if let crate::server::core::downloader::DownloaderRuntime::Sync(sync) = &self.downloader {
            info!(
                page_url = streamer.url,
                stream_url = download_config.url,
                platform = stream.platform,
                "开始边录边传，已解析流直链"
            );
            return crate::server::common::sync::run_sync_pipeline(
                sync,
                self.token.clone(),
                &ctx,
                download_config,
                self.sync_session
                    .clone()
                    .expect("sync downloader must have a sync session"),
            )
            .await;
        }

        // 执行下载
        // let hook = processor.create_hook(danmaku_client.clone());
        let hook = |event| {
            match event {
                SegmentEvent::Start { .. } => {
                    warn!("Ignoring unexpected segment start event");
                }
                SegmentEvent::Segment(mut event) => {
                    // 分段时，获取到的是已下载的文件名
                    // 触发弹幕滚动保存
                    if let Some(ref client) = danmaku_client {
                        let danmaku_file_path = event.prev_file_path.with_extension("xml");
                        match client.rolling(&danmaku_file_path.display().to_string()) {
                            Ok(true) => event.danmaku_file_path = Some(danmaku_file_path),
                            Ok(false) => {}
                            Err(e) => error!("Danmaku rolling error: {}", e),
                        }
                    }
                    // 异步处理事件
                    // let processor = processor.clone();
                    if let Err(e) = processor.process(event) {
                        error!("Failed to process segment event: {}", e);
                    }
                }
            }
        };

        info!(
            page_url = streamer.url,
            stream_url = download_config.url,
            platform = stream.platform,
            suffix = download_config.suffix,
            "开始下载，已解析流直链"
        );

        let result = self
            .downloader
            .download(Box::new(hook), download_config)
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
        // 清理设总时限：取消已传导到录制阶段，正常应很快退出；
        // 边录边传会继续把已录分段传完并投稿，可能远超 30 秒，让它在后台收尾即可，
        // 不能让 stop 无限挂起拖住整个 worker。
        if tokio::time::timeout(Duration::from_secs(30), self.done_notify.notified())
            .await
            .is_err()
        {
            warn!("等待下载任务退出超时（30 秒），任务将在后台完成收尾，继续关闭流程");
        }
        Ok(())
    }
}


/// Whether a finished download attempt counts as progress for live-retry backoff.
///
/// Successful segment completion and a clean stream end may retry immediately
/// while the room is still live. Errors (and download `Err`s) must not reset the
/// counter — otherwise still-Live rooms spam `Retrying download in 0ns...`.
fn download_attempt_progressed(result: &AppResult<DownloadStatus>) -> bool {
    matches!(
        result,
        Ok(DownloadStatus::SegmentCompleted) | Ok(DownloadStatus::StreamEnded)
    )
}

/// Exponential backoff delay for the current `retry_count`.
///
/// `retry_count == 0` means "immediate next segment" (Duration::ZERO). Non-zero
/// counts use `base_delay * 2^retry_count`, capped at `max_delay`.
fn retry_delay(retry_count: u32, base_delay: Duration, max_delay: Duration) -> Duration {
    if retry_count == 0 {
        Duration::ZERO
    } else {
        (base_delay.saturating_mul(2_u32.saturating_pow(retry_count))).min(max_delay)
    }
}

/// 启动完整下载流程。
///
/// 只能由 `Monitor` 在取得下载池许可后调用；调用方必须把许可移动到同一个任务中，
/// 并持有到本函数返回，保证 `pool1_size` 是下载并发的唯一限制。
pub async fn start_download_workflow(
    downloader: Arc<dyn LivePlugin + Send + Sync>,
    ctx: Context,
    sender: Sender<UploaderMessage>,
    rooms_handle: Arc<Monitor>,
) {
    let task = Arc::new(DownloadTask::new(downloader_runtime(
        ctx.config().downloader,
        ctx.live_stream(),
    )));
    ctx.change_status(Stage::Download, WorkerStatus::Working(task.clone()))
        .await;

    tokio::spawn({
        let streamer_info = ctx.streamer_info();
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

    let _ = task.execute(&ctx, sender, downloader, rooms_handle).await;

    process(&[], &ctx.live_streamer().downloaded_processor).await;

    info!(
        "Download workflow completed {} => {:?}",
        ctx.live_streamer().url,
        ctx.status(Stage::Download)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::errors::AppError;
    use error_stack::Report;

    #[test]
    fn segment_completed_and_stream_ended_count_as_progress() {
        assert!(download_attempt_progressed(&Ok(DownloadStatus::SegmentCompleted)));
        assert!(download_attempt_progressed(&Ok(DownloadStatus::StreamEnded)));
    }

    #[test]
    fn download_error_and_err_do_not_count_as_progress() {
        assert!(!download_attempt_progressed(&Ok(DownloadStatus::Error(
            "Streamlink error: Some(1)".into()
        ))));
        assert!(!download_attempt_progressed(&Ok(DownloadStatus::Downloading)));
        let err: AppResult<DownloadStatus> =
            Err(Report::new(AppError::Custom("boom".into())));
        assert!(!download_attempt_progressed(&err));
    }

    #[test]
    fn retry_delay_is_zero_only_when_counter_reset() {
        let base = Duration::from_secs(2);
        let max = Duration::from_secs(60);
        assert_eq!(retry_delay(0, base, max), Duration::ZERO);
        assert_eq!(retry_delay(1, base, max), Duration::from_secs(4));
        assert_eq!(retry_delay(2, base, max), Duration::from_secs(8));
        assert_eq!(retry_delay(10, base, max), max);
    }
}
