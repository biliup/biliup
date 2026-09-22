//! 直播预览：把正在写盘的媒体字节旁路（tee）一份给浏览器内的播放器，不向 CDN 再拉一路。
//!
//! 结构：
//!
//! - 每路录制任务一个 [`PreviewHub`]，寿命与整场录制（`execute()`）相同，跨分段、跨断流重试。
//! - 下载器每次开始拉流时用 [`PreviewHub::attach`] 拿到一个 [`PreviewSink`]（写入端），
//!   在写盘点旁边调用 [`PreviewSink::push`]；拉流结束时 `PreviewSink` 随之 drop。
//! - HTTP 端点用 [`PreviewHub::subscribe`] 拿到 [`Subscription`]：先是一份「文件头 + 序列头 +
//!   当前 GOP」的快照，之后是与写盘同步的实时分块。
//!
//! 写入端热路径上只做三件事：把分块的引用追加进当前 GOP 缓冲、`try_recv` 待处理的订阅请求、
//! `broadcast::send`。没有 `.await`、没有锁等待、没有可失败的返回值；订阅者的快慢只影响
//! 它自己（掉队即断开重连），不会传导回录制。
//!
//! 内存上限：GOP 快照最多 [`MAX_GOP_BYTES`]，超过就丢掉这一 GOP、等下一个关键帧；
//! 广播缓冲固定 [`BROADCAST_CAPACITY`] 个槽位，每槽最多 [`MAX_CHUNK_BYTES`]（更大的分块会被切开），
//! 且只在有订阅者时才占用。订阅者数量由信号量限制（[`PreviewHub::new`] 的参数）。

use bytes::Bytes;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, broadcast, oneshot};
use tracing::debug;

/// 每路直播默认允许同时观看的预览连接数。
pub const DEFAULT_MAX_SUBSCRIBERS: usize = 4;
/// 广播缓冲槽位数。慢订阅者落后超过这么多分块就会收到 `Lagged`。
pub const BROADCAST_CAPACITY: usize = 256;
/// 单个广播分块的上限；更大的分块（如一个 400 KB 的关键帧 tag）切成多块发送，
/// 因此广播缓冲最多占用 `BROADCAST_CAPACITY * MAX_CHUNK_BYTES` = 16 MB。
pub const MAX_CHUNK_BYTES: usize = 64 * 1024;
/// GOP 快照的字节上限；超过则丢弃本 GOP 的快照数据，新订阅者等下一个关键帧。
pub const MAX_GOP_BYTES: usize = 8 * 1024 * 1024;

/// 旁路出来的容器格式，决定 HTTP 响应的 `Content-Type` 与前端播放器的模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewFormat {
    /// FLV：`video/x-flv`，httpflv 与 mesio 的 FLV 会话，前端 mpegts.js
    Flv,
    /// 连续的 MPEG-TS 字节流（不是 m3u8）：`video/mp2t`，HLS 分片拼接，前端 mpegts.js
    MpegTs,
    /// 分片 MP4（init segment + moof/mdat 分片）：`video/mp4`，前端直接 MediaSource 追加
    Fmp4,
}

impl PreviewFormat {
    pub fn content_type(self) -> &'static str {
        match self {
            PreviewFormat::Flv => "video/x-flv",
            PreviewFormat::MpegTs => "video/mp2t",
            PreviewFormat::Fmp4 => "video/mp4",
        }
    }

    /// 接口里用的短名：`flv` / `mpegts`（与 mpegts.js 的 `type` 一致）/ `fmp4`
    pub fn as_str(self) -> &'static str {
        match self {
            PreviewFormat::Flv => "flv",
            PreviewFormat::MpegTs => "mpegts",
            PreviewFormat::Fmp4 => "fmp4",
        }
    }
}

/// 写入端推送的分块类型，决定它在快照里的位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkKind {
    /// 文件头：FLV 的 9 字节头 + PreviousTagSize0，或 fMP4 的 init segment（ftyp + moov）。
    /// 进快照；FLV 的不广播给已有订阅者（解码器不能中途收到文件头），fMP4 的会广播
    /// （MSE 允许中途追加新的 init segment，对应上游换了初始化分片）。
    Header,
    /// 解码所需的序列头（onMetaData / AAC / AVC 序列头），按槽位替换保存，同时广播。
    /// 槽位由调用方定义（FLV 用 tag 类型），同一槽位后来的覆盖先前的。
    SequenceHeader(u8),
    /// 关键帧（或 TS 分片的起点）：清空并重新开始当前 GOP 缓冲。
    Keyframe,
    /// 其它媒体数据，追加到当前 GOP。
    Media,
}

/// `/v1/streamers` 里透出的预览状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewStatus {
    /// 当前下载器能否提供预览（`true` 时 `format` 可能仍为 `None`：刚开始拉流、尚未确定容器）
    pub available: bool,
    pub format: Option<PreviewFormat>,
    /// fMP4 时从 init segment 的 `moov` 解出的 RFC 6381 编码串（如 `avc1.64001f,mp4a.40.2`），
    /// 供前端 `MediaSource.isTypeSupported()` 校验并 `addSourceBuffer`；其它容器为 `None`
    pub codecs: Option<String>,
    /// 不可预览的原因，面向用户的中文说明
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum HubState {
    /// 下载器能 tee，但还没开始拉流 / 尚未确定容器
    Pending,
    Available {
        format: PreviewFormat,
        codecs: Option<String>,
    },
    Unavailable(String),
}

struct SnapshotRequest {
    reply: oneshot::Sender<SnapshotReply>,
}

struct SnapshotReply {
    format: PreviewFormat,
    snapshot: Vec<Bytes>,
    rx: broadcast::Receiver<Bytes>,
}

struct Shared {
    state: RwLock<HubState>,
    /// 当前写入端的订阅请求入口，`(代数, 发送端)`；没有写入端时为 `None`
    requests: RwLock<Option<(u64, std_mpsc::Sender<SnapshotRequest>)>>,
    generation: AtomicU64,
    subscribers: Arc<Semaphore>,
    max_subscribers: usize,
}

/// 一路录制的预览 hub，`Clone` 得到的是同一个 hub 的另一个句柄。
#[derive(Clone)]
pub struct PreviewHub(Arc<Shared>);

impl Default for PreviewHub {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_SUBSCRIBERS)
    }
}

