//! mesio 下载器：以库（API）方式接入 rust-srec 的 `mesio-engine` 与修复管线，
//! 在进程内完成 FLV/HLS 拉流、时间戳修复、关键帧索引注入与按大小/时长分段，
//! 不依赖外部 `mesio` 二进制。
//!
//! 生命周期与其它下载器一致：`download` 阻塞到直播流结束 / 被 `stop` 取消 /
//! 录制时间范围到期，期间每关闭一个分段文件就触发一次 [`SegmentEvent::Segment`]。

use crate::server::common::construct_headers;
use crate::server::common::util::media_ext_from_url;
use crate::server::core::downloader::{
    self, DownloadConfig, DownloadStatus, SegmentEvent, SegmentInfo,
};
use crate::server::errors::{AppError, AppResult};
use biliup::downloader::util::ByteCounter;
use error_stack::Report;
use flv_fix::{
    ContinuityMode, FlvPipeline, FlvPipelineConfig, FlvWriter, FlvWriterConfig, ScriptFillerConfig,
};
use futures::{Stream, StreamExt, stream};
use hls::HlsData;
use hls_fix::{HlsPipeline, HlsPipelineConfig, HlsWriter, HlsWriterConfig};
use mesio_engine::flv::FlvProtocolConfig;
use mesio_engine::{
    DownloadEvent, DownloadRequest, DownloadSession, DownloaderConfig, DownloaderSession,
    FlvRequestOptions, HlsProtocolBuilder, HlsRequestOptions, MesioConfig, MesioDownloader,
    ProtocolSelection, ProtocolType,
};
use pipeline_common::config::PipelineConfig;
use pipeline_common::{
    CancellationToken, ChannelSpec, PipelineError, PipelineProvider, ProtocolWriter,
    RunCompletionError, SpawnedPipeline, StreamerContext, WriterStats, settle_run, spawn_pipeline,
};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tracing::{debug, info, warn};

/// 管线内部通道容量（条目数），与 mesio-cli 默认值一致。
const CHANNEL_SIZE: usize = 64;

/// 分段回调载荷：已关闭的分段文件路径与 0 起始的序号。
type SegmentClosed = (PathBuf, u32);

/// mesio 下载器实例。可跨多次 `download` 复用，每次调用使用新的取消令牌。
pub struct Mesio {
    token: RwLock<CancellationToken>,
}

impl Default for Mesio {
    fn default() -> Self {
        Self::new()
    }
}

impl Mesio {
    pub fn new() -> Self {
        Self {
            token: RwLock::new(CancellationToken::new()),
        }
    }

