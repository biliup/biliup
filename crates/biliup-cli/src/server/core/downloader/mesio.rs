//! mesio 下载器：以库（API）方式接入 rust-srec 的 `mesio-engine` 与修复管线，
//! 在进程内完成 FLV/HLS 拉流、时间戳修复、关键帧索引注入与按大小/时长分段，
//! 不依赖外部 `mesio` 二进制。
//!
//! 生命周期与其它下载器一致：`download` 阻塞到直播流结束 / 被 `stop` 取消 /
//! 录制时间范围到期，期间每打开一个分段文件触发一次 [`SegmentEvent::Start`]，
//! 每关闭一个触发一次 [`SegmentEvent::Segment`]。

mod index_tap;

use crate::server::common::construct_headers;
use crate::server::common::util::media_ext_from_url;
use crate::server::core::downloader::{
    self, DownloadConfig, DownloadStatus, SegmentEvent, SegmentInfo,
};
use crate::server::errors::{AppError, AppResult};
use biliup::downloader::preview::{self, ChunkKind, PreviewFormat, PreviewSink};
use biliup::downloader::util::ByteCounter;
use error_stack::Report;
use flv::{CodecKind, FlvData};
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

/// writer 线程回传的分段事件。开段与关段走同一条通道，回调看到的顺序与落盘顺序一致。
#[derive(Debug)]
enum WriterEvent {
    Opened(PathBuf),
    /// 已关闭的分段文件路径、0 起始的序号、时长（秒）与字节数。
    Closed(PathBuf, u32, f64, u64),
}

/// 直播预览旁路：写入端与「把一个管线条目旁路给它」的函数。
type PreviewTee<I> = (PreviewSink, fn(&mut PreviewSink, &I));

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
        let (seg_tx, seg_rx) = unbounded_channel::<WriterEvent>();

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
                let index_queue =
                    index_tap::install(&mut writer, seg_tx, download_config.index_tap.clone());
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
                let preview = download_config.preview.attach(PreviewFormat::Flv);
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
                    Some((preview, tee_flv)),
                    index_queue,
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
                // TS 分片给浏览器里的 mpegts.js；fMP4 分片直接交给 MediaSource
                let format = if first.is_ts() {
                    PreviewFormat::MpegTs
                } else {
                    PreviewFormat::Fmp4
                };
                let preview = Some((
                    download_config.preview.attach(format),
                    tee_hls as fn(&mut PreviewSink, &HlsData),
                ));
                let mut writer = HlsWriter::new(HlsWriterConfig {
                    output_dir,
                    base_name,
                    extension: extension.to_string(),
                    max_file_size: (pipeline_config.max_file_size > 0)
                        .then_some(pipeline_config.max_file_size),
                });
                let index_queue =
                    index_tap::install(&mut writer, seg_tx, download_config.index_tap.clone());
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
                    preview,
                    index_queue,
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
    if let Some(limit) = download_config.segment_time_limit() {
        builder = builder.max_duration(limit);
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

fn segment_start_hook(
    seg_tx: UnboundedSender<WriterEvent>,
) -> impl Fn(&std::path::Path, u32) + Send + Sync + 'static {
    move |path, _index| {
        let _ = seg_tx.send(WriterEvent::Opened(path.to_path_buf()));
    }
}