impl std::fmt::Debug for PreviewHub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreviewHub")
            .field("state", &*self.0.state.read().unwrap())
            .field("attached", &self.0.requests.read().unwrap().is_some())
            .field("max_subscribers", &self.0.max_subscribers)
            .finish()
    }
}

/// [`PreviewHub::subscribe`] 失败的原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscribeError {
    /// 当前下载器 / 容器不支持预览，附原因
    Unavailable(String),
    /// 能预览，但此刻没有写入端（还没开始拉流，或正在断流重试）
    NotAttached,
    /// 该路的预览连接数已达上限
    TooManySubscribers(usize),
    /// 等待关键帧超时（流停滞）
    Timeout,
}

impl std::fmt::Display for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubscribeError::Unavailable(reason) => write!(f, "当前下载器不支持预览: {reason}"),
            SubscribeError::NotAttached => write!(f, "录制尚未开始拉流或正在重连"),
            SubscribeError::TooManySubscribers(n) => {
                write!(f, "该直播间的预览连接数已达上限（{n}）")
            }
            SubscribeError::Timeout => write!(f, "等待关键帧超时"),
        }
    }
}

impl std::error::Error for SubscribeError {}

/// 一个预览订阅：先发 `snapshot` 里的分块，再从 `rx` 取实时分块。
///
/// 持有该路的一个连接许可，drop 即释放。
pub struct Subscription {
    pub format: PreviewFormat,
    pub snapshot: Vec<Bytes>,
    pub rx: broadcast::Receiver<Bytes>,
    _permit: OwnedSemaphorePermit,
}

impl PreviewHub {
    /// 新建一个能 tee 的 hub，`max_subscribers` 为该路同时允许的预览连接数。
    pub fn new(max_subscribers: usize) -> Self {
        Self(Arc::new(Shared {
            state: RwLock::new(HubState::Pending),
            requests: RwLock::new(None),
            generation: AtomicU64::new(0),
            subscribers: Arc::new(Semaphore::new(max_subscribers)),
            max_subscribers,
        }))
    }

    /// 新建一个明确不可预览的 hub（ffmpeg / streamlink 等子进程下载器）。
    pub fn unavailable(reason: impl Into<String>) -> Self {
        let hub = Self::new(0);
        hub.mark_unavailable(reason);
        hub
    }

    /// 标记为不可预览（例如 mesio 判出 fMP4 容器）。
    pub fn mark_unavailable(&self, reason: impl Into<String>) {
        *self.0.state.write().unwrap() = HubState::Unavailable(reason.into());
    }

    pub fn max_subscribers(&self) -> usize {
        self.0.max_subscribers
    }

    /// 当前状态，供 `/v1/streamers` 透出。
    pub fn status(&self) -> PreviewStatus {
        match &*self.0.state.read().unwrap() {
            HubState::Pending => PreviewStatus {
                available: true,
                format: None,
                codecs: None,
                reason: None,
            },
            HubState::Available { format, codecs } => PreviewStatus {
                available: true,
                format: Some(*format),
                codecs: codecs.clone(),
                reason: None,
            },
            HubState::Unavailable(reason) => PreviewStatus {
                available: false,
                format: None,
                codecs: None,
                reason: Some(reason.clone()),
            },
        }
    }

    /// 下载器开始拉流时调用，得到写入端。同一时刻只有最新的写入端接收订阅请求；
    /// 上一个写入端 drop 时其订阅者会收到 `Closed`，由播放器重连拿新快照
    /// （换直链后序列头与时间戳可能都变了，不试图让旧连接无缝跨过去）。
    pub fn attach(&self, format: PreviewFormat) -> PreviewSink {
        let (tx, _rx) = broadcast::channel(BROADCAST_CAPACITY);
        let (request_tx, request_rx) = std_mpsc::channel();
        let generation = self.0.generation.fetch_add(1, Ordering::Relaxed) + 1;
        *self.0.requests.write().unwrap() = Some((generation, request_tx));
        *self.0.state.write().unwrap() = HubState::Available {
            format,
            codecs: None,
        };
        PreviewSink {
            hub: self.0.clone(),
            generation,
            format,
            tx,
            requests: request_rx,
            header: None,
            sequence_headers: Vec::new(),
            gop: Vec::new(),
            gop_bytes: 0,
            gop_ready: false,
        }
    }

    /// 是否有写入端正在推送。
    pub fn is_attached(&self) -> bool {
        self.0.requests.read().unwrap().is_some()
    }

    /// 订阅预览。请求交给写入端在下一次推送时处理，因此拿到的快照与随后的实时分块
    /// 严格衔接（不重复、不缺帧）；写入端会等到有完整关键帧起点时才回应，`timeout`
    /// 需要长于一个 GOP / 一个 HLS 分片。
    pub async fn subscribe(&self, timeout: Duration) -> Result<Subscription, SubscribeError> {
        if let HubState::Unavailable(reason) = &*self.0.state.read().unwrap() {
            return Err(SubscribeError::Unavailable(reason.clone()));
        }
        let permit = self
            .0
            .subscribers
            .clone()
            .try_acquire_owned()
            .map_err(|_| SubscribeError::TooManySubscribers(self.0.max_subscribers))?;
        let request_tx = match &*self.0.requests.read().unwrap() {
            Some((_, tx)) => tx.clone(),
            None => return Err(SubscribeError::NotAttached),
        };
        let (reply_tx, reply_rx) = oneshot::channel();
        request_tx
            .send(SnapshotRequest { reply: reply_tx })
            .map_err(|_| SubscribeError::NotAttached)?;
        match tokio::time::timeout(timeout, reply_rx).await {
            Ok(Ok(reply)) => Ok(Subscription {
                format: reply.format,
                snapshot: reply.snapshot,
                rx: reply.rx,
                _permit: permit,
            }),
            // 写入端在回应前被 drop（拉流结束 / 重试），请求随之作废
            Ok(Err(_)) => Err(SubscribeError::NotAttached),
            Err(_) => Err(SubscribeError::Timeout),
        }
    }
}

/// 预览写入端，由持有写盘循环的下载器独占。
///
/// 所有方法都不会阻塞、不会失败：广播满了覆盖最旧的分块，没有订阅者时什么都不存。
pub struct PreviewSink {
    hub: Arc<Shared>,
    generation: u64,
    format: PreviewFormat,
    tx: broadcast::Sender<Bytes>,
    requests: std_mpsc::Receiver<SnapshotRequest>,
    header: Option<Bytes>,
    sequence_headers: Vec<(u8, Bytes)>,
    gop: Vec<Bytes>,
    gop_bytes: usize,
    /// 当前 GOP 缓冲是否从关键帧起且未超上限；为 `false` 时订阅请求留到下一个关键帧再回应
    gop_ready: bool,
}