    /// 开始下载，直到流结束、被取消或录制时间范围到期。
    pub(crate) async fn download<'a>(
        &self,
        mut callback: Box<dyn FnMut(SegmentEvent) + Send + Sync + 'a>,
        download_config: DownloadConfig,
    ) -> AppResult<DownloadStatus> {
        let token = CancellationToken::new();
        *self.token.write().unwrap() = token.clone();

        let url = download_config.url.clone();
        let headers = construct_headers(&download_config.headers).map_err(AppError::Custom)?;

        let base = DownloaderConfig::builder()
            .with_headers(headers)
            .with_caching_enabled(false)
            .with_system_proxy(true)
            .build();
        let flv_config = FlvProtocolConfig::builder()
            .with_base_config(base.clone())
            .build();
        let hls_config = HlsProtocolBuilder::new()
            .with_base_config(base)
            .get_config();
        let engine = MesioDownloader::new(MesioConfig {
            flv: flv_config,
            hls: hls_config,
            token: token.clone(),
        });

        let protocol = select_protocol(&url);
        info!(url = %url, protocol = ?protocol_name(&protocol), "mesio 开始拉流");
        let request = DownloadRequest::from_url(&url)
            .map_err(|e| Report::new(AppError::Custom(format!("mesio 无法解析流地址: {e}"))))?
            .with_protocol(protocol)
            .with_cancel(token.clone());
        let session = engine
            .start(request)
            .await
            .map_err(|e| Report::new(AppError::Custom(format!("mesio 拉流失败: {e}"))))?;

        let pipeline_config = pipeline_config(&download_config);
        let output_dir = download_config.output_dir.clone();
        let base_name = download_config.recorder.filename_template();
        let (seg_tx, seg_rx) = unbounded_channel::<SegmentClosed>();

        // 录制时间范围：管线只负责按时长切片而不会自行退出，到点后主动取消，
        // 与 ffmpeg 内部分段用 `-t` 截停的语义一致。
        let window_timer = download_config.time_range_remaining().map(|remaining| {
            let secs = downloader::parse_duration(&remaining);
            let token = token.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(secs)).await;
                info!("录制时间范围结束，停止 mesio 拉流");
                token.cancel();
            })
        });

        let outcome = match session {
            DownloaderSession::Flv(session) => {
                warn_on_suffix_mismatch(&download_config.suffix, "flv");
                let mut writer = FlvWriter::new(FlvWriterConfig {
                    output_dir,
                    base_name,
                });
                writer.set_on_segment_complete_callback(segment_complete_hook(seg_tx));
                let keyframe_duration_ms = pipeline_config
                    .max_duration
                    .map(|d| u32::try_from(d.as_millis()).unwrap_or(u32::MAX));
                let flv_pipeline_config = FlvPipelineConfig::builder()
                    .duplicate_tag_filtering(false)
                    .continuity_mode(ContinuityMode::Reset)
                    .keyframe_index_config(Some(match keyframe_duration_ms {
                        Some(keyframe_duration_ms) => ScriptFillerConfig {
                            keyframe_duration_ms,
                        },
                        None => ScriptFillerConfig::default(),
                    }))
                    .build();
                let DownloadSession {
                    items,
                    events,
                    handle,
                } = session;
                let event_task = tokio::spawn(log_events(events));
                let items = items.map(|r| r.map_err(|e| PipelineError::Strategy(Box::new(e))));
                let outcome = run_pipeline::<FlvPipeline, _>(
                    &pipeline_config,
                    flv_pipeline_config,
                    Box::pin(items),
                    ChannelSpec::items(CHANNEL_SIZE),
                    writer,
                    seg_rx,
                    token.clone(),
                    callback.as_mut(),
                    (download_config.bytes_written.clone(), |item| item.size()),
                )
                .await;
                handle.cancel();
                event_task.abort();
                outcome
            }
            DownloaderSession::Hls(session) => {
                let DownloadSession {
                    items,
                    events,
                    handle,
                } = session;
                let event_task = tokio::spawn(log_events(events));
                let mut items = items;
                // 先取第一个分片以确定容器类型（TS 或 fMP4），再把它放回流中
                let first = match items.next().await {
                    Some(Ok(segment)) => segment,
                    Some(Err(e)) => {
                        handle.cancel();
                        event_task.abort();
                        abort_timer(window_timer);
                        return Ok(finish(&token, Err(format!("获取首个 HLS 分片失败: {e}"))));
                    }
                    None => {
                        handle.cancel();
                        event_task.abort();
                        abort_timer(window_timer);
                        return Ok(finish(&token, Err("HLS 流为空".to_string())));
                    }
                };
                let extension = hls_extension(&first);
                warn_on_suffix_mismatch(&download_config.suffix, extension);
                let mut writer = HlsWriter::new(HlsWriterConfig {
                    output_dir,
                    base_name,
                    extension: extension.to_string(),
                    max_file_size: (pipeline_config.max_file_size > 0)
                        .then_some(pipeline_config.max_file_size),
                });
                writer.set_on_segment_complete_callback(segment_complete_hook(seg_tx));
                let items = stream::once(async { Ok(first) })
                    .chain(items)
                    .map(|r| r.map_err(|e| PipelineError::Strategy(Box::new(e))));
                let outcome = run_pipeline::<HlsPipeline, _>(
                    &pipeline_config,
                    HlsPipelineConfig::default(),
                    Box::pin(items),
                    HlsPipeline::channel_spec(CHANNEL_SIZE),
                    writer,
                    seg_rx,
                    token.clone(),
                    callback.as_mut(),
                    (download_config.bytes_written.clone(), |item| item.size()),
                )
                .await;
                handle.cancel();
                event_task.abort();
                outcome
            }
        };

        abort_timer(window_timer);
        Ok(finish(&token, outcome))
    }

    /// 停止下载：取消当前令牌，拉流与管线随之收尾并关闭最后一个分段。
    pub(crate) async fn stop(&self) -> AppResult<()> {
        self.token.read().unwrap().cancel();
        Ok(())
    }
}

