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
use crate::server::fleet::ha::UnitOutput;
use crate::server::infrastructure::context::{Context, Stage, WorkerStatus};
use crate::server::infrastructure::models::hook_step::process;
use crate::server::workbench::index;
use crate::server::workbench::recorder::{
    ClosedSegment, RecorderHandle, SessionRecorder, SessionTarget,
};
use crate::server::workbench::retention::Retention;
use async_channel::Sender;
use biliup::downloader::live::{LivePlugin, LiveStatus, LiveStream, strip_ws_expire_override};
use biliup::downloader::preview::PreviewHub;
use danmaku_client::DanmakuEvent;
use error_stack::ResultExt;
use futures::FutureExt;
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
    output: UnitOutput,
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
            )
            .with_retention(Retention::without_delay(ctx.pool().clone())),
            ctx,
            output: UnitOutput::default(),
        }
    }

    /// 录完的分段数与交给投稿流程的分段数
    pub fn output(&self) -> UnitOutput {
        self.output
    }

    /// 这个分段交给 [`Self::process`] 后会不会被过滤删除。
    pub fn will_discard(&self, path: &Path) -> bool {
        self.file_validator.will_delete(path)
    }

    /// 处理分段事件。`settled`：录制器处理完这次关段（过滤删除要等它）。
    pub fn process(
        &mut self,
        mut event: SegmentInfo,
        settled: impl Future<Output = ()> + Send + 'static,
    ) -> AppResult<()> {
        self.output.seen += 1;
        // 验证文件有效性
        let settled = settled.boxed().shared();
        let protected = crate::server::plugins::mosaic::masking_required(
            &self.ctx.config(),
            &self.ctx.live_streamer().override_cfg,
        ) || crate::server::plugins::mosaic::is_unmasked(&event.prev_file_path);
        if protected {
            self.file_validator
                .validate_without_size(&event.prev_file_path)?;
            event.ready = Some(crate::server::core::downloader::SegmentReady(settled));
        } else {
            self.file_validator
                .validate(&event.prev_file_path, settled)?;
        }

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
                // 不设上限：投稿流程可能整场都轮不到上传槽位（槽位由别的房间整场占着），
                // 有界通道满了 `force_send` 会挤掉最早的分段，那一段就再也不会投稿或后处理
                let (tx, rx) = async_channel::unbounded();

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
        self.output.sent += 1;

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

/// 每路直播同时允许的预览连接数（固定槽位；满了新打开的预览挤掉最早的一条，自动重连的返回 429）。
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
            // 弹幕是附带的：起不来（如插件接受、弹幕端不认的房间地址）也照常录像。
            // 用 `?` 直接返回会跳过下面的收尾，房间不再交回监控循环，直到重启都不会被检测
            if let Err(e) = client.download().await {
                error!(url = url, error = ?e, "弹幕客户端启动失败，本场只录像不录弹幕");
            }
        }

        if let Some(session) = &self.sync_session {
            let mut session = session.lock().await;
            session.set_recorder(workbench.handle());
            session.set_retention(Retention::after_upload(ctx.pool().clone(), &ctx.config()));
        }

        // 初始化组件
        let mut processor = SegmentEventProcessor::new(sender, ctx.clone());
        // 边录边传：记录已确认分 P 数，只有真正推进投稿才算“有进展”。
        // 否则持续性错误（如 cookie 失效、preupload 被拒）会在直播中零延迟热循环。
        let mut last_committed = match &self.sync_session {
            Some(session) => session.lock().await.committed_parts(),
            None => 0,
        };
        crate::server::fleet::ha::unit_started(ctx);
        let result = loop {
            // 创建守卫确保清理
            // 创建事件处理器
            // 执行下载
            let bytes_before = self.meter.counter().total();
            let components = {
                let attempt = self.download(
                    &mut processor,
                    ctx.clone(),
                    danmaku_client.clone(),
                    &stream,
                    workbench.handle(),
                );
                // 一主一备模式 2：备机接手了主机断网期间中断过的这一段，停掉这次拉流（没有配对时不会就绪）
                let stop = crate::server::fleet::ha::stop_requested(ctx);
                tokio::pin!(attempt, stop);
                tokio::select! {
                    biased;
                    components = &mut attempt => components,
                    () = &mut stop => {
                        info!(url = url, "一主一备：备机已接手这个房间，主机停止这次拉流");
                        if let Err(e) = self.downloader.stop().await {
                            warn!(url = url, error = ?e, "停止拉流失败");
                        }
                        attempt.await
                    }
                }
            };
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
            crate::server::fleet::events::recording_finished(ctx, &components, &self.token);
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

            // 复检开播期间被停止（暂停、删除房间、退出）：stop 时没有在跑的拉流可停，
            // 这里再开一次的话会一直录到分段结束甚至直播结束
            if self.token.is_cancelled() {
                info!(url = url, "task is cancelled");
                break components;
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
            tokio::select! {
                () = self.token.cancelled() => {
                    info!(url = url, "task is cancelled");
                    break components;
                }
                () = tokio::time::sleep(delay) => {}
            }
            if crate::server::fleet::ha::yield_recording(ctx) {
                info!(url = url, "一主一备：备机已接手这个房间，主机不续录");
                break components;
            }
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
        crate::server::fleet::ha::unit_ended(ctx, processor.output());
        if crate::server::fleet::ha::auto_clip_allowed(ctx) {
            crate::server::auto_clip::runner::session_finished(
                ctx.pool(),
                &ctx.config(),
                ctx.live_streamer(),
                ctx.id(),
            );
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
        if crate::server::plugins::mosaic::masking_required(&ctx.config(), &streamer.override_cfg) {
            // Put a durable raw marker in the basename before any byte is written.
            // A crash/restart or later config change must not make a `.part` raw
            // recording eligible for HA recovery uploads.
            let prefix = download_config
                .recorder
                .filename_prefix
                .as_deref()
                .unwrap_or("{streamer}%Y-%m-%dT%H_%M_%S");
            download_config.recorder.filename_prefix = Some(format!("{prefix}.unmasked"));
        }
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
            if crate::server::plugins::mosaic::masking_required(
                &ctx.config(),
                &streamer.override_cfg,
            ) {
                return Err(AppError::Custom(
                    "画面遮挡需要落盘处理，不能与边录边传同时启用；请选择 mesio、stream-gears 或 ffmpeg 下载器".into(),
                ).into());
            }
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
                    let protect = crate::server::plugins::mosaic::masking_required(
                        &ctx.config(),
                        &streamer.override_cfg,
                    );
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
                    // Rename the actual closed file before enqueueing it. A failed
                    // processor/restart must never leave raw media at a normal
                    // uploadable filename. All disk downloaders share this hook.
                    let quarantined = if protect {
                        match crate::server::plugins::mosaic::quarantine(&event.prev_file_path) {
                            Ok(path) => {
                                event.prev_file_path = path;
                                true
                            }
                            Err(e) => {
                                error!(path = ?event.prev_file_path, error = %e, "无法隔离未遮挡录像，本段停止投稿");
                                false
                            }
                        }
                    } else {
                        true
                    };
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
                    if quarantined && let Err(e) = processor.process(event, workbench.settled()) {
                        error!("Failed to process segment event: {}", e);
                    }
                }
            }
        };

        // 拉流前的探测（网宿 expire=0）期间可能已被停止：下载器还没启动，stop 停不到它
        if self.token.is_cancelled() {
            return Ok(DownloadStatus::StreamEnded);
        }

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
/// 只能由 `Monitor` 在取得下载池槽位后调用；调用方必须把槽位移动到同一个任务中，
/// 并持有到本函数返回，保证 `pool1_size` 是下载并发的唯一限制。
pub async fn start_download_workflow(
    downloader: Arc<dyn LivePlugin + Send + Sync>,
    ctx: Context,
    sender: Sender<UploaderMessage>,
    rooms_handle: Arc<Monitor>,
) {
    let task = Arc::new(DownloadTask::new(
        downloader_runtime(
            ctx.config().downloader,
            ctx.live_stream(),
            ctx.live_streamer().format.as_deref(),
        ),
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

    /// 投稿流程暂时没在收（上传池被别的房间整场占着、或上传比录制慢）时，分段要排队等着，
    /// 不能被新分段挤掉：挤掉的分段既不投稿也不跑后处理，只留下一行 warn。
    #[tokio::test]
    async fn segments_queue_up_while_the_uploader_is_not_reading() {
        use crate::server::config::Config;
        use crate::server::infrastructure::connection_pool::ConnectionManager;
        use crate::server::infrastructure::context::Worker;
        use crate::server::infrastructure::models::live_streamer::LiveStreamer;
        use biliup::downloader::live::DownloaderHint;
        use std::path::PathBuf;

        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let mut config = Config::default();
        config.filtering_threshold = 0;
        let streamer = LiveStreamer {
            id: 1,
            url: "https://queue.example/1".to_string(),
            remark: "queue".to_string(),
            filename_prefix: None,
            time_range: None,
            upload_streamers_id: None,
            format: None,
            override_cfg: None,
            preprocessor: None,
            segment_processor: None,
            downloaded_processor: None,
            postprocessor: None,
            opt_args: None,
            excluded_keywords: None,
        };
        let worker = Arc::new(Worker::new(
            streamer,
            None,
            Arc::new(std::sync::RwLock::new(config)),
            Default::default(),
        ));
        let stream = LiveStream {
            name: "queue".into(),
            url: "https://queue.example/1".into(),
            title: "t".into(),
            date: chrono::Utc::now(),
            live_cover_url: String::new(),
            avatar_url: None,
            raw_stream_url: "http://127.0.0.1:9/x.flv".into(),
            platform: "queue".into(),
            stream_headers: Default::default(),
            suffix: "flv".into(),
            danmaku: None,
            downloader_hint: DownloaderHint::StreamGears,
            runtime_options: None,
        };
        let ctx = Context::new(1, worker, pool, stream);
        let (uploader, uploads) = async_channel::unbounded();
        let mut processor = SegmentEventProcessor::new(uploader, ctx);

        let mut recorded = Vec::new();
        for i in 0..40 {
            let path = dir.path().join(format!("seg-{i}.flv"));
            std::fs::write(&path, b"flv").unwrap();
            processor
                .process(SegmentInfo::new(path.clone(), None, None, i), async {})
                .unwrap();
            recorded.push(path);
        }

        let UploaderMessage::SegmentEvent(segments, _) = uploads.try_recv().unwrap();
        let queued: Vec<PathBuf> = std::iter::from_fn(|| segments.try_recv().ok())
            .map(|segment| segment.prev_file_path)
            .collect();
        assert_eq!(queued, recorded, "每个录好的分段都要交给投稿流程");
    }

    /// 跑真实的 `DownloadTask::execute`：mesio 拉一个本地必然 404 的直链（首连即失败、不落盘），
    /// 续录前的开播复检由测试放行。
    mod execute {
        use super::*;
        use crate::server::config::Config;
        use crate::server::core::downloader::mesio::Mesio;
        use crate::server::core::slots::Slots;
        use crate::server::infrastructure::connection_pool::ConnectionManager;
        use crate::server::infrastructure::context::Worker;
        use crate::server::infrastructure::models::live_streamer::LiveStreamer;
        use async_trait::async_trait;
        use axum::Router;
        use axum::http::StatusCode;
        use axum::routing::get;
        use biliup::downloader::live::{DanmakuSource, DownloaderHint, LiveRequest, LiveResult};
        use std::collections::HashMap;
        use std::sync::atomic::AtomicUsize;
        use tokio::sync::mpsc;

        const ROOM: &str = "https://gated.example/1";

        /// 本地「CDN」：数拉流请求，一律 404
        async fn counting_cdn() -> (String, Arc<AtomicUsize>) {
            let hits = Arc::new(AtomicUsize::new(0));
            let counter = hits.clone();
            let app = Router::new().route(
                "/live.flv",
                get(move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                    async { StatusCode::NOT_FOUND }
                }),
            );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
            (format!("http://{addr}/live.flv"), hits)
        }

        /// 续录前的开播复检：先报给测试，等测试放行后报仍在直播
        struct GatedLive {
            stream: LiveStream,
            probing: mpsc::UnboundedSender<()>,
            gate: Arc<Notify>,
        }

        #[async_trait]
        impl LivePlugin for GatedLive {
            fn name(&self) -> &'static str {
                "gated"
            }

            fn matches(&self, url: &str) -> bool {
                url.starts_with("https://gated.example/")
            }

            async fn check_stream(&self, _request: LiveRequest) -> LiveResult<LiveStatus> {
                let _ = self.probing.send(());
                self.gate.notified().await;
                Ok(LiveStatus::Live {
                    stream: Box::new(self.stream.clone()),
                })
            }
        }

        struct Harness {
            task: Arc<DownloadTask>,
            run: tokio::task::JoinHandle<AppResult<()>>,
            probing: mpsc::UnboundedReceiver<()>,
            gate: Arc<Notify>,
            hits: Arc<AtomicUsize>,
            _dir: tempfile::TempDir,
        }

        /// `delay`：退避上限（秒），即 `Config::delay`
        async fn start(danmaku: Option<DanmakuSource>, delay: u64) -> Harness {
            let dir = tempfile::tempdir().unwrap();
            let db = dir.path().join("data.sqlite3");
            let pool = ConnectionManager::new_pool(db.to_str().unwrap())
                .await
                .unwrap();
            sqlx::query("INSERT INTO livestreamers (id, url, remark) VALUES (1, ?, 'gated')")
                .bind(ROOM)
                .execute(&pool)
                .await
                .unwrap();
            let (stream_url, hits) = counting_cdn().await;
            let stream = LiveStream {
                name: "gated".into(),
                url: ROOM.into(),
                title: "t".into(),
                date: chrono::Utc::now(),
                live_cover_url: String::new(),
                avatar_url: None,
                raw_stream_url: stream_url,
                platform: "gated".into(),
                stream_headers: HashMap::new(),
                suffix: "flv".into(),
                danmaku,
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            };
            let session = crate::server::workbench::store::open_session(
                &pool,
                1,
                &crate::server::core::live::streamer_info(&stream),
                crate::server::workbench::recorder::now_ms(),
                0,
            )
            .await
            .unwrap();
            let mut config = Config::default();
            config.delay = delay;
            let streamer = LiveStreamer {
                id: 1,
                url: ROOM.to_string(),
                remark: "gated".to_string(),
                filename_prefix: None,
                time_range: None,
                upload_streamers_id: None,
                format: None,
                override_cfg: None,
                preprocessor: None,
                segment_processor: None,
                downloaded_processor: None,
                postprocessor: None,
                opt_args: None,
                excluded_keywords: None,
            };
            let worker = Arc::new(Worker::new(
                streamer,
                None,
                Arc::new(std::sync::RwLock::new(config)),
                Default::default(),
            ));
            let ctx = Context::new(session.id, worker, pool.clone(), stream.clone());
            let (probing_tx, probing) = mpsc::unbounded_channel();
            let gate = Arc::new(Notify::new());
            let plugin = Arc::new(GatedLive {
                stream: stream.clone(),
                probing: probing_tx,
                gate: gate.clone(),
            });
            let (uploader, _) = async_channel::bounded(1);
            let monitor = Arc::new(Monitor::new(
                uploader.clone(),
                Arc::new(Slots::new(1)),
                pool,
            ));
            let task = Arc::new(DownloadTask::new(
                DownloaderRuntime::Mesio(Mesio::new()),
                &stream,
            ));
            let run = tokio::spawn({
                let task = task.clone();
                async move { task.execute(&ctx, uploader, plugin, monitor).await }
            });
            Harness {
                task,
                run,
                probing,
                gate,
                hits,
                _dir: dir,
            }
        }

        async fn first_recheck(h: &mut Harness) {
            let probed = tokio::time::timeout(Duration::from_secs(10), h.probing.recv()).await;
            assert!(
                matches!(probed, Ok(Some(()))),
                "拉流结束后应复检开播：{probed:?}"
            );
            assert_eq!(h.hits.load(Ordering::SeqCst), 1);
        }

        async fn stop(h: &Harness) -> tokio::task::JoinHandle<AppResult<()>> {
            let task = h.task.clone();
            let stop = tokio::spawn(async move { task.stop().await });
            tokio::time::timeout(Duration::from_secs(5), h.task.token.cancelled())
                .await
                .expect("stop 应立即取消任务");
            stop
        }

        async fn finished_within(h: Harness, limit: Duration) -> usize {
            tokio::time::timeout(limit, h.run)
                .await
                .expect("停止后下载任务应很快结束")
                .unwrap()
                .unwrap();
            h.hits.load(Ordering::SeqCst)
        }

        /// 停止发生在续录前的开播复检期间：复检报「仍在直播」后不能再开一次拉流。
        /// mesio / stream-gears 每次拉流换新令牌、ffmpeg 每次起新进程，stop 停不到这一次，
        /// 它会一直录到分段结束或直播结束，用户点的停止 / 暂停形同虚设。
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn stopping_during_the_live_recheck_starts_no_further_download() {
            let _role = crate::server::fleet::ha::test_guard().await;
            let mut h = start(None, 1).await;
            first_recheck(&mut h).await;

            let stop = stop(&h).await;
            h.gate.notify_one();

            let hits = finished_within(h, Duration::from_secs(10)).await;
            stop.await.unwrap().unwrap();
            assert_eq!(hits, 1, "停止之后不应再拉流");
        }

        /// 停止发生在失败重试的退避等待期间：立即结束，不等退避睡完再拉一次流。
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn stopping_during_the_retry_backoff_ends_the_task_at_once() {
            let _role = crate::server::fleet::ha::test_guard().await;
            // 第一次失败后退避 2s * 2 = 4s
            let mut h = start(None, 60).await;
            first_recheck(&mut h).await;
            h.gate.notify_one();
            tokio::time::sleep(Duration::from_millis(300)).await;

            let stop = stop(&h).await;
            let hits = finished_within(h, Duration::from_secs(2)).await;
            stop.await.unwrap().unwrap();
            assert_eq!(hits, 1, "停止之后不应再拉流");
        }

        /// 插件给了弹幕源、弹幕客户端却起不来（抖音插件接受 www.douyin.com 的房间地址，
        /// 弹幕端只认 live.douyin.com）：照常录像。不能让整个下载任务直接返回——那样既不录，
        /// 也不把房间交回监控循环，这个房间直到重启都不会再被检测。
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn a_danmaku_client_that_fails_to_start_does_not_abort_the_recording() {
            let _role = crate::server::fleet::ha::test_guard().await;
            let danmaku = DanmakuSource {
                platform: "douyin".into(),
                url: "https://www.douyin.com/user/someone".into(),
                room_id: None,
                cookie: None,
                raw: false,
                detail: false,
                extra: HashMap::new(),
                movie_id: None,
                password: None,
            };
            let mut h = start(Some(danmaku), 1).await;
            first_recheck(&mut h).await;

            let stop = stop(&h).await;
            h.gate.notify_one();
            finished_within(h, Duration::from_secs(10)).await;
            stop.await.unwrap().unwrap();
        }
    }
}