impl PreviewSink {
    pub fn format(&self) -> PreviewFormat {
        self.format
    }

    /// 当前有多少实时订阅者在收。
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// 写入端在流里发现浏览器放不了的内容（如 HEVC 序列头）时把 hub 标为不可预览。
    /// 只在序列头出现时调用一次，不在逐 tag 的路径上。
    pub fn mark_unavailable(&self, reason: impl Into<String>) {
        *self.hub.state.write().unwrap() = HubState::Unavailable(reason.into());
    }

    /// 推送一个分块。热路径：不 await、不加锁等待、不返回错误。
    pub fn push(&mut self, kind: ChunkKind, chunk: Bytes) {
        match kind {
            ChunkKind::Header => {
                if self.format == PreviewFormat::Fmp4 {
                    // init segment：每次 attach 只出现一次（上游换初始化分片时才会再来），
                    // 解出编码串供前端 addSourceBuffer；MSE 允许中途追加新 init，照常广播
                    let codecs = mp4::codecs_from_init(&chunk);
                    if let HubState::Available { codecs: slot, .. } =
                        &mut *self.hub.state.write().unwrap()
                    {
                        *slot = codecs;
                    }
                    self.header = Some(chunk.clone());
                    self.broadcast(chunk);
                } else {
                    self.header = Some(chunk);
                }
                return;
            }
            ChunkKind::SequenceHeader(slot) => {
                match self.sequence_headers.iter_mut().find(|(s, _)| *s == slot) {
                    Some(entry) => entry.1 = chunk.clone(),
                    None => self.sequence_headers.push((slot, chunk.clone())),
                }
                self.broadcast(chunk);
            }
            ChunkKind::Keyframe => {
                self.gop.clear();
                self.gop_bytes = 0;
                self.gop_ready = true;
                self.retain(chunk.clone());
                self.broadcast(chunk);
            }
            ChunkKind::Media => {
                if self.gop_ready {
                    self.retain(chunk.clone());
                }
                self.broadcast(chunk);
            }
        }
        if self.gop_ready {
            self.serve_requests();
        }
    }

    fn retain(&mut self, chunk: Bytes) {
        self.gop_bytes += chunk.len();
        if self.gop_bytes > MAX_GOP_BYTES {
            debug!(
                bytes = self.gop_bytes,
                "preview GOP exceeds the snapshot limit, waiting for the next keyframe"
            );
            self.gop.clear();
            self.gop_bytes = 0;
            self.gop_ready = false;
        } else {
            self.gop.push(chunk);
        }
    }

    /// 没有订阅者时 `send` 直接返回 `Err`、不存任何东西；有订阅者时满了覆盖最旧的分块。
    fn broadcast(&self, chunk: Bytes) {
        let mut rest = chunk;
        while rest.len() > MAX_CHUNK_BYTES {
            let piece = rest.split_to(MAX_CHUNK_BYTES);
            let _ = self.tx.send(piece);
        }
        if !rest.is_empty() {
            let _ = self.tx.send(rest);
        }
    }

    /// 先广播再处理请求：新接收端只会收到本分块之后的数据，而本分块已在快照里。
    fn serve_requests(&mut self) {
        while let Ok(request) = self.requests.try_recv() {
            let rx = self.tx.subscribe();
            let mut snapshot = Vec::with_capacity(1 + self.sequence_headers.len() + self.gop.len());
            snapshot.extend(self.header.iter().cloned());
            snapshot.extend(self.sequence_headers.iter().map(|(_, b)| b.clone()));
            snapshot.extend(self.gop.iter().cloned());
            let _ = request.reply.send(SnapshotReply {
                format: self.format,
                snapshot,
                rx,
            });
        }
    }
}

impl Drop for PreviewSink {
    fn drop(&mut self) {
        let mut requests = self.hub.requests.write().unwrap();
        if matches!(&*requests, Some((generation, _)) if *generation == self.generation) {
            *requests = None;
        }
    }
}

/// FLV 相关的分块构造与分类，供 httpflv 与 mesio 两条路径共用。
pub mod flv {
    use super::ChunkKind;
    use bytes::{BufMut, Bytes, BytesMut};

    /// 标准 FLV 文件头（音视频标志位都置上）+ PreviousTagSize0，共 13 字节。
    pub const FILE_HEADER: [u8; 13] = [
        0x46, 0x4c, 0x56, 0x01, 0x05, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00,
    ];

    pub const TAG_AUDIO: u8 = 8;
    pub const TAG_VIDEO: u8 = 9;
    pub const TAG_SCRIPT: u8 = 18;

    /// 由 tag 类型与载荷判定分块类型（不依赖任何解析器，几次字节比较）：
    /// script 与 AAC / AVC 序列头 → `SequenceHeader(tag 类型)`；视频关键帧 → `Keyframe`；其它 → `Media`。
    pub fn classify(tag_type: u8, body: &[u8]) -> ChunkKind {
        match tag_type & 0x1f {
            TAG_SCRIPT => ChunkKind::SequenceHeader(TAG_SCRIPT),
            TAG_AUDIO => {
                // SoundFormat 10 = AAC，第二字节 0 = AudioSpecificConfig
                let aac = body.first().is_some_and(|b| b >> 4 == 10);
                if aac && body.get(1) == Some(&0) {
                    ChunkKind::SequenceHeader(TAG_AUDIO)
                } else {
                    ChunkKind::Media
                }
            }
            TAG_VIDEO => {
                let Some(&first) = body.first() else {
                    return ChunkKind::Media;
                };
                let enhanced = first & 0x80 != 0;
                let frame_type = (first >> 4) & 0x07;
                if enhanced {
                    // E-RTMP：低 4 位是 PacketType，0 = SequenceStart
                    if first & 0x0f == 0 {
                        return ChunkKind::SequenceHeader(TAG_VIDEO);
                    }
                } else {
                    let codec = first & 0x0f;
                    // AVC(7) / HEVC(12) 的第二字节 0 = 序列头
                    if matches!(codec, 7 | 12) && body.get(1) == Some(&0) {
                        return ChunkKind::SequenceHeader(TAG_VIDEO);
                    }
                }
                if frame_type == 1 {
                    ChunkKind::Keyframe
                } else {
                    ChunkKind::Media
                }
            }
            _ => ChunkKind::Media,
        }
    }