/// 把管线结果映射为下载状态。取消（用户停止 / 时间范围到期）不算错误。
fn finish(token: &CancellationToken, outcome: Result<WriterStats, String>) -> DownloadStatus {
    match outcome {
        Ok(stats) => {
            info!(
                files = stats.files_created,
                bytes = stats.bytes_written,
                duration_secs = stats.duration_secs,
                cancelled = token.is_cancelled(),
                "mesio 拉流结束"
            );
            if token.is_cancelled() {
                DownloadStatus::SegmentCompleted
            } else {
                DownloadStatus::StreamEnded
            }
        }
        Err(_) if token.is_cancelled() => DownloadStatus::SegmentCompleted,
        Err(message) => DownloadStatus::Error(format!("mesio error: {message}")),
    }
}

fn abort_timer(timer: Option<tokio::task::JoinHandle<()>>) {
    if let Some(timer) = timer {
        timer.abort();
    }
}

/// 把 `segment_time` / `file_size` 翻译为管线限制；0 或未设置表示不限制。
/// `segment_time` 会按录制时间范围裁短（见 [`DownloadConfig::segment_duration`]）。
fn pipeline_config(download_config: &DownloadConfig) -> PipelineConfig {
    let mut builder = PipelineConfig::builder()
        .max_file_size(download_config.file_size.unwrap_or(0))
        .channel_size(CHANNEL_SIZE);
    if let Some(segment) = download_config.segment_duration() {
        let secs = downloader::parse_duration(&segment);
        if secs > 0 {
            builder = builder.max_duration(Duration::from_secs(secs));
        }
    }
    builder.build()
}

/// mesio 自身只按 `.flv` / `.m3u8` 路径后缀识别协议；这里补上 biliup 的扩展名推断
/// （含 query 里的 format/type 参数），识别不出时按 FLV 处理，由引擎再核对 Content-Type。
fn select_protocol(url: &str) -> ProtocolSelection {
    match media_ext_from_url(url).as_deref() {
        Some("m3u8" | "m3u" | "ts" | "m4s") => ProtocolSelection::Hls(HlsRequestOptions::default()),
        Some("flv") => ProtocolSelection::Flv(FlvRequestOptions::default()),
        _ => match MesioDownloader::detect_protocol(url) {
            Ok(ProtocolType::Hls) => ProtocolSelection::Hls(HlsRequestOptions::default()),
            _ => ProtocolSelection::Flv(FlvRequestOptions::default()),
        },
    }
}

fn protocol_name(protocol: &ProtocolSelection) -> &'static str {
    match protocol {
        ProtocolSelection::Hls(_) => "hls",
        ProtocolSelection::Flv(_) => "flv",
        ProtocolSelection::Auto => "auto",
    }
}

/// HLS 输出容器扩展名。fMP4 分片拼接（init + fragments）即合法的分片 MP4，
/// 用 `mp4` 而非 `m4s`，后续分段校验与投稿都按 mp4 处理。
fn hls_extension(first: &HlsData) -> &'static str {
    match first {
        HlsData::M4sData(_) => "mp4",
        _ => "ts",
    }
}

/// mesio 不做转封装：用户配置的保存格式与实际容器不一致时只提示，不改扩展名。
fn warn_on_suffix_mismatch(configured: &str, actual: &str) {
    let configured = configured.trim_start_matches('.').to_ascii_lowercase();
    // `m3u8` 表示"HLS 流"，TS 与 fMP4 两种容器都算匹配
    let same = configured == actual || configured == "m3u8";
    if !same {
        warn!(
            configured,
            actual,
            "mesio 不支持转封装，将按实际容器 .{actual} 保存；如需 .{configured} 请改用 ffmpeg"
        );
    }
}

fn segment_complete_hook(
    seg_tx: UnboundedSender<SegmentClosed>,
) -> impl Fn(&std::path::Path, u32, f64, u64, Option<&pipeline_common::SplitReason>)
+ Send
+ Sync
+ 'static {
    move |path, index, duration_secs, size_bytes, reason| {
        info!(
            path = %path.display(),
            index,
            duration_secs,
            size_bytes,
            reason = ?reason,
            "mesio 分段完成"
        );
        let _ = seg_tx.send((path.to_path_buf(), index));
    }
}

