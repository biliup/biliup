use crate::server::common::live_image::spawn_avatar_download;
use crate::server::common::recording_policy;
use crate::server::common::sync::SyncSession;
use crate::server::common::throughput::{RateMeter, Sampler};
use crate::server::common::upload::UploaderMessage;
use crate::server::common::util::FileValidator;
use crate::server::core::downloader::cover_downloader;
use crate::server::core::downloader::ws_expire::resolve_ws_expire_override;
use crate::server::core::downloader::{
    DanmakuClient, DownloadStatus, DownloaderRuntime, SegmentEvent, SegmentInfo,
};
use crate::server::core::live::{danmaku_client, downloader_runtime, live_request};
use crate::server::core::monitor::Monitor;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::{Context, Stage, WorkerStatus};
use crate::server::infrastructure::models::hook_step::process;
use crate::server::workbench::index;
use crate::server::workbench::recorder::{
    ClosedSegment, RecorderHandle, SessionRecorder, SessionTarget,
};
use async_channel::Sender;
use biliup::downloader::live::{LivePlugin, LiveStatus, LiveStream, strip_ws_expire_override};
use biliup::downloader::preview::PreviewHub;
use danmaku_client::DanmakuEvent;
use error_stack::ResultExt;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::{Mutex, Notify, broadcast};
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

    /// 这个分段交给 [`Self::process`] 后会不会被过滤删除。
    pub fn will_discard(&self, path: &Path) -> bool {
        self.file_validator.will_delete(path)
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

/// 正在录制的直播间的封面与头像地址，供界面透出。
///
/// 只放在内存里，随下载任务消亡；每次 `check_stream` 拿到新的流信息就刷新一遍。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LiveMedia {
    pub cover_url: Option<String>,
    pub avatar_url: Option<String>,
}

/// 正在录制的那一路流的来源：平台名与 CDN 直链。供浏览器直连模式（`preview_transport = direct`）
/// 判定能力、下发直链；随每次 `check_stream` 刷新（换直链 / 重试后是新的）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveSource {
    pub platform: String,
    pub url: String,
}

impl LiveSource {
    pub fn from_stream(stream: &LiveStream) -> Self {
        Self {
            platform: stream.platform.clone(),
            url: stream.raw_stream_url.clone(),
        }
    }
}

impl LiveMedia {
    pub fn from_stream(stream: &LiveStream) -> Self {
        let non_empty = |s: &str| (!s.trim().is_empty()).then(|| s.to_string());
        Self {
            cover_url: non_empty(&stream.live_cover_url),
            avatar_url: stream.avatar_url.as_deref().and_then(non_empty),
        }
    }
}

/// 每路直播同时允许的预览连接数（信号量上限，超出返回 429）。
///
/// 与前端监视器的默认同屏路数无关：监视器每路占其对应直播间的一个连接。
pub const PREVIEW_MAX_SUBSCRIBERS_PER_ROOM: usize = 4;

/// 下载任务
pub struct DownloadTask {
    token: CancellationToken,
    done_notify: Notify,
    downloader: DownloaderRuntime,
    sync_session: Option<Arc<Mutex<SyncSession>>>,
    /// 写盘速率表；各下载器拿它的计数器句柄累加，采样任务随 `execute` 启停。
    meter: Arc<RateMeter>,
    media: std::sync::RwLock<LiveMedia>,
    /// 当前拉的那条直链与平台名，随 `check_stream` 刷新
    source: std::sync::RwLock<LiveSource>,
    /// 直播预览 hub，寿命与本任务相同：跨分段、跨断流重试都是同一个。
    preview: PreviewHub,
    /// 实时弹幕广播：弹幕客户端每解出一条就 `send` 一份给预览播放器；
    /// 平台没有弹幕实现（`stream.danmaku` 为 None）时为 `None`。
    danmaku_tx: Option<broadcast::Sender<DanmakuEvent>>,
    /// 追加 `expire=0` 的网宿直链探测通过、下载器却一个字节没拿到就失败过：
    /// 本任务余下的拉流不再追加，直接用原直链（stream-gears 有自己的首连兜底，不受影响）。
    ws_expire_rejected: AtomicBool,
}