    /// 把一个 tag 编成「11 字节 tag 头 + 载荷 + 4 字节 PreviousTagSize（= 11 + 载荷长）」。
    /// 每个分块自带尾随的 PreviousTagSize，与 FLV 文件里 tag 的排列一致，可以直接拼接。
    pub fn tag_chunk(tag_type: u8, timestamp: u32, body: &[u8]) -> Bytes {
        let mut out = BytesMut::with_capacity(11 + body.len() + 4);
        out.put_u8(tag_type);
        let size = body.len() as u32;
        out.put_u8((size >> 16) as u8);
        out.put_u8((size >> 8) as u8);
        out.put_u8(size as u8);
        out.put_u8((timestamp >> 16) as u8);
        out.put_u8((timestamp >> 8) as u8);
        out.put_u8(timestamp as u8);
        out.put_u8((timestamp >> 24) as u8);
        out.put_slice(&[0, 0, 0]);
        out.put_slice(body);
        out.put_u32(11 + size);
        out.freeze()
    }

    /// 已有现成的 tag 头与 PreviousTagSize 字节时（httpflv 的写出循环），直接拼接。
    pub fn tag_chunk_from_parts(header: &[u8], body: &[u8], previous_tag_size: &[u8]) -> Bytes {
        let mut out = BytesMut::with_capacity(header.len() + body.len() + previous_tag_size.len());
        out.put_slice(header);
        out.put_slice(body);
        out.put_slice(previous_tag_size);
        out.freeze()
    }

    /// 编码 11 字节 tag 头。
    pub fn tag_header(tag_type: u8, data_size: u32, timestamp: u32) -> [u8; 11] {
        [
            tag_type,
            (data_size >> 16) as u8,
            (data_size >> 8) as u8,
            data_size as u8,
            (timestamp >> 16) as u8,
            (timestamp >> 8) as u8,
            timestamp as u8,
            (timestamp >> 24) as u8,
            0,
            0,
            0,
        ]
    }
}