fn segment_complete_hook(
    seg_tx: UnboundedSender<WriterEvent>,
) -> impl Fn(&std::path::Path, u32, f64, u64, Option<&pipeline_common::SplitReason>)
+ Send
+ Sync
+ 'static {
    move |path, index, duration_secs, size_bytes, reason| {
        // 输出目录是 `.` 时 writer 给出 `./x.flv`；去掉前缀，与 stream-gears 给出的路径同一形式，
        // 弹幕 XML、文件列表都按这个路径命名和记录
        let path = path.strip_prefix(".").unwrap_or(path);
        info!(
            path = %path.display(),
            index,
            duration_secs,
            size_bytes,
            reason = ?reason,
            "mesio 分段完成"
        );
        let _ = seg_tx.send(WriterEvent::Closed(
            path.to_path_buf(),
            index,
            duration_secs,
            size_bytes,
        ));
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
///
/// `preview`：直播预览的写入端与「把一个条目旁路给它」的函数，同样在条目进入管线前
/// 调用（预览拿到的是拉到的原始流，不含修复管线的改动）。只是 push，不会失败、不 await。
///
/// `index_queue`：关键帧索引旁路的条目队列（见 [`index_tap`]），在管线之后、writer 之前记下每个条目
/// 要进索引的部分（FLV 是就地判定的结果，HLS 是分片 `Bytes` 的引用计数）。
#[allow(clippy::too_many_arguments)]
async fn run_pipeline<'a, P, W>(
    common: &PipelineConfig,
    config: P::Config,
    items: Pin<Box<dyn Stream<Item = Result<P::Item, PipelineError>> + Send>>,
    spec: ChannelSpec<P::Item>,
    mut writer: W,
    mut seg_rx: UnboundedReceiver<WriterEvent>,
    token: CancellationToken,
    callback: &mut (dyn FnMut(SegmentEvent) + Send + Sync + 'a),
    byte_meter: (ByteCounter, fn(&P::Item) -> usize),
    preview: Option<PreviewTee<P::Item>>,
    index_queue: Option<std::sync::mpsc::Sender<index_tap::Entry>>,
) -> Result<WriterStats, String>
where
    P: PipelineProvider,
    P::Item: index_tap::Indexable + Send + 'static,
    W: ProtocolWriter<Item = P::Item>,
{
    let context = Arc::new(StreamerContext::new(token));
    let provider = P::with_config(context, common, config);
    let SpawnedPipeline {
        input_tx,
        output_rx,
        tasks,
    } = spawn_pipeline(provider.build_pipeline(), spec);
    let output_rx = match index_queue {
        Some(queue) => index_tap::forward(output_rx, queue, CHANNEL_SIZE),
        None => output_rx,
    };

    let writer_task = tokio::task::spawn_blocking(move || writer.run(output_rx));

    let forward = tokio::spawn(async move {
        let (bytes_written, item_size) = byte_meter;
        let mut preview = preview;
        let mut items = items;
        while let Some(item) = items.next().await {
            if let Ok(item) = &item {
                bytes_written.add(item_size(item) as u64);
                if let Some((sink, tee)) = preview.as_mut() {
                    tee(sink, item);
                }
            }
            if input_tx.send(item).await.is_err() {
                debug!("mesio 管线已关闭，停止转发");
                break;
            }
        }
    });

    while let Some(event) = seg_rx.recv().await {
        callback(match event {
            WriterEvent::Opened(path) => SegmentEvent::Start {
                next_file_path: path,
            },
            WriterEvent::Closed(path, index, duration_secs, size_bytes) => SegmentEvent::Segment(
                SegmentInfo::new(path, None, None, index as usize)
                    .with_stats(duration_secs, size_bytes),
            ),
        });
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

/// 把一个 mesio FLV 条目旁路给直播预览，重新编成与 FLV 文件一致的字节排列。
///
/// 类型判定沿用 `flv` crate 对 tag 的分类；修复管线的控制项（`Split` / `EndOfSequence`）
/// 不是媒体字节，跳过。视频序列头是 HEVC 时把 hub 标为不可预览（Chrome 的 MSE 放不了），
/// 录制本身不受影响。
fn tee_flv(sink: &mut PreviewSink, item: &FlvData) {
    match item {
        FlvData::Header(header) => {
            if let Ok(bytes) = flv::encode::encode_header_bytes(header) {
                sink.push(ChunkKind::Header, bytes::Bytes::copy_from_slice(&bytes));
            }
        }
        FlvData::Tag(tag) => {
            let class = tag.classification();
            let kind = if tag.is_script_tag() {
                ChunkKind::SequenceHeader(preview::flv::TAG_SCRIPT)
            } else if tag.is_audio_sequence_header() {
                ChunkKind::SequenceHeader(preview::flv::TAG_AUDIO)
            } else if tag.is_video_sequence_header() {
                if class.codec == Some(CodecKind::Hevc) {
                    sink.mark_unavailable(
                        "视频为 HEVC 编码，浏览器内的播放器无法解码，录制不受影响",
                    );
                }
                ChunkKind::SequenceHeader(preview::flv::TAG_VIDEO)
            } else if tag.is_video_tag() && class.keyframe_media {
                ChunkKind::Keyframe
            } else {
                ChunkKind::Media
            };
            let tag_type = u8::from(tag.tag_type()) | if tag.is_filtered() { 0x20 } else { 0 };
            sink.push(
                kind,
                preview::flv::tag_chunk(tag_type, tag.timestamp_ms, tag.data()),
            );
        }
        FlvData::Split(_) | FlvData::EndOfSequence(_) => {}
    }
}

/// 把一个 mesio HLS 分片旁路给直播预览。
///
/// TS：每个分片自含（带 PAT/PMT），整片作为一个分块推送，起点是否关键帧由写入端嗅探决定。
/// fMP4：init segment（ftyp + moov）作为文件头进快照；分段按 moof + mdat 切成分片，
/// 视频首帧是关键帧的分片作为 GOP 起点（B 站的 fmp4 分段边界不对齐关键帧，不能按分段起播）。
fn tee_hls(sink: &mut PreviewSink, item: &HlsData) {
    let Some(data) = item.data() else { return };
    if item.is_mp4_init() {
        sink.push(ChunkKind::Header, data.clone());
    } else if item.is_mp4() {
        // 分段里可能有多个 moof/mdat 对，且分段边界未必是关键帧：按分片切开、关键帧分片起 GOP
        sink.push_fmp4_segment(data.clone());
    } else {
        sink.push_ts_segment_start(data.clone());
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
    fn segment_paths_under_the_current_dir_drop_the_dot_prefix() {
        let (tx, mut rx) = unbounded_channel();
        let hook = segment_complete_hook(tx);
        hook(std::path::Path::new("./x.flv"), 0, 1.0, 10, None);
        hook(std::path::Path::new("./sub/y.flv"), 1, 1.0, 10, None);
        hook(std::path::Path::new("/data/z.flv"), 2, 1.0, 10, None);
        hook(std::path::Path::new("w.flv"), 3, 1.0, 10, None);

        let paths: Vec<PathBuf> = std::iter::from_fn(|| rx.try_recv().ok())
            .map(|(path, ..)| path)
            .collect();
        assert_eq!(
            paths,
            ["x.flv", "sub/y.flv", "/data/z.flv", "w.flv"].map(PathBuf::from)
        );
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

        // 与 stream-gears 同一套解析：纯秒数不再被静默当成不分段
        let seconds = pipeline_config(&DownloadConfig {
            segment_time: Some("3600".to_string()),
            ..Default::default()
        });
        assert_eq!(seconds.max_duration, Some(Duration::from_secs(3600)));
    }

    /// 端到端：本地 HTTP 服务吐一段真实 FLV 录像，走完整的引擎拉流 → 修复管线 → 落盘，
    /// 校验分段回调触发、按大小切片、文件名沿用 biliup 的模板；
    /// 关键帧索引旁路边写边建的 `.idx` 与对落盘文件扫盘的结果一致。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn downloads_a_local_flv_through_the_engine_and_reports_segments() {
        use crate::server::common::util::Recorder;
        use crate::server::infrastructure::models::StreamerInfo;
        use axum::{Router, routing::get};
        use std::sync::Mutex;

        // 造一段合法 FLV：头 + 一个视频关键帧标签，重复多次以触发按大小分段
        let mut payload = Vec::new();
        payload.extend_from_slice(&[0x46, 0x4C, 0x56, 0x01, 0x01, 0, 0, 0, 9, 0, 0, 0, 0]);
        // AVC 关键帧：一个 1 字节的 IDR NALU（类型 5）
        let data = vec![0x17u8, 0x01, 0, 0, 0, 0, 0, 0, 1, 0x65];
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
            preview: Default::default(),
            index_tap: Some(crate::server::workbench::index::live::spawn()),
        };
        let index_tap = config.index_tap.clone().unwrap();
        let bytes_written = config.bytes_written.clone();
        let preview = config.preview.clone();

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
        index_tap.sync().await;
        assert_eq!(
            bytes_written.total(),
            payload_len as u64,
            "进入管线的字节应等于源 FLV 的长度（文件头 + 全部 tag）"
        );
        // 预览 hub 在 FLV 会话开始时被 attach 为 FLV，会话结束后写入端已 drop 但格式保留
        let preview_status = preview.status();
        assert!(preview_status.available);
        assert_eq!(
            preview_status.format,
            Some(biliup::downloader::preview::PreviewFormat::Flv)
        );
        assert!(!preview.is_attached());
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
            assert_eq!(info.size_bytes, Some(size), "reported size of {name}");
            assert!(
                info.duration_secs.is_some_and(|d| d >= 0.0),
                "reported duration of {name}: {:?}",
                info.duration_secs
            );
        }

        use crate::server::workbench::index;
        let scratch = tempfile::tempdir().unwrap();
        for info in seen.iter() {
            let path = &info.prev_file_path;
            let streamed = index::load(path).expect("streamed index");
            assert_eq!(streamed.source_len, std::fs::metadata(path).unwrap().len());
            assert!(!index::live::is_live(path));
            let copy = scratch.path().join(path.file_name().unwrap());
            std::fs::copy(path, &copy).unwrap();
            let scanned = index::refresh(&copy, true).unwrap();
            assert!(!scanned.keyframes.is_empty());
            assert_eq!(streamed.keyframes, scanned.keyframes, "{}", path.display());
            assert_eq!(streamed.header_len, scanned.header_len);
            assert_eq!(streamed.base_ts, scanned.base_ts);
            assert_eq!(streamed.duration_ms, scanned.duration_ms);
            assert_eq!(streamed.scanned_upto, scanned.scanned_upto);
        }
    }

    /// mesio FLV 条目旁路：文件头 / script / 序列头 / 关键帧 / 普通帧各归其位，
    /// 新订阅者拿到「文件头 + onMetaData + 序列头 + 从关键帧起的 GOP」，字节排列与 FLV 文件一致。
    #[tokio::test]
    async fn tee_flv_rebuilds_a_playable_flv_prefix_for_new_subscribers() {
        use biliup::downloader::preview::{PreviewFormat, PreviewHub};
        use flv::{FlvHeader, FlvTag, FlvTagType};

        let tag = |tag_type: FlvTagType, ts: u32, data: &[u8]| {
            FlvData::Tag(FlvTag::new(
                ts,
                0,
                tag_type,
                false,
                bytes::Bytes::copy_from_slice(data),
            ))
        };
        let hub = PreviewHub::new(4);
        // 这里只看 tee 的分块判定，快照只留当前 GOP（多 GOP 快照在 preview.rs 里测）
        let mut sink = hub
            .attach(PreviewFormat::Flv)
            .with_snapshot_window(Duration::ZERO);
        tee_flv(&mut sink, &FlvData::Header(FlvHeader::new(true, true)));
        tee_flv(
            &mut sink,
            &tag(FlvTagType::ScriptData, 0, b"\x02\x00\x0aonMetaData\x05"),
        );
        tee_flv(
            &mut sink,
            &tag(FlvTagType::Audio, 0, &[0xaf, 0x00, 0x12, 0x10]),
        );
        tee_flv(
            &mut sink,
            &tag(FlvTagType::Video, 0, &[0x17, 0x00, 0, 0, 0, 0x01, 0x64]),
        );
        // 第一个 GOP 会被第二个关键帧替掉
        tee_flv(
            &mut sink,
            &tag(FlvTagType::Video, 0, &[0x17, 0x01, 0, 0, 0, 0xa1]),
        );
        tee_flv(
            &mut sink,
            &tag(FlvTagType::Video, 40, &[0x27, 0x01, 0, 0, 0, 0xb1]),
        );
        tee_flv(
            &mut sink,
            &tag(FlvTagType::Video, 80, &[0x17, 0x01, 0, 0, 0, 0xa2]),
        );
        tee_flv(&mut sink, &tag(FlvTagType::Audio, 90, &[0xaf, 0x01, 0x21]));
        tee_flv(&mut sink, &FlvData::Split(flv::SplitReason::SizeLimit));

        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        tee_flv(
            &mut sink,
            &tag(FlvTagType::Video, 120, &[0x27, 0x01, 0, 0, 0, 0xb2]),
        );
        let sub = pending.await.unwrap().unwrap();

        let chunks = sub.snapshot;
        assert_eq!(
            chunks.len(),
            1 + 3 + 3,
            "header + 3 sequence headers + GOP of 3"
        );
        assert_eq!(
            &chunks[0][..],
            &[0x46, 0x4C, 0x56, 1, 5, 0, 0, 0, 9, 0, 0, 0, 0]
        );
        // 每个 tag 分块：11 字节头 + 载荷 + 4 字节 PreviousTagSize
        let script = &chunks[1];
        assert_eq!(script[0], 18);
        assert_eq!(&script[script.len() - 4..], &(11u32 + 14).to_be_bytes());
        assert_eq!(chunks[2][0], 8);
        assert_eq!(&chunks[3][11..13], &[0x17, 0x00]);
        assert_eq!(&chunks[4][11..17], &[0x17, 0x01, 0, 0, 0, 0xa2]);
        assert_eq!(&chunks[4][4..8], &[0, 0, 80, 0], "timestamp 80 ms");
        assert_eq!(&chunks[5][11..14], &[0xaf, 0x01, 0x21]);
        assert_eq!(&chunks[6][11..17], &[0x27, 0x01, 0, 0, 0, 0xb2]);
        assert!(hub.status().available);
    }

    #[test]
    fn tee_flv_marks_hevc_streams_as_not_previewable() {
        use biliup::downloader::preview::{PreviewFormat, PreviewHub};
        use flv::{FlvTag, FlvTagType};

        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Flv);
        // E-RTMP 序列头：0x90 = ExHeader | KeyFrame | SequenceStart，fourcc hvc1
        let hevc = FlvData::Tag(FlvTag::new(
            0,
            0,
            FlvTagType::Video,
            false,
            bytes::Bytes::from_static(&[0x90, b'h', b'v', b'c', b'1', 1, 2, 3]),
        ));
        tee_flv(&mut sink, &hevc);
        let status = hub.status();
        assert!(!status.available);
        assert!(status.reason.unwrap().contains("HEVC"));
    }

    #[tokio::test]
    async fn tee_hls_forwards_ts_segments_as_keyframe_chunks() {
        use biliup::downloader::preview::{PreviewFormat, PreviewHub};

        let hub = PreviewHub::new(4);
        let mut sink = hub
            .attach(PreviewFormat::MpegTs)
            .with_snapshot_window(Duration::ZERO);
        let segment = || m3u8_rs::MediaSegment {
            uri: "1.ts".to_string(),
            ..Default::default()
        };
        let ts = HlsData::ts(
            segment(),
            bytes::Bytes::from_static(&[0x47, 0x40, 0x00, 0x10]),
        );
        tee_hls(&mut sink, &ts);
        tee_hls(&mut sink, &HlsData::end_marker());
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        tee_hls(&mut sink, &ts);
        let sub = pending.await.unwrap().unwrap();
        // 快照只含最近一个完整分片，且以 TS 同步字节开头
        assert_eq!(sub.snapshot.len(), 1);
        assert_eq!(sub.snapshot[0][0], 0x47);
    }

    /// fMP4：init segment 进快照并解出编码串，moof/mdat 分片按关键帧分块；
    /// 新订阅者拿到 init + 最近一个完整分片。
    #[tokio::test]
    async fn tee_hls_keeps_the_fmp4_init_segment_for_new_subscribers() {
        use biliup::downloader::preview::{PreviewFormat, PreviewHub};

        fn bx(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
            let mut out = ((8 + body.len()) as u32).to_be_bytes().to_vec();
            out.extend_from_slice(kind);
            out.extend_from_slice(body);
            out
        }
        let hub = PreviewHub::new(4);
        let mut sink = hub
            .attach(PreviewFormat::Fmp4)
            .with_snapshot_window(Duration::ZERO);
        let segment = |uri: &str| m3u8_rs::MediaSegment {
            uri: uri.to_string(),
            ..Default::default()
        };
        // 一个只含 avc1/avcC 的最小 init：ftyp + moov/trak/mdia/minf/stbl/stsd
        let mut entry = vec![0u8; 78];
        entry.extend_from_slice(&bx(b"avcC", &[1, 0x64, 0x00, 0x28]));
        let mut stsd = vec![0, 0, 0, 0, 0, 0, 0, 1];
        stsd.extend_from_slice(&bx(b"avc1", &entry));
        let trak = bx(
            b"trak",
            &bx(b"mdia", &bx(b"minf", &bx(b"stbl", &bx(b"stsd", &stsd)))),
        );
        let mut init = bx(b"ftyp", b"iso5");
        init.extend_from_slice(&bx(b"moov", &trak));
        let init = bytes::Bytes::from(init);

        tee_hls(
            &mut sink,
            &HlsData::mp4_init(segment("init.mp4"), init.clone()),
        );
        let status = hub.status();
        assert_eq!(status.format, Some(PreviewFormat::Fmp4));
        assert_eq!(status.codecs.as_deref(), Some("avc1.640028"));

        let seg1 = bytes::Bytes::from_static(b"moof1mdat1");
        let seg2 = bytes::Bytes::from_static(b"moof2mdat2");
        tee_hls(&mut sink, &HlsData::mp4_segment(segment("1.m4s"), seg1));
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        tee_hls(
            &mut sink,
            &HlsData::mp4_segment(segment("2.m4s"), seg2.clone()),
        );
        let sub = pending.await.unwrap().unwrap();
        assert_eq!(sub.snapshot, vec![init, seg2]);
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