/// 拉流 → 修复管线 → 写盘，同时把分段关闭事件回传给 biliup 的分段回调。
///
/// 结束条件：拉流结束或被取消后 `input_tx` 关闭 → 管线排空 → writer 关闭最后一个
/// 文件并退出 → writer 被 drop 使 `seg_rx` 断开，回调排空后收尾。
///
/// `byte_meter`：写盘速率计数器与「一个管线条目占多少字节」的取值函数。
/// 修复管线的 writer 来自外部 crate，拿不到逐条写出量，因此在条目进入管线前计数；
/// 修复只增删极少量 tag，与实际落盘量相差可忽略。
#[allow(clippy::too_many_arguments)]
async fn run_pipeline<'a, P, W>(
    common: &PipelineConfig,
    config: P::Config,
    items: Pin<Box<dyn Stream<Item = Result<P::Item, PipelineError>> + Send>>,
    spec: ChannelSpec<P::Item>,
    mut writer: W,
    mut seg_rx: UnboundedReceiver<SegmentClosed>,
    token: CancellationToken,
    callback: &mut (dyn FnMut(SegmentEvent) + Send + Sync + 'a),
    byte_meter: (ByteCounter, fn(&P::Item) -> usize),
) -> Result<WriterStats, String>
where
    P: PipelineProvider,
    W: ProtocolWriter<Item = P::Item>,
{
    let context = Arc::new(StreamerContext::new(token));
    let provider = P::with_config(context, common, config);
    let SpawnedPipeline {
        input_tx,
        output_rx,
        tasks,
    } = spawn_pipeline(provider.build_pipeline(), spec);

    let writer_task = tokio::task::spawn_blocking(move || writer.run(output_rx));

    let forward = tokio::spawn(async move {
        let (bytes_written, item_size) = byte_meter;
        let mut items = items;
        while let Some(item) = items.next().await {
            if let Ok(item) = &item {
                bytes_written.add(item_size(item) as u64);
            }
            if input_tx.send(item).await.is_err() {
                debug!("mesio 管线已关闭，停止转发");
                break;
            }
        }
    });

    while let Some((path, index)) = seg_rx.recv().await {
        callback(SegmentEvent::Segment(SegmentInfo::new(
            path,
            None,
            None,
            index as usize,
        )));
    }

    if let Err(e) = forward.await {
        warn!(error = %e, "mesio 转发任务异常退出");
    }
    let writer_result = match writer_task.await {
        Ok(result) => result,
        Err(e) => return Err(format!("writer task panicked: {e}")),
    };
    match settle_run(writer_result, tasks).await {
        Ok(stats) => Ok(stats),
        Err(RunCompletionError::Writer(e)) => Err(format!("writer: {e}")),
        Err(RunCompletionError::Pipeline(e)) => Err(format!("pipeline: {e}")),
    }
}