/// 从 fMP4 init segment（`ftyp` + `moov`）里解出 RFC 6381 编码串，供浏览器
/// `MediaSource.isTypeSupported('video/mp4; codecs="…"')` 与 `addSourceBuffer`。
///
/// 只读 `moov/trak/mdia/minf/stbl/stsd` 下的 sample entry，不解析别的；
/// 解不出的轨道跳过，全都解不出返回 `None`（前端据此提示而不是转圈）。
pub mod mp4 {
    /// 一个 box 的类型与载荷（不含 8 / 16 字节头）
    struct Box<'a> {
        kind: [u8; 4],
        body: &'a [u8],
    }

    /// 顺序遍历 `data` 里的顶层 box；损坏 / 截断时提前结束而不是 panic。
    fn boxes(mut data: &[u8]) -> impl Iterator<Item = Box<'_>> {
        std::iter::from_fn(move || {
            if data.len() < 8 {
                return None;
            }
            let size32 = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
            let kind = [data[4], data[5], data[6], data[7]];
            let (header, size) = match size32 {
                0 => (8, data.len()),
                1 => {
                    if data.len() < 16 {
                        return None;
                    }
                    let large = u64::from_be_bytes([
                        data[8], data[9], data[10], data[11], data[12], data[13], data[14],
                        data[15],
                    ]);
                    (16, usize::try_from(large).ok()?)
                }
                n => (8, n),
            };
            if size < header || size > data.len() {
                return None;
            }
            let body = &data[header..size];
            data = &data[size..];
            Some(Box { kind, body })
        })
    }

    fn child<'a>(data: &'a [u8], kind: &[u8; 4]) -> Option<&'a [u8]> {
        boxes(data).find(|b| &b.kind == kind).map(|b| b.body)
    }

    /// `stsd` 是 full box：version/flags(4) + entry_count(4)，其后是 sample entry 列表
    fn sample_entries(stsd: &[u8]) -> impl Iterator<Item = Box<'_>> {
        boxes(stsd.get(8..).unwrap_or(&[]))
    }

    /// 解出全部轨道的编码串并用 `,` 连接；一个都解不出时返回 `None`。
    pub fn codecs_from_init(init: &[u8]) -> Option<String> {
        let moov = child(init, b"moov")?;
        let mut codecs = Vec::new();
        for trak in boxes(moov).filter(|b| &b.kind == b"trak") {
            let stsd = child(trak.body, b"mdia")
                .and_then(|mdia| child(mdia, b"minf"))
                .and_then(|minf| child(minf, b"stbl"))
                .and_then(|stbl| child(stbl, b"stsd"));
            let Some(stsd) = stsd else { continue };
            for entry in sample_entries(stsd) {
                if let Some(codec) = sample_entry_codec(&entry.kind, entry.body) {
                    codecs.push(codec);
                }
            }
        }
        (!codecs.is_empty()).then(|| codecs.join(","))
    }

    /// VisualSampleEntry 固定字段 78 字节，AudioSampleEntry 28 字节（QuickTime v1 为 44），
    /// 其后是 `avcC` / `hvcC` / `esds` 等子 box。
    fn sample_entry_codec(kind: &[u8; 4], body: &[u8]) -> Option<String> {
        let fourcc = String::from_utf8_lossy(kind).to_string();
        match kind {
            b"avc1" | b"avc3" => {
                let avcc = child(body.get(78..)?, b"avcC")?;
                // configurationVersion, AVCProfileIndication, profile_compatibility, AVCLevelIndication
                let (profile, compat, level) = (*avcc.get(1)?, *avcc.get(2)?, *avcc.get(3)?);
                Some(format!("{fourcc}.{profile:02x}{compat:02x}{level:02x}"))
            }
            b"hvc1" | b"hev1" => {
                let hvcc = child(body.get(78..)?, b"hvcC")?;
                Some(hevc_codec(&fourcc, hvcc)?)
            }
            b"av01" => {
                let av1c = child(body.get(78..)?, b"av1C")?;
                let b1 = *av1c.get(1)?;
                let b2 = *av1c.get(2)?;
                let profile = b1 >> 5;
                let level = b1 & 0x1f;
                let tier = if b2 & 0x80 != 0 { 'H' } else { 'M' };
                let depth = match (b2 & 0x40 != 0, b2 & 0x20 != 0) {
                    (true, true) => 12,
                    (true, false) => 10,
                    _ => 8,
                };
                Some(format!("av01.{profile}.{level:02}{tier}.{depth:02}"))
            }
            b"vp09" => {
                let vpcc = child(body.get(78..)?, b"vpcC")?;
                // full box：version/flags(4) profile(1) level(1) bitDepth(4b)|chroma(3b)|range(1b)
                let profile = *vpcc.get(4)?;
                let level = *vpcc.get(5)?;
                let depth = *vpcc.get(6)? >> 4;
                Some(format!("vp09.{profile:02}.{level:02}.{depth:02}"))
            }
            b"mp4a" => {
                let esds = [28usize, 44]
                    .into_iter()
                    .filter_map(|offset| body.get(offset..))
                    .find_map(|rest| child(rest, b"esds"))?;
                Some(aac_codec(esds.get(4..)?)?)
            }
            b"ac-3" => Some("ac-3".to_string()),
            b"ec-3" => Some("ec-3".to_string()),
            b"Opus" => Some("opus".to_string()),
            b"fLaC" => Some("flac".to_string()),
            _ => None,
        }
    }

    /// ISO 14496-15 Annex E：`hvc1.<profile_space><profile_idc>.<compat flags 位序反转的十六进制>.<L|H><level_idc>.<constraint bytes 去尾零，点分>`
    fn hevc_codec(fourcc: &str, hvcc: &[u8]) -> Option<String> {
        let b1 = *hvcc.get(1)?;
        let profile_space = match b1 >> 6 {
            0 => "",
            1 => "A",
            2 => "B",
            _ => "C",
        };
        let tier = if b1 & 0x20 != 0 { 'H' } else { 'L' };
        let profile_idc = b1 & 0x1f;
        let compat =
            u32::from_be_bytes([*hvcc.get(2)?, *hvcc.get(3)?, *hvcc.get(4)?, *hvcc.get(5)?])
                .reverse_bits();
        let constraints = hvcc.get(6..12)?;
        let level_idc = *hvcc.get(12)?;
        let mut out = format!("{fourcc}.{profile_space}{profile_idc}.{compat:X}.{tier}{level_idc}");
        let trimmed = constraints
            .iter()
            .rposition(|b| *b != 0)
            .map(|last| &constraints[..=last])
            .unwrap_or(&[]);
        for byte in trimmed {
            out.push_str(&format!(".{byte:X}"));
        }
        Some(out)
    }

    /// 读 MPEG-4 描述符的可变长度（每字节 7 位，高位为续标志，最多 4 字节）
    fn descriptor_len(data: &[u8]) -> Option<(usize, usize)> {
        let mut len = 0usize;
        for (i, byte) in data.iter().take(4).enumerate() {
            len = (len << 7) | (*byte & 0x7f) as usize;
            if byte & 0x80 == 0 {
                return Some((len, i + 1));
            }
        }
        None
    }

    fn descriptor(data: &[u8], tag: u8) -> Option<&[u8]> {
        let mut rest = data;
        while let Some((&t, after)) = rest.split_first() {
            let (len, consumed) = descriptor_len(after)?;
            let body = after.get(consumed..consumed + len)?;
            if t == tag {
                return Some(body);
            }
            rest = &after[consumed + len..];
        }
        None
    }

    /// `esds` → ES_Descriptor(0x03) → DecoderConfigDescriptor(0x04) → objectTypeIndication；
    /// 0x40（MPEG-4 Audio）再读 DecoderSpecificInfo(0x05) 的 audioObjectType → `mp4a.40.<aot>`
    fn aac_codec(esds_body: &[u8]) -> Option<String> {
        let es = descriptor(esds_body, 0x03)?;
        // ES_ID(2) + flags(1)，可选字段按 flags 出现；HLS 里几乎总是 0
        let flags = *es.get(2)?;
        let mut offset = 3;
        if flags & 0x80 != 0 {
            offset += 2;
        }
        if flags & 0x40 != 0 {
            offset += 1 + *es.get(offset)? as usize;
        }
        if flags & 0x20 != 0 {
            offset += 2;
        }
        let dcd = descriptor(es.get(offset..)?, 0x04)?;
        let oti = *dcd.first()?;
        if oti != 0x40 {
            return Some(format!("mp4a.{oti:02x}"));
        }
        // objectTypeIndication(1) streamType(1) bufferSizeDB(3) maxBitrate(4) avgBitrate(4) = 13
        let dsi = descriptor(dcd.get(13..)?, 0x05)?;
        let first = *dsi.first()?;
        let mut aot = (first >> 3) as u32;
        if aot == 31 {
            let second = *dsi.get(1)?;
            aot = 32 + (((first & 0x07) as u32) << 3 | (second >> 5) as u32);
        }
        Some(format!("mp4a.40.{aot}"))
    }

    #[cfg(test)]
    pub(super) mod build {
        //! 测试用的 box 组装
        pub fn bx(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
            let mut out = ((8 + body.len()) as u32).to_be_bytes().to_vec();
            out.extend_from_slice(kind);
            out.extend_from_slice(body);
            out
        }

        pub fn full(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
            let mut b = vec![0, 0, 0, 0];
            b.extend_from_slice(body);
            bx(kind, &b)
        }

        pub fn visual_entry(kind: &[u8; 4], children: &[u8]) -> Vec<u8> {
            let mut body = vec![0u8; 78];
            body[7] = 1; // data_reference_index
            body.extend_from_slice(children);
            bx(kind, &body)
        }

        pub fn audio_entry(kind: &[u8; 4], children: &[u8]) -> Vec<u8> {
            let mut body = vec![0u8; 28];
            body[7] = 1;
            body.extend_from_slice(children);
            bx(kind, &body)
        }

        pub fn stsd(entries: &[Vec<u8>]) -> Vec<u8> {
            let mut body = (entries.len() as u32).to_be_bytes().to_vec();
            for e in entries {
                body.extend_from_slice(e);
            }
            full(b"stsd", &body)
        }

        pub fn trak(stsd: &[u8]) -> Vec<u8> {
            bx(b"trak", &bx(b"mdia", &bx(b"minf", &bx(b"stbl", stsd))))
        }

        pub fn init(traks: &[Vec<u8>]) -> Vec<u8> {
            let mut moov = bx(b"mvhd", &[0u8; 100]);
            for t in traks {
                moov.extend_from_slice(t);
            }
            let mut out = bx(b"ftyp", b"iso5\0\0\0\x01iso5avc1");
            out.extend_from_slice(&bx(b"moov", &moov));
            out
        }

        pub fn esds_aac(aot: u8) -> Vec<u8> {
            // DecoderSpecificInfo: audioObjectType(5) samplingFrequencyIndex(4) channelConfiguration(4)
            let asc = [aot << 3, 0x10];
            let mut dsi = vec![0x05, asc.len() as u8];
            dsi.extend_from_slice(&asc);
            let mut dcd_body = vec![0x40, 0x15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            dcd_body.extend_from_slice(&dsi);
            let mut dcd = vec![0x04, dcd_body.len() as u8];
            dcd.extend_from_slice(&dcd_body);
            let mut es_body = vec![0, 1, 0];
            es_body.extend_from_slice(&dcd);
            let mut es = vec![0x03, es_body.len() as u8];
            es.extend_from_slice(&es_body);
            full(b"esds", &es)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast::error::RecvError;

    #[test]
    fn codecs_are_read_from_the_init_segment() {
        use mp4::build::*;
        let avcc = bx(b"avcC", &[1, 0x64, 0x00, 0x1f, 0xff, 0xe1]);
        let video = trak(&stsd(&[visual_entry(b"avc1", &avcc)]));
        let audio = trak(&stsd(&[audio_entry(b"mp4a", &esds_aac(2))]));
        let avc_aac = init(&[video, audio]);
        assert_eq!(
            mp4::codecs_from_init(&avc_aac).as_deref(),
            Some("avc1.64001f,mp4a.40.2")
        );

        // HEVC Main：profile_space 0 / tier L / profile_idc 1，compat 0x60000000 → 6，level 93，约束 B0
        let mut hvcc = vec![1, 0x01, 0x60, 0, 0, 0, 0xb0, 0, 0, 0, 0, 0, 93];
        hvcc.extend_from_slice(&[0xf0, 0x00, 0xfc, 0xfd]);
        let hevc = trak(&stsd(&[visual_entry(b"hvc1", &bx(b"hvcC", &hvcc))]));
        let he_aac = trak(&stsd(&[audio_entry(b"mp4a", &esds_aac(5))]));
        assert_eq!(
            mp4::codecs_from_init(&init(&[hevc, he_aac])).as_deref(),
            Some("hvc1.1.6.L93.B0,mp4a.40.5")
        );

        // 不认识的 sample entry 跳过；没有 moov 或全都解不出时为 None
        let odd = trak(&stsd(&[visual_entry(b"xxxx", &[])]));
        assert_eq!(mp4::codecs_from_init(&init(&[odd])), None);
        assert_eq!(mp4::codecs_from_init(b"\x00\x00\x00\x08ftyp"), None);
        assert_eq!(mp4::codecs_from_init(&[0, 0, 0]), None);
        // 截断的 moov 不 panic
        let full_init = init(&[trak(&stsd(&[visual_entry(b"avc1", &avcc)]))]);
        assert_eq!(
            mp4::codecs_from_init(&full_init[..full_init.len() - 10]),
            None
        );
    }

    #[tokio::test]
    async fn fmp4_header_sets_codecs_and_is_broadcast_to_live_subscribers() {
        use mp4::build::*;
        let avcc = bx(b"avcC", &[1, 0x4d, 0x40, 0x28]);
        let init = Bytes::from(init(&[trak(&stsd(&[visual_entry(b"avc1", &avcc)]))]));
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Fmp4);
        assert_eq!(hub.status().codecs, None);
        sink.push(ChunkKind::Header, init.clone());
        let status = hub.status();
        assert_eq!(status.format, Some(PreviewFormat::Fmp4));
        assert_eq!(status.codecs.as_deref(), Some("avc1.4d4028"));

        let segment = Bytes::from_static(b"moof+mdat #1");
        sink.push(ChunkKind::Keyframe, segment.clone());
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        let segment2 = Bytes::from_static(b"moof+mdat #2");
        sink.push(ChunkKind::Keyframe, segment2.clone());
        let mut sub = pending.await.unwrap().unwrap();
        // 快照 = init + 最近一个完整分片
        assert_eq!(sub.snapshot, vec![init.clone(), segment2]);
        // 上游换 init 时，已有订阅者也会收到新的 init（MSE 允许中途追加）
        sink.push(ChunkKind::Header, init.clone());
        assert_eq!(sub.rx.recv().await.unwrap(), init);
    }

    fn key(n: u8) -> Bytes {
        Bytes::from(vec![0x17, 1, n, n, n])
    }

    fn inter(n: u8) -> Bytes {
        Bytes::from(vec![0x27, 1, n])
    }

    #[test]
    fn classify_recognises_headers_keyframes_and_media() {
        use flv::*;
        assert_eq!(
            classify(TAG_SCRIPT, b"\x02\x00\x0aonMetaData"),
            ChunkKind::SequenceHeader(TAG_SCRIPT)
        );
        assert_eq!(
            classify(TAG_AUDIO, &[0xaf, 0x00, 0x12, 0x10]),
            ChunkKind::SequenceHeader(TAG_AUDIO)
        );
        assert_eq!(classify(TAG_AUDIO, &[0xaf, 0x01, 0x21]), ChunkKind::Media);
        // MP3 音频不是 AAC，没有序列头
        assert_eq!(classify(TAG_AUDIO, &[0x2f, 0x00]), ChunkKind::Media);
        assert_eq!(
            classify(TAG_VIDEO, &[0x17, 0x00, 0, 0, 0, 1, 0x64]),
            ChunkKind::SequenceHeader(TAG_VIDEO)
        );
        assert_eq!(
            classify(TAG_VIDEO, &[0x17, 0x01, 0, 0, 0]),
            ChunkKind::Keyframe
        );
        assert_eq!(
            classify(TAG_VIDEO, &[0x27, 0x01, 0, 0, 0]),
            ChunkKind::Media
        );
        // E-RTMP：0x90 = ExHeader + KeyFrame + SequenceStart，0x91 = KeyFrame + CodedFrames
        assert_eq!(
            classify(TAG_VIDEO, &[0x90, b'h', b'v', b'c', b'1']),
            ChunkKind::SequenceHeader(TAG_VIDEO)
        );
        assert_eq!(
            classify(TAG_VIDEO, &[0x91, b'h', b'v', b'c', b'1']),
            ChunkKind::Keyframe
        );
        assert_eq!(classify(TAG_VIDEO, &[]), ChunkKind::Media);
        // filter 位（0x20）不影响类型判定
        assert_eq!(
            classify(TAG_SCRIPT | 0x20, b"x"),
            ChunkKind::SequenceHeader(TAG_SCRIPT)
        );
    }

    #[test]
    fn tag_chunk_layout_matches_flv_file_layout() {
        let chunk = flv::tag_chunk(9, 0x0102_0304, &[0xaa, 0xbb, 0xcc]);
        assert_eq!(chunk.len(), 11 + 3 + 4);
        assert_eq!(chunk[0], 9);
        assert_eq!(&chunk[1..4], &[0, 0, 3]);
        assert_eq!(&chunk[4..7], &[0x02, 0x03, 0x04]);
        assert_eq!(chunk[7], 0x01);
        assert_eq!(&chunk[8..11], &[0, 0, 0]);
        assert_eq!(&chunk[11..14], &[0xaa, 0xbb, 0xcc]);
        assert_eq!(&chunk[14..], &(14u32).to_be_bytes());
        let header = flv::tag_header(9, 3, 0x0102_0304);
        assert_eq!(&chunk[..11], &header);
        assert_eq!(
            flv::tag_chunk_from_parts(&header, &[0xaa, 0xbb, 0xcc], &(14u32).to_be_bytes()),
            chunk
        );
    }

    #[test]
    fn status_follows_attach_and_unavailable() {
        let hub = PreviewHub::new(4);
        assert_eq!(
            hub.status(),
            PreviewStatus {
                available: true,
                format: None,
                codecs: None,
                reason: None
            }
        );
        assert!(!hub.is_attached());
        {
            let sink = hub.attach(PreviewFormat::MpegTs);
            assert!(hub.is_attached());
            assert_eq!(hub.status().format, Some(PreviewFormat::MpegTs));
            assert_eq!(hub.status().codecs, None);
            assert_eq!(sink.format(), PreviewFormat::MpegTs);
        }
        // 写入端 drop 后没有入口，但格式仍保留（重试期间界面不闪烁）
        assert!(!hub.is_attached());
        assert_eq!(hub.status().format, Some(PreviewFormat::MpegTs));

        let unavailable = PreviewHub::unavailable("ffmpeg");
        let status = unavailable.status();
        assert!(!status.available);
        assert_eq!(status.reason.as_deref(), Some("ffmpeg"));
        assert!(format!("{unavailable:?}").contains("Unavailable"));
    }

    #[tokio::test]
    async fn subscribe_without_sink_or_on_unavailable_hub_fails_fast() {
        let hub = PreviewHub::new(4);
        assert_eq!(
            hub.subscribe(Duration::from_millis(50)).await.err(),
            Some(SubscribeError::NotAttached)
        );
        let unavailable = PreviewHub::unavailable("streamlink");
        assert_eq!(
            unavailable.subscribe(Duration::from_millis(50)).await.err(),
            Some(SubscribeError::Unavailable("streamlink".into()))
        );
    }

    /// 新订阅者拿到的快照 = 文件头 + 序列头 + 从最近关键帧起的整个 GOP，
    /// 实时接收端从快照之后的第一个分块开始，既不重复也不缺帧。
    #[tokio::test]
    async fn snapshot_is_keyframe_aligned_and_continues_seamlessly() {
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Flv);
        sink.push(ChunkKind::Header, Bytes::from_static(&flv::FILE_HEADER));
        sink.push(ChunkKind::SequenceHeader(18), Bytes::from_static(b"meta"));
        sink.push(ChunkKind::SequenceHeader(8), Bytes::from_static(b"aac"));
        sink.push(ChunkKind::SequenceHeader(9), Bytes::from_static(b"avc"));
        sink.push(ChunkKind::Keyframe, key(1));
        sink.push(ChunkKind::Media, inter(2));
        sink.push(ChunkKind::Media, inter(3));
        // 序列头更新后，快照里应是新的
        sink.push(ChunkKind::SequenceHeader(9), Bytes::from_static(b"avc2"));

        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        // 请求在写入端下一次推送时处理
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, inter(4));
        let mut sub = pending.await.unwrap().expect("subscribed");

        assert_eq!(sub.format, PreviewFormat::Flv);
        let expected: Vec<Bytes> = vec![
            Bytes::from_static(&flv::FILE_HEADER),
            Bytes::from_static(b"meta"),
            Bytes::from_static(b"aac"),
            Bytes::from_static(b"avc2"),
            key(1),
            inter(2),
            inter(3),
            inter(4),
        ];
        assert_eq!(sub.snapshot, expected);

        sink.push(ChunkKind::Media, inter(5));
        sink.push(ChunkKind::Keyframe, key(6));
        assert_eq!(sub.rx.recv().await.unwrap(), inter(5));
        assert_eq!(sub.rx.recv().await.unwrap(), key(6));
        assert_eq!(sink.receiver_count(), 1);
    }

    /// 关键帧之前（或 GOP 超限后）不回应订阅，等到下一个关键帧再给快照。
    #[tokio::test]
    async fn subscription_waits_for_the_next_keyframe() {
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::MpegTs);
        sink.push(ChunkKind::Media, inter(0));
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, inter(1));
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(!pending.is_finished(), "must not answer mid-GOP");
        sink.push(ChunkKind::Keyframe, key(2));
        let sub = pending.await.unwrap().unwrap();
        assert_eq!(sub.snapshot, vec![key(2)]);
    }

    #[tokio::test]
    async fn subscription_times_out_when_the_stream_is_stalled() {
        let hub = PreviewHub::new(4);
        let _sink = hub.attach(PreviewFormat::Flv);
        assert_eq!(
            hub.subscribe(Duration::from_millis(30)).await.err(),
            Some(SubscribeError::Timeout)
        );
    }

    #[tokio::test]
    async fn gop_over_the_limit_is_dropped_until_the_next_keyframe() {
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Flv);
        sink.push(ChunkKind::Keyframe, key(1));
        let big = Bytes::from(vec![0u8; MAX_GOP_BYTES / 2 + 1]);
        sink.push(ChunkKind::Media, big.clone());
        assert!(sink.gop_ready);
        sink.push(ChunkKind::Media, big);
        assert!(!sink.gop_ready);
        assert!(sink.gop.is_empty());
        assert_eq!(sink.gop_bytes, 0);
        sink.push(ChunkKind::Keyframe, key(2));
        assert!(sink.gop_ready);
        assert_eq!(sink.gop, vec![key(2)]);
    }

    #[tokio::test]
    async fn large_chunks_are_split_for_broadcast_but_kept_whole_in_snapshot() {
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Flv);
        sink.push(ChunkKind::Keyframe, key(1));
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, inter(2));
        let mut sub = pending.await.unwrap().unwrap();

        let big = Bytes::from(
            (0..(MAX_CHUNK_BYTES * 2 + 5))
                .map(|i| i as u8)
                .collect::<Vec<_>>(),
        );
        sink.push(ChunkKind::Media, big.clone());
        let mut received = Vec::new();
        for _ in 0..3 {
            let piece = sub.rx.recv().await.unwrap();
            assert!(piece.len() <= MAX_CHUNK_BYTES);
            received.extend_from_slice(&piece);
        }
        assert_eq!(received, big);
        assert!(sub.rx.try_recv().is_err());
    }

    /// 订阅者上限：超出的订阅立刻被拒，释放一个后又能进。
    #[tokio::test]
    async fn subscriber_limit_is_enforced_per_hub() {
        let hub = PreviewHub::new(2);
        let mut sink = hub.attach(PreviewFormat::Flv);
        sink.push(ChunkKind::Keyframe, key(1));

        let subscribe = |hub: PreviewHub| {
            tokio::spawn(async move { hub.subscribe(Duration::from_secs(5)).await })
        };
        let a = subscribe(hub.clone());
        let b = subscribe(hub.clone());
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, inter(2));
        let a = a.await.unwrap().unwrap();
        let b = b.await.unwrap().unwrap();

        assert_eq!(
            hub.subscribe(Duration::from_secs(1)).await.err(),
            Some(SubscribeError::TooManySubscribers(2))
        );
        drop(a);
        let c = subscribe(hub.clone());
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, inter(3));
        assert!(c.await.unwrap().is_ok());
        drop(b);
    }

    /// 慢订阅者不拖慢写入端：一个从不读取的订阅者在场，写入端推 20 倍缓冲容量的分块也
    /// 不会等待；该订阅者随后收到 `Lagged`（由端点断开它让播放器重连）。
    /// 另一个边读边收的订阅者，收到的加上被明确告知跳过的正好等于推送总数——不会静默丢数据。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn slow_subscriber_never_blocks_the_producer() {
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Flv);
        sink.push(ChunkKind::Keyframe, key(0));

        let slow = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        let fast = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(30)).await;
        sink.push(ChunkKind::Media, inter(1));
        let mut slow = slow.await.unwrap().unwrap();
        let mut fast = fast.await.unwrap().unwrap();

        let total = BROADCAST_CAPACITY * 20;
        let reader = tokio::spawn(async move {
            let mut got = 0usize;
            let mut skipped = 0usize;
            loop {
                match fast.rx.recv().await {
                    Ok(_) => got += 1,
                    Err(RecvError::Lagged(n)) => skipped += n as usize,
                    Err(RecvError::Closed) => break,
                }
            }
            (got, skipped)
        });

        let started = std::time::Instant::now();
        for i in 0..total {
            sink.push(ChunkKind::Media, inter((i % 250) as u8));
            // 让读取任务有机会跟上，模拟真实节奏而不是纯 CPU 竞速
            if i % 64 == 0 {
                tokio::task::yield_now().await;
            }
        }
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_secs(5),
            "producer took {elapsed:?} for {total} chunks with a stalled subscriber"
        );

        // 从不读取的订阅者：第一次 recv 就发现自己掉队了
        assert!(matches!(slow.rx.recv().await, Err(RecvError::Lagged(_))));

        drop(sink);
        // 正常读取的订阅者：收到的 + 被告知跳过的 = 全部（不多不少，没有静默丢失）
        let (got, skipped) = reader.await.unwrap();
        assert_eq!(got + skipped, total);
    }

    /// 写入端 drop（拉流结束 / 断流重试）后订阅者收到 `Closed`，hub 本身仍可再次 attach。
    #[tokio::test]
    async fn dropping_the_sink_closes_subscribers_and_allows_reattach() {
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Flv);
        sink.push(ChunkKind::Keyframe, key(1));
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, inter(2));
        let mut sub = pending.await.unwrap().unwrap();

        // 一个尚未被回应的请求，也随写入端消失而作废
        let orphan = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        drop(sink);
        assert!(matches!(sub.rx.recv().await, Err(RecvError::Closed)));
        assert_eq!(
            orphan.await.unwrap().err(),
            Some(SubscribeError::NotAttached)
        );
        assert!(!hub.is_attached());

        let mut sink2 = hub.attach(PreviewFormat::Flv);
        sink2.push(ChunkKind::Keyframe, key(9));
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink2.push(ChunkKind::Media, inter(10));
        assert_eq!(
            pending.await.unwrap().unwrap().snapshot,
            vec![key(9), inter(10)]
        );
    }

    /// 旧写入端晚于新写入端 drop 时，不能把新写入端的入口清掉。
    #[test]
    fn stale_sink_drop_does_not_detach_the_newer_sink() {
        let hub = PreviewHub::new(4);
        let old = hub.attach(PreviewFormat::Flv);
        let _new = hub.attach(PreviewFormat::MpegTs);
        drop(old);
        assert!(hub.is_attached());
        assert_eq!(hub.status().format, Some(PreviewFormat::MpegTs));
    }

    #[test]
    fn nothing_is_buffered_without_subscribers() {
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Flv);
        for i in 0..1000u32 {
            sink.push(ChunkKind::Media, Bytes::from(i.to_be_bytes().to_vec()));
        }
        assert_eq!(sink.receiver_count(), 0);
        assert_eq!(sink.tx.len(), 0);
    }
}

#[cfg(test)]
mod real_init_segments {
    //! 手工校验：读 ffmpeg 生成的 init segment（本地有文件时才跑）。
    //! `cargo test -p biliup -- --ignored real_init` 前先用 ffmpeg 的 hls fmp4 输出准备样本。
    use super::mp4::codecs_from_init;

    #[test]
    #[ignore]
    fn ffmpeg_generated_init_segments_parse() {
        for (path, expected) in [
            ("/tmp/lp2-fmp4/init.mp4", "avc1.64001f,mp4a.40.2"),
            ("/tmp/lp2-fmp4/init_hevc.mp4", "hvc1.1.6.L60.90"),
        ] {
            let Ok(bytes) = std::fs::read(path) else {
                continue;
            };
            assert_eq!(
                codecs_from_init(&bytes).as_deref(),
                Some(expected),
                "{path}"
            );
        }
    }
}