/// 每路实时弹幕广播的槽位数；掉队的订阅者跳过丢掉的那几条继续收，不断开。
pub const DANMAKU_BROADCAST_CAPACITY: usize = 256;

impl DownloadTask {
    pub fn new(downloader: DownloaderRuntime, stream: &LiveStream) -> Self {
        let sync_session = matches!(&downloader, DownloaderRuntime::Sync(_))
            .then(|| Arc::new(Mutex::new(SyncSession::default())));
        let preview = preview_hub_for(&downloader);
        let danmaku_tx = stream
            .danmaku
            .as_ref()
            .map(|_| broadcast::channel(DANMAKU_BROADCAST_CAPACITY).0);
        Self {
            token: CancellationToken::new(),
            done_notify: Notify::new(),
            downloader,
            sync_session,
            meter: Arc::new(RateMeter::new()),
            media: std::sync::RwLock::new(LiveMedia::from_stream(stream)),
            source: std::sync::RwLock::new(LiveSource::from_stream(stream)),
            preview,
            danmaku_tx,
            ws_expire_rejected: AtomicBool::new(false),
        }
    }

    /// 当前正在录制的那条流的平台名与 CDN 直链。
    pub fn live_source(&self) -> LiveSource {
        self.source.read().unwrap().clone()
    }

    /// 本任务的直播预览 hub。
    pub fn preview(&self) -> &PreviewHub {
        &self.preview
    }

    /// 这一路有没有弹幕客户端（平台实现了弹幕且该流带弹幕源）。
    pub fn danmaku_available(&self) -> bool {
        self.danmaku_tx.is_some()
    }

    /// 订阅实时弹幕；没有弹幕客户端的平台返回 `None`。
    pub fn subscribe_danmaku(&self) -> Option<broadcast::Receiver<DanmakuEvent>> {
        self.danmaku_tx.as_ref().map(|tx| tx.subscribe())
    }

    /// 最近一个滑动窗口内的写盘速率（字节/秒）。
    ///
    /// 边录边传与 yt-dlp 的字节不经过计数器，报告 `None` 而不是一个恒为 0 的假数。
    pub fn bytes_per_sec(&self) -> Option<u64> {
        match &self.downloader {
            DownloaderRuntime::Sync(_) | DownloaderRuntime::YtDlp(_) => None,
            _ => self.meter.bytes_per_sec(),
        }
    }

    /// 本任务的写盘速率表（测试里用来喂采样）。
    #[cfg(test)]
    pub(crate) fn rate_meter(&self) -> &RateMeter {
        &self.meter
    }

    /// 当前直播间的封面与头像地址。
    pub fn live_media(&self) -> LiveMedia {
        self.media.read().unwrap().clone()
    }

    fn refresh_media(&self, ctx: &Context, stream: &LiveStream) {
        *self.source.write().unwrap() = LiveSource::from_stream(stream);
        let media = LiveMedia::from_stream(stream);
        let changed = self.media.read().unwrap().avatar_url != media.avatar_url;
        if changed {
            spawn_avatar_download(
                ctx.live_streamer().id,
                media.avatar_url.clone(),
                ctx.live_streamer().url.clone(),
            );
        }
        *self.media.write().unwrap() = media;
    }