/// 把引擎事件写进日志。进度事件只在 debug 级别记录，避免刷屏。
async fn log_events(mut events: mesio_engine::DownloadEventStream) {
    while let Some(event) = events.next().await {
        match event {
            DownloadEvent::Progress { .. } => debug!(?event, "mesio"),
            DownloadEvent::RetryScheduled { .. }
            | DownloadEvent::SegmentTimeout { .. }
            | DownloadEvent::GapSkipped { .. }
            | DownloadEvent::Lagged { .. } => warn!(?event, "mesio"),
            other => info!(event = ?other, "mesio"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn protocol_follows_media_extension_including_query_hints() {
        assert!(matches!(
            select_protocol("https://cdn.example/live/abc.flv?wsSecret=1"),
            ProtocolSelection::Flv(_)
        ));
        assert!(matches!(
            select_protocol("https://cdn.example/live/abc.m3u8?token=1"),
            ProtocolSelection::Hls(_)
        ));
        assert!(matches!(
            select_protocol("https://cdn.example/live/index?format=m3u8"),
            ProtocolSelection::Hls(_)
        ));
        assert!(matches!(
            select_protocol("https://cdn.example/live/playlist?x=1"),
            ProtocolSelection::Hls(_)
        ));
        // 无任何线索时按 FLV 处理，交给引擎核对 Content-Type
        assert!(matches!(
            select_protocol("https://cdn.example/live/stream"),
            ProtocolSelection::Flv(_)
        ));
    }

    #[test]
    fn fmp4_segments_are_saved_as_mp4_and_ts_as_ts() {
        assert_eq!(hls_extension(&HlsData::end_marker()), "ts");
        let m4s = HlsData::mp4_init(
            m3u8_rs::MediaSegment {
                uri: "init.mp4".to_string(),
                ..Default::default()
            },
            bytes::Bytes::new(),
        );
        assert_eq!(hls_extension(&m4s), "mp4");
    }

    #[test]
    fn segment_time_and_file_size_become_pipeline_limits() {
        let config = DownloadConfig {
            segment_time: Some("01:30:00".to_string()),
            file_size: Some(2_000_000_000),
            headers: HashMap::new(),
            ..Default::default()
        };
        let pipeline = pipeline_config(&config);
        assert_eq!(pipeline.max_file_size, 2_000_000_000);
        assert_eq!(pipeline.max_duration, Some(Duration::from_secs(5400)));

        let unlimited = pipeline_config(&DownloadConfig::default());
        assert_eq!(unlimited.max_file_size, 0);
        assert_eq!(unlimited.max_duration, None);
    }

    /// 端到端：本地 HTTP 服务吐一段真实 FLV 录像，走完整的引擎拉流 → 修复管线 → 落盘，
    /// 校验分段回调触发、按大小切片、文件名沿用 biliup 的模板。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn downloads_a_local_flv_through_the_engine_and_reports_segments() {
        use crate::server::common::util::Recorder;
        use crate::server::infrastructure::models::StreamerInfo;
        use axum::{Router, routing::get};
        use std::sync::Mutex;

        // 造一段合法 FLV：头 + 一个视频关键帧标签，重复多次以触发按大小分段
        let mut payload = Vec::new();
        payload.extend_from_slice(&[0x46, 0x4C, 0x56, 0x01, 0x01, 0, 0, 0, 9, 0, 0, 0, 0]);
        let data = vec![0x17u8, 0x01, 0, 0, 0, 0xAA, 0xBB, 0xCC, 0xDD];
        for i in 0..400u32 {
            let ts = i * 40;
            payload.push(9);
            payload.extend_from_slice(&(data.len() as u32).to_be_bytes()[1..]);
            payload.extend_from_slice(&(ts & 0xFF_FFFF).to_be_bytes()[1..]);
            payload.push((ts >> 24) as u8);
            payload.extend_from_slice(&[0, 0, 0]);
            payload.extend_from_slice(&data);
            payload.extend_from_slice(&((11 + data.len()) as u32).to_be_bytes());
        }
        let payload_len = payload.len();
        let body = bytes::Bytes::from(payload);

        let app = Router::new().route(
            "/live.flv",
            get(move || {
                let body = body.clone();
                async move { ([("content-type", "video/x-flv")], body) }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let dir = tempfile::tempdir().unwrap();
        let recorder = Recorder::new(
            Some("mesio-e2e-%Y%m%d".to_string()),
            StreamerInfo::new("t", "u", "title", chrono::Utc::now(), ""),
        );
        let config = DownloadConfig {
            url: format!("http://{addr}/live.flv"),
            segment_time: None,
            time_range: None,
            file_size: Some(4096),
            headers: HashMap::from([("User-Agent".to_string(), "biliup-test".to_string())]),
            recorder,
            output_dir: dir.path().to_path_buf(),
            suffix: "flv".to_string(),
            bytes_written: ByteCounter::new(),
        };
        let bytes_written = config.bytes_written.clone();

        let seen: Arc<Mutex<Vec<SegmentInfo>>> = Arc::default();
        let sink = seen.clone();
        let status = Mesio::new()
            .download(
                Box::new(move |event| {
                    if let SegmentEvent::Segment(info) = event {
                        sink.lock().unwrap().push(info);
                    }
                }),
                config,
            )
            .await
            .expect("download");

        assert_eq!(status, DownloadStatus::StreamEnded);
        assert_eq!(
            bytes_written.total(),
            payload_len as u64,
            "进入管线的字节应等于源 FLV 的长度（文件头 + 全部 tag）"
        );
        let seen = seen.lock().unwrap();
        assert!(
            seen.len() >= 2,
            "expected size-based splitting, got {seen:?}"
        );
        for (i, info) in seen.iter().enumerate() {
            assert_eq!(info.segment_index, i);
            let name = info.prev_file_path.file_name().unwrap().to_string_lossy();
            assert!(name.starts_with("mesio-e2e-20"), "{name}");
            assert!(name.ends_with(".flv"), "{name}");
            let size = std::fs::metadata(&info.prev_file_path).unwrap().len();
            assert!(size > 13, "segment {name} is empty");
        }
    }

    #[test]
    fn cancellation_is_a_normal_segment_end_not_an_error() {
        let token = CancellationToken::new();
        assert_eq!(
            finish(&token, Err("boom".into())),
            DownloadStatus::Error("mesio error: boom".into())
        );
        token.cancel();
        assert_eq!(
            finish(&token, Err("cancelled".into())),
            DownloadStatus::SegmentCompleted
        );
    }
}