    pub(self) async fn execute(
        &self,
        ctx: &Context,
        sender: Sender<UploaderMessage>,
        plugin: Arc<dyn LivePlugin + Send + Sync>,
        rooms_handle: Arc<Monitor>,
    ) -> AppResult<()> {
        // 速率采样任务与本次录制同寿命，句柄 drop 时随之停止
        let _sampler = Sampler::spawn(self.meter.clone());
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
        // 切片工作台的场次 / 分段记录，与本次下载任务同寿命；下面提前返回时 drop 也会收尾。
        // 媒体字节经过本进程写盘的下载器边写边建关键帧索引，其余的关段后扫盘
        let live_index = matches!(
            self.downloader,
            DownloaderRuntime::StreamGears(_) | DownloaderRuntime::Mesio(_)
        )
        .then(index::live::spawn);
        let workbench = SessionRecorder::spawn(
            ctx.pool().clone(),
            SessionTarget {
                session_id: ctx.id(),
                streamer_id: ctx.live_streamer().id,
                bytes: match &self.downloader {
                    DownloaderRuntime::Sync(_) | DownloaderRuntime::YtDlp(_) => None,
                    _ => Some(self.meter.counter()),
                },
            },
            live_index,
        );
        let danmaku_client = danmaku_client(
            stream.danmaku.as_ref(),
            filename_prefix.as_deref(),
            &stream.name,
            self.danmaku_tx.clone(),
        );
        // 启动弹幕客户端
        if let Some(ref client) = danmaku_client {
            // 启动弹幕下载逻辑
            info!("Starting danmaku client for stream: {}", url);
            client.download().await?;
        }

        if let Some(session) = &self.sync_session {
            session.lock().await.set_recorder(workbench.handle());
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
            let bytes_before = self.meter.counter().total();
            let components = self
                .download(
                    &mut processor,
                    ctx.clone(),
                    danmaku_client.clone(),
                    &stream,
                    workbench.handle(),
                )
                .await;
            if !matches!(self.downloader, DownloaderRuntime::StreamGears(_))
                && ws_expire_override_failed(
                    &stream.raw_stream_url,
                    &components,
                    self.meter.counter().total() > bytes_before,
                )
                && !self.ws_expire_rejected.swap(true, Ordering::Relaxed)
            {
                warn!(
                    url = url,
                    "追加 expire=0 的直链没拉到数据，本次录制余下的拉流改用原直链"
                );
            }

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
                    self.refresh_media(ctx, &stream);
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
        // 场次的 ended_at 要在房间交回监控循环之前写好，很快再开播时才能接上这一场
        if tokio::time::timeout(Duration::from_secs(30), workbench.finish())
            .await
            .is_err()
        {
            warn!(url = url, "切片工作台场次收尾超时，转入后台完成");
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
        workbench: RecorderHandle,
    ) -> AppResult<DownloadStatus> {
        workbench.run_started();
        // 获取配置和主播信息
        let streamer = ctx.live_streamer();
        let mut download_config = ctx.download_config(stream);
        // stream-gears 在自己的首连里处理网宿 403，其它下载器拉流前先探一次
        if !matches!(self.downloader, DownloaderRuntime::StreamGears(_)) {
            download_config.url = match strip_ws_expire_override(&download_config.url) {
                Some(original) if self.ws_expire_rejected.load(Ordering::Relaxed) => {
                    original.to_string()
                }
                _ => {
                    resolve_ws_expire_override(download_config.url, &download_config.headers).await
                }
            };
        }
        download_config.bytes_written = self.meter.counter();
        download_config.preview = self.preview.clone();
        download_config.index_tap = workbench.index_tap();
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
                SegmentEvent::Start { next_file_path } => {
                    workbench.opened(&next_file_path);
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
                    workbench.closed(
                        &event.prev_file_path,
                        ClosedSegment {
                            duration_ms: event
                                .duration_secs
                                .filter(|d| d.is_finite() && *d > 0.0)
                                .map(|d| (d * 1000.0).round() as u64),
                            bytes: event.size_bytes,
                            danmaku_path: event.danmaku_file_path.clone(),
                            discard: processor.will_discard(&event.prev_file_path),
                        },
                    );
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

/// 按下载器类型建预览 hub：媒体字节经过本进程写盘的（stream-gears / mesio）能旁路；
/// 子进程直接落盘的，以及边录边传，明确标为不可预览并给出原因，界面据此禁用按钮。
fn preview_hub_for(downloader: &DownloaderRuntime) -> PreviewHub {
    match downloader {
        DownloaderRuntime::StreamGears(_) | DownloaderRuntime::Mesio(_) => {
            PreviewHub::new(PREVIEW_MAX_SUBSCRIBERS_PER_ROOM)
        }
        DownloaderRuntime::Ffmpeg(_) => {
            PreviewHub::unavailable("ffmpeg 子进程直接写盘，媒体数据不经过 biliup，无法预览")
        }
        DownloaderRuntime::StreamLink(_) => {
            PreviewHub::unavailable("streamlink 子进程直接写盘，媒体数据不经过 biliup，无法预览")
        }
        DownloaderRuntime::YtDlp(_) => PreviewHub::unavailable(
            "yt-dlp / ytarchive 子进程直接写盘，媒体数据不经过 biliup，无法预览",
        ),
        DownloaderRuntime::Sync(_) => PreviewHub::unavailable("边录边传走投稿管线，不提供预览"),
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

/// 用追加 `expire=0` 的网宿直链拉流（探测已通过）却没写出任何字节就失败：
/// 说明网宿对下载器的请求与对 HEAD 探测的判断不一致，不能再信探测结果。
fn ws_expire_override_failed(
    stream_url: &str,
    result: &AppResult<DownloadStatus>,
    wrote_bytes: bool,
) -> bool {
    strip_ws_expire_override(stream_url).is_some()
        && !wrote_bytes
        && !download_attempt_progressed(result)
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
    let task = Arc::new(DownloadTask::new(
        downloader_runtime(ctx.config().downloader, ctx.live_stream()),
        ctx.live_stream(),
    ));
    ctx.change_status(Stage::Download, WorkerStatus::Working(task.clone()))
        .await;

    // 主播头像几乎不变：开播时下载一次存到 data/avatar/，地址没变就不再下载
    spawn_avatar_download(
        ctx.live_streamer().id,
        task.live_media().avatar_url,
        ctx.live_streamer().url.clone(),
    );

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
    fn ws_expire_override_counts_as_failed_only_without_data() {
        let overridden = "https://ws1a.douyucdn.cn/live/a.flv?wsAuth=a&expire=300&fcdn=ws&expire=0";
        let original = "https://ws1a.douyucdn.cn/live/a.flv?wsAuth=a&expire=300&fcdn=ws";
        let failed = Ok(DownloadStatus::Error("FFmpeg error: Some(8)".into()));
        let err: AppResult<DownloadStatus> = Err(Report::new(AppError::Unknown));

        assert!(ws_expire_override_failed(overridden, &failed, false));
        assert!(ws_expire_override_failed(overridden, &err, false));
        assert!(!ws_expire_override_failed(overridden, &failed, true));
        assert!(!ws_expire_override_failed(
            overridden,
            &Ok(DownloadStatus::StreamEnded),
            false
        ));
        assert!(!ws_expire_override_failed(original, &failed, false));
    }

    #[test]
    fn segment_completed_and_stream_ended_count_as_progress() {
        assert!(download_attempt_progressed(&Ok(
            DownloadStatus::SegmentCompleted
        )));
        assert!(download_attempt_progressed(&Ok(
            DownloadStatus::StreamEnded
        )));
    }

    #[test]
    fn download_error_and_err_do_not_count_as_progress() {
        assert!(!download_attempt_progressed(&Ok(DownloadStatus::Error(
            "Streamlink error: Some(1)".into()
        ))));
        assert!(!download_attempt_progressed(&Ok(
            DownloadStatus::Downloading
        )));
        let err: AppResult<DownloadStatus> = Err(Report::new(AppError::Custom("boom".into())));
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
