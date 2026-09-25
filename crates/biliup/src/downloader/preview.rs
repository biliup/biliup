//! 直播预览：把正在写盘的媒体字节旁路（tee）一份给浏览器内的播放器，不向 CDN 再拉一路。
//!
//! 结构：
//!
//! - 每路录制任务一个 [`PreviewHub`]，寿命与整场录制（`execute()`）相同，跨分段、跨断流重试。
//! - 下载器每次开始拉流时用 [`PreviewHub::attach`] 拿到一个 [`PreviewSink`]（写入端），
//!   在写盘点旁边调用 [`PreviewSink::push`]；拉流结束时 `PreviewSink` 随之 drop。
//! - HTTP 端点用 [`PreviewHub::subscribe`] / [`PreviewHub::subscribe_with_depth`] 拿到 [`Subscription`]：
//!   先是一份「文件头 + 序列头 + 最近一段时间内的完整 GOP + 当前 GOP」的快照（回溯多久由播放器按
//!   自己要维持的缓冲深度指定，最多 [`SNAPSHOT_WINDOW`]），之后是与写盘同步的实时分块。
//!
//! 写入端热路径上只做三件事：把分块的引用追加进当前 GOP 缓冲、`try_recv` 待处理的订阅请求、
//! `broadcast::send`。没有 `.await`、没有锁等待、没有可失败的返回值；订阅者的快慢只影响
//! 它自己（掉队即从最近的关键帧重新对齐），不会传导回录制。
//!
//! 内存上限：单个 GOP 最多 [`MAX_GOP_BYTES`]，超过就丢掉这一 GOP、等下一个关键帧；整份快照
//! （已完成 GOP + 当前 GOP）最多 [`MAX_SNAPSHOT_BYTES`]，超过就先丢最旧的 GOP；
//! 广播缓冲按格式固定槽位数（[`BROADCAST_CAPACITY_FLV`] / [`BROADCAST_CAPACITY_SEGMENTED`]），
//! 每槽最多 [`MAX_CHUNK_BYTES`]（更大的分块会被切开），
//! 且只在有订阅者时才占用。订阅者数量是固定槽位（[`PreviewHub::new`] 的参数，见 [`PreviewSlots`]）：
//! 满了由 HTTP 订阅路径决定拒绝新来的还是挤掉最早的一条，写入端不参与。

use bytes::Bytes;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, oneshot, watch};
use tracing::debug;

/// 每路直播默认允许同时观看的预览连接数。
pub const DEFAULT_MAX_SUBSCRIBERS: usize = 4;
/// 快照里保留最近多少时间内（按到达时刻计）的已完成 GOP。
///
/// 新订阅者拿到的快照就是它起播时的全部缓冲：只给当前 GOP 时，播放器从零到一个 GOP 之间的
/// 缓冲起步，之后到达 = 消耗，缓冲永远这么薄，链路上几十到几百毫秒的抖动就 `waiting`
/// （浏览器直连 CDN 时 CDN 会先给几秒的 GOP 缓存，中转也得给同样的深度）。
/// 6 s 盖住前端最深的档位（「流畅」5 s 缓冲 + 起播对齐的余量）；更浅的档位（「低延迟」2 s）
/// 由播放器用 `snapshot_ms` 只要自己那一份，见 [`PreviewHub::subscribe_with_depth`]。
pub const SNAPSHOT_WINDOW: Duration = Duration::from_secs(6);
/// 整份快照（已完成 GOP + 当前 GOP）的字节上限，超过先丢最旧的 GOP。
/// 6 s × 20 Mbps = 15 MB，够到 4K 直播；1080p 常见的 3–6 Mbps 只用 2–5 MB。
pub const MAX_SNAPSHOT_BYTES: usize = 16 * 1024 * 1024;
/// FLV 的广播缓冲槽位数：一个分块就是一个 tag（音频几百字节、视频几 KB 到几十 KB），
/// 而 CDN 常常整 GOP 突发送达（实测斗鱼一次 ~320 个 tag / 2 MB 在 1 ms 内解析完），
/// 缓冲必须装得下一整个突发，否则刚建立的订阅者还没来得及读就 `Lagged`。
/// 1024 槽 × 实测均值 ~10 KB ≈ 10 MB；理论上限 `1024 * MAX_CHUNK_BYTES` = 64 MB 只在每个 tag
/// 都 ≥ 64 KB 时才会到（那相当于 5 MB/s 以上的码率）。
pub const BROADCAST_CAPACITY_FLV: usize = 1024;
/// TS / fMP4 的广播缓冲槽位数：分块是网络读块（16–64 KB）或整个 moof+mdat 分片，
/// 数量少、体积大，256 槽 × `MAX_CHUNK_BYTES` = 16 MB 上限。
pub const BROADCAST_CAPACITY_SEGMENTED: usize = 256;
/// 单个广播分块的上限；更大的分块（如一个 400 KB 的关键帧 tag）切成多块发送，
/// 广播缓冲最多占用「槽位数 × MAX_CHUNK_BYTES」。
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

    /// 该格式的广播缓冲槽位数，见 [`BROADCAST_CAPACITY_FLV`] / [`BROADCAST_CAPACITY_SEGMENTED`]。
    pub fn broadcast_capacity(self) -> usize {
        match self {
            PreviewFormat::Flv => BROADCAST_CAPACITY_FLV,
            PreviewFormat::MpegTs | PreviewFormat::Fmp4 => BROADCAST_CAPACITY_SEGMENTED,
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
    /// 此刻占着该路槽位的预览连接数
    pub subscribers: usize,
    /// 该路的槽位数；不可预览的 hub 为 0
    pub max_subscribers: usize,
}

/// 预览许可池所在的层级，决定满员时挤掉的范围与说明文字。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotScope {
    /// 单个直播间（[`PreviewHub`] 自己的槽位）
    Room,
    /// 整个进程（所有直播间合计）
    Process,
}

impl SlotScope {
    pub fn as_str(self) -> &'static str {
        match self {
            SlotScope::Room => "该直播间",
            SlotScope::Process => "进程内",
        }
    }
}

/// 一条预览连接在各级许可池里的身份。池满时被挤掉的连接由它得知，响应随之结束。
///
/// `Clone` 得到的是同一张票；同一张票可以同时占多个池（直播间 + 进程）的槽位，
/// 任何一级挤掉它，它在其它池里的槽位也不再计入占用（见 [`PreviewSlots::acquire`]）。
#[derive(Clone)]
pub struct PreviewTicket(Arc<TicketInner>);

struct TicketInner {
    id: u64,
    opened: Instant,
    evicted: watch::Sender<Option<SlotScope>>,
}

impl Default for PreviewTicket {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PreviewTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreviewTicket")
            .field("id", &self.0.id)
            .field("evicted_by", &self.evicted_by())
            .finish()
    }
}

impl PreviewTicket {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(Arc::new(TicketInner {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            opened: Instant::now(),
            evicted: watch::Sender::new(None),
        }))
    }

    /// 进程内唯一的连接编号，日志里用
    pub fn id(&self) -> u64 {
        self.0.id
    }

    /// 拿到这张票（开始占槽位）以来的时长
    pub fn age(&self) -> Duration {
        self.0.opened.elapsed()
    }

    /// 被哪一级许可池挤掉；还没被挤掉为 `None`
    pub fn evicted_by(&self) -> Option<SlotScope> {
        *self.0.evicted.borrow()
    }

    /// 被挤掉时完成（已经被挤掉则立刻完成）。可放进 `select!`，丢弃即取消等待。
    pub async fn evicted(&self) -> SlotScope {
        let mut rx = self.0.evicted.subscribe();
        let scope = rx.wait_for(Option::is_some).await.ok().and_then(|v| *v);
        match scope {
            Some(scope) => scope,
            // 发送端就在 `self` 里，活得比这个 future 久，走不到这里
            None => std::future::pending().await,
        }
    }

    fn evict(&self, scope: SlotScope) {
        self.0.evicted.send_if_modified(|current| {
            if current.is_some() {
                return false;
            }
            *current = Some(scope);
            true
        });
    }
}

/// [`PreviewSlots::acquire`] 挤掉的那条连接。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Evicted {
    /// 被挤掉的连接编号（[`PreviewTicket::id`]）
    pub id: u64,
    /// 它占了多久
    pub age: Duration,
}

/// 固定槽位的预览许可池。满员时可以拒绝新连接，也可以挤掉最早进来的那一条：
/// 服务端看不出客户端是否还在看（经过替客户端读完上游的代理 / 隧道时，关掉播放器 TCP 也不断），
/// 所以不能指望旧连接自己释放；而用户新打开的预览一定是他此刻想看的。
///
/// 只在 HTTP 订阅路径上使用（短暂加锁、挤人都在这里），写入端热路径不碰它。
#[derive(Clone)]
pub struct PreviewSlots(Arc<SlotsInner>);

struct SlotsInner {
    scope: SlotScope,
    capacity: usize,
    /// 按拿到槽位的先后排列，队首最早
    holders: Mutex<VecDeque<PreviewTicket>>,
}

impl SlotsInner {
    fn holders(&self) -> MutexGuard<'_, VecDeque<PreviewTicket>> {
        self.holders.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl PreviewSlots {
    pub fn new(scope: SlotScope, capacity: usize) -> Self {
        Self(Arc::new(SlotsInner {
            scope,
            capacity,
            holders: Mutex::new(VecDeque::with_capacity(capacity)),
        }))
    }

    pub fn scope(&self) -> SlotScope {
        self.0.scope
    }

    pub fn capacity(&self) -> usize {
        self.0.capacity
    }

    /// 占着槽位的连接数；已被（任何一级）挤掉、正在收尾的连接不算
    pub fn occupied(&self) -> usize {
        self.0
            .holders()
            .iter()
            .filter(|t| t.evicted_by().is_none())
            .count()
    }

    /// 为 `ticket` 占一个槽位。满员时：`evict` 为真就挤掉最早进来的那条，把位置让给它；
    /// 否则返回 `Err(当前占用数)`。已被挤掉、响应还没来得及结束的连接不占名额，
    /// 所以直播间一级挤掉的那条不会让进程一级再多挤一条。
    pub fn acquire(
        &self,
        ticket: &PreviewTicket,
        evict: bool,
    ) -> Result<(PreviewSlot, Option<Evicted>), usize> {
        let mut holders = self.0.holders();
        holders.retain(|t| t.evicted_by().is_none());
        let mut evicted = None;
        if holders.len() >= self.0.capacity {
            if !evict {
                return Err(holders.len());
            }
            let Some(oldest) = holders.pop_front() else {
                return Err(0);
            };
            oldest.evict(self.0.scope);
            evicted = Some(Evicted {
                id: oldest.id(),
                age: oldest.age(),
            });
        }
        holders.push_back(ticket.clone());
        drop(holders);
        Ok((
            PreviewSlot {
                slots: self.0.clone(),
                ticket: ticket.id(),
            },
            evicted,
        ))
    }
}

/// 占着的一个槽位，drop 即归还（被挤掉的连接此时早已不计入占用，归还是空操作）。
pub struct PreviewSlot {
    slots: Arc<SlotsInner>,
    ticket: u64,
}

impl Drop for PreviewSlot {
    fn drop(&mut self) {
        let mut holders = self.slots.holders();
        if let Some(i) = holders.iter().position(|t| t.id() == self.ticket) {
            holders.remove(i);
        }
    }
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
    /// 快照里已完成 GOP 最多回溯多久（见 [`PreviewHub::subscribe_with_depth`]）；`None` 给整个保留窗口
    depth: Option<Duration>,
}

struct SnapshotReply {
    format: PreviewFormat,
    snapshot: Vec<Bytes>,
    /// `snapshot` 开头有几个分块是文件头（0 或 1）
    header_len: usize,
    rx: broadcast::Receiver<Bytes>,
}

struct Shared {
    state: RwLock<HubState>,
    /// 当前写入端的订阅请求入口，`(代数, 发送端)`；没有写入端时为 `None`
    requests: RwLock<Option<(u64, std_mpsc::Sender<SnapshotRequest>)>>,
    generation: AtomicU64,
    subscribers: PreviewSlots,
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
            .field("subscribers", &self.0.subscribers.occupied())
            .field("max_subscribers", &self.0.subscribers.capacity())
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
    /// 该路的预览连接数已达上限（附上限），且调用方不许挤掉旧连接
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
                write!(f, "该直播间的预览连接数已达上限（每个直播间最多 {n} 路）")
            }
            SubscribeError::Timeout => write!(f, "等待关键帧超时"),
        }
    }
}

impl std::error::Error for SubscribeError {}

/// 一个预览订阅：先发 `snapshot` 里的分块，再从 `rx` 取实时分块。
///
/// 持有该路的一个槽位，drop 即释放；掉队后可用 [`PreviewHub::resubscribe`] 原地重新对齐。
pub struct Subscription {
    pub format: PreviewFormat,
    pub snapshot: Vec<Bytes>,
    pub rx: broadcast::Receiver<Bytes>,
    header_len: usize,
    /// 订阅时要的快照深度，掉队重新对齐时沿用
    depth: Option<Duration>,
    _permit: PreviewSlot,
}

impl Subscription {
    /// 快照去掉开头的文件头：掉队后在同一条响应里续播时用——FLV 的解码器不能中途再收到
    /// 文件头；序列头与 GOP 照常给（序列头可能在掉队期间换过）。
    pub fn snapshot_after_header(&self) -> &[Bytes] {
        &self.snapshot[self.header_len.min(self.snapshot.len())..]
    }
}

impl PreviewHub {
    /// 新建一个能 tee 的 hub，`max_subscribers` 为该路同时允许的预览连接数。
    pub fn new(max_subscribers: usize) -> Self {
        Self(Arc::new(Shared {
            state: RwLock::new(HubState::Pending),
            requests: RwLock::new(None),
            generation: AtomicU64::new(0),
            subscribers: PreviewSlots::new(SlotScope::Room, max_subscribers),
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
        self.0.subscribers.capacity()
    }

    /// 此刻占着该路槽位的预览连接数
    pub fn subscribers(&self) -> usize {
        self.0.subscribers.occupied()
    }

    /// 当前状态，供 `/v1/streamers` 透出。
    pub fn status(&self) -> PreviewStatus {
        let subscribers = self.subscribers();
        let max_subscribers = self.max_subscribers();
        match &*self.0.state.read().unwrap() {
            HubState::Pending => PreviewStatus {
                available: true,
                format: None,
                codecs: None,
                reason: None,
                subscribers,
                max_subscribers,
            },
            HubState::Available { format, codecs } => PreviewStatus {
                available: true,
                format: Some(*format),
                codecs: codecs.clone(),
                reason: None,
                subscribers,
                max_subscribers,
            },
            HubState::Unavailable(reason) => PreviewStatus {
                available: false,
                format: None,
                codecs: None,
                reason: Some(reason.clone()),
                subscribers,
                max_subscribers,
            },
        }
    }

    /// 下载器开始拉流时调用，得到写入端。同一时刻只有最新的写入端接收订阅请求；
    /// 上一个写入端 drop 时其订阅者会收到 `Closed`，由播放器重连拿新快照
    /// （换直链后序列头与时间戳可能都变了，不试图让旧连接无缝跨过去）。
    pub fn attach(&self, format: PreviewFormat) -> PreviewSink {
        let (tx, _rx) = broadcast::channel(format.broadcast_capacity());
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
            gop_started: Instant::now(),
            history: VecDeque::new(),
            history_bytes: 0,
            snapshot_window: SNAPSHOT_WINDOW,
            fmp4_init: mp4::InitInfo::default(),
            ring_used: false,
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
        self.subscribe_with_depth(None, timeout).await
    }

    /// 同 [`subscribe`](Self::subscribe)，但快照里的已完成 GOP 只回溯 `depth`：
    /// 从当前 GOP 往前，取到第一个关键帧到达时刻早于「现在 − depth」的 GOP 为止（含），
    /// 所以起播缓冲在 `depth` 到 `depth + 一个 GOP` 之间；`Some(ZERO)` 只给当前 GOP，
    /// `None` 给整个 [`SNAPSHOT_WINDOW`]。播放器要多深的缓冲就要多深的快照，多要的只会被追帧丢掉。
    ///
    /// 满员时拒绝（[`SubscribeError::TooManySubscribers`]）；要挤掉最早的连接用
    /// [`reserve`](Self::reserve) + [`subscribe_reserved`](Self::subscribe_reserved)。
    pub async fn subscribe_with_depth(
        &self,
        depth: Option<Duration>,
        timeout: Duration,
    ) -> Result<Subscription, SubscribeError> {
        let (slot, _) = self.reserve(&PreviewTicket::new(), false)?;
        self.subscribe_reserved(slot, depth, timeout).await
    }

    /// 为 `ticket` 占该路一个槽位（不等写入端）。满员时 `evict` 为真就挤掉该路最早的那条连接
    /// （它的 `ticket` 随之报告被挤掉，由持有者结束响应），并把被挤掉的是谁交回；
    /// 否则返回 [`SubscribeError::TooManySubscribers`]。不可预览时返回 `Unavailable`。
    pub fn reserve(
        &self,
        ticket: &PreviewTicket,
        evict: bool,
    ) -> Result<(PreviewSlot, Option<Evicted>), SubscribeError> {
        if let HubState::Unavailable(reason) = &*self.0.state.read().unwrap() {
            return Err(SubscribeError::Unavailable(reason.clone()));
        }
        self.0
            .subscribers
            .acquire(ticket, evict)
            .map_err(|_| SubscribeError::TooManySubscribers(self.max_subscribers()))
    }

    /// 掉队（`Lagged`）后原地重新对齐：复用 `previous` 的连接许可与快照深度，向写入端再要一份从最近
    /// 关键帧起的快照与新的接收端。丢掉的那段补不回来，但连接不断、播放器不必重连；
    /// 调用方发快照时应去掉文件头（[`Subscription::snapshot_after_header`]）。
    pub async fn resubscribe(
        &self,
        previous: Subscription,
        timeout: Duration,
    ) -> Result<Subscription, SubscribeError> {
        let Subscription { _permit, depth, .. } = previous;
        self.subscribe_reserved(_permit, depth, timeout).await
    }

    /// 用 [`reserve`](Self::reserve) 占到的槽位订阅；语义同 [`subscribe_with_depth`](Self::subscribe_with_depth)。
    pub async fn subscribe_reserved(
        &self,
        permit: PreviewSlot,
        depth: Option<Duration>,
        timeout: Duration,
    ) -> Result<Subscription, SubscribeError> {
        let request_tx = match &*self.0.requests.read().unwrap() {
            Some((_, tx)) => tx.clone(),
            None => return Err(SubscribeError::NotAttached),
        };
        let (reply_tx, reply_rx) = oneshot::channel();
        request_tx
            .send(SnapshotRequest {
                reply: reply_tx,
                depth,
            })
            .map_err(|_| SubscribeError::NotAttached)?;
        match tokio::time::timeout(timeout, reply_rx).await {
            Ok(Ok(reply)) => Ok(Subscription {
                format: reply.format,
                snapshot: reply.snapshot,
                rx: reply.rx,
                header_len: reply.header_len,
                depth,
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
    /// 当前 GOP 的关键帧到达时刻
    gop_started: Instant,
    /// 已完成的 GOP，旧的在前；与当前 GOP 连续（中间没有被丢弃的 GOP、没有换过序列头）
    history: VecDeque<Gop>,
    history_bytes: usize,
    /// 已完成 GOP 的保留时长，见 [`SNAPSHOT_WINDOW`]；零表示快照只含当前 GOP
    snapshot_window: Duration,
    /// fMP4：从 init segment 读出的视频轨信息，用于判定分片首帧是否关键帧
    fmp4_init: mp4::InitInfo,
    /// 广播缓冲里有没有数据；有且订阅者归零时换新通道释放
    ring_used: bool,
}

/// 快照里一个已完成的 GOP。
struct Gop {
    chunks: Vec<Bytes>,
    bytes: usize,
    started: Instant,
}

impl PreviewSink {
    pub fn format(&self) -> PreviewFormat {
        self.format
    }

    /// 改快照里已完成 GOP 的保留时长（默认 [`SNAPSHOT_WINDOW`]）；`Duration::ZERO` 只保留当前 GOP。
    pub fn with_snapshot_window(mut self, window: Duration) -> Self {
        self.snapshot_window = window;
        self
    }

    /// 快照当前的字节数（已完成 GOP + 当前 GOP，不含文件头与序列头）。
    pub fn snapshot_bytes(&self) -> usize {
        self.history_bytes + self.gop_bytes
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

    /// 推送一个 fMP4 分段（可能含多个 moof/mdat 对）：按分片切开，视频首帧是关键帧的分片
    /// 作为 GOP 起点，其余追加——新订阅者由此从关键帧起播，而不是从分段边界起。
    /// 读不出 flags 的分片按关键帧处理（宁可多给也不能让预览永远起不来）。
    pub fn push_fmp4_segment(&mut self, segment: Bytes) {
        let frags = mp4::fragments(&segment, &self.fmp4_init);
        if frags.is_empty() {
            self.push(ChunkKind::Keyframe, segment);
            return;
        }
        for frag in frags {
            let kind = match frag.video_sync {
                Some(false) => ChunkKind::Media,
                _ => ChunkKind::Keyframe,
            };
            self.push(kind, segment.slice(frag.start..frag.end));
        }
    }

    /// 推送一个 MPEG-TS 分片的起始块：嗅探第一个视频 PES 是否从 IDR 起，是（或说不准）
    /// 就作为 GOP 起点，否则追加到前一个 GOP。
    pub fn push_ts_segment_start(&mut self, chunk: Bytes) {
        let kind = match ts::segment_starts_with_idr(&chunk) {
            Some(false) => ChunkKind::Media,
            _ => ChunkKind::Keyframe,
        };
        self.push(kind, chunk);
    }

    /// 推送一个分块。热路径：不 await、不加锁等待、不返回错误。
    pub fn push(&mut self, kind: ChunkKind, chunk: Bytes) {
        match kind {
            ChunkKind::Header => {
                if self.format == PreviewFormat::Fmp4 {
                    // init segment：每次 attach 只出现一次（上游换初始化分片时才会再来），
                    // 解出编码串供前端 addSourceBuffer，记下视频轨用于分片的关键帧判定；
                    // MSE 允许中途追加新 init，照常广播。换了 init 的分片参数可能变了，
                    // 旧 GOP 不能再和新 init 一起给新订阅者
                    if self.header.as_ref().is_some_and(|old| *old != chunk) {
                        self.drop_history();
                    }
                    self.fmp4_init = mp4::init_info(&chunk);
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
                    Some(entry) => {
                        // 序列头内容变了（换分辨率 / 编码参数）：之前的 GOP 与新序列头不配，
                        // 不能再放进快照；当前 GOP 沿用既有行为，与新序列头一起给出
                        if entry.1 != chunk {
                            entry.1 = chunk.clone();
                            self.drop_history();
                        }
                    }
                    None => self.sequence_headers.push((slot, chunk.clone())),
                }
                self.broadcast(chunk);
            }
            ChunkKind::Keyframe => {
                let now = Instant::now();
                self.archive_gop(now);
                self.gop_ready = true;
                self.gop_started = now;
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

    /// 关键帧到来：把当前 GOP 收进历史，再按时长与字节上限修剪历史。
    fn archive_gop(&mut self, now: Instant) {
        if self.gop_ready && !self.gop.is_empty() && !self.snapshot_window.is_zero() {
            self.history_bytes += self.gop_bytes;
            self.history.push_back(Gop {
                chunks: std::mem::take(&mut self.gop),
                bytes: self.gop_bytes,
                started: self.gop_started,
            });
        } else {
            self.gop.clear();
        }
        self.gop_bytes = 0;
        while let Some(oldest) = self.history.front()
            && now.duration_since(oldest.started) > self.snapshot_window
        {
            self.pop_oldest_gop();
        }
        self.trim_history_bytes();
    }

    fn pop_oldest_gop(&mut self) {
        if let Some(gop) = self.history.pop_front() {
            self.history_bytes -= gop.bytes;
        }
    }

    /// 整份快照超过字节上限时先丢最旧的 GOP；当前 GOP 自己的上限由 `retain` 管。
    fn trim_history_bytes(&mut self) {
        while !self.history.is_empty() && self.history_bytes + self.gop_bytes > MAX_SNAPSHOT_BYTES {
            self.pop_oldest_gop();
        }
    }

    fn drop_history(&mut self) {
        self.history.clear();
        self.history_bytes = 0;
    }

    fn retain(&mut self, chunk: Bytes) {
        self.gop_bytes += chunk.len();
        if self.gop_bytes > MAX_GOP_BYTES {
            debug!(
                bytes = self.gop_bytes,
                "preview GOP exceeds the snapshot limit, waiting for the next keyframe"
            );
            // 当前 GOP 作废，历史与之不再连续，一并放掉
            self.gop.clear();
            self.gop_bytes = 0;
            self.gop_ready = false;
            self.drop_history();
        } else {
            self.gop.push(chunk);
            self.trim_history_bytes();
        }
    }

    /// 没有订阅者时 `send` 直接返回 `Err`、不存任何东西；有订阅者时满了覆盖最旧的分块。
    ///
    /// 缓冲里的分块在最后一个订阅者走后不会自动释放（tokio broadcast 只在被覆盖时丢），
    /// FLV 1024 槽就是 ~10 MB 一直挂着。所以订阅者归零时换一条新通道，把旧缓冲整个放掉；
    /// 之后的 `subscribe()` 拿的是新通道，对订阅者透明。
    fn broadcast(&mut self, chunk: Bytes) {
        if self.tx.receiver_count() == 0 {
            if self.ring_used {
                self.tx = broadcast::channel(self.format.broadcast_capacity()).0;
                self.ring_used = false;
            }
            return;
        }
        self.ring_used = true;
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
        let mut contiguous = false;
        while let Ok(request) = self.requests.try_recv() {
            if !contiguous {
                // 历史只有几个 GOP，整理成连续切片的代价可以忽略；没有请求时不做
                self.history.make_contiguous();
                contiguous = true;
            }
            let rx = self.tx.subscribe();
            let history =
                history_for_depth(self.history.as_slices().0, request.depth, Instant::now());
            let history_chunks: usize = history.iter().map(|g| g.chunks.len()).sum();
            let mut snapshot = Vec::with_capacity(
                1 + self.sequence_headers.len() + history_chunks + self.gop.len(),
            );
            snapshot.extend(self.header.iter().cloned());
            snapshot.extend(self.sequence_headers.iter().map(|(_, b)| b.clone()));
            snapshot.extend(history.iter().flat_map(|g| g.chunks.iter().cloned()));
            snapshot.extend(self.gop.iter().cloned());
            let _ = request.reply.send(SnapshotReply {
                format: self.format,
                snapshot,
                header_len: usize::from(self.header.is_some()),
                rx,
            });
        }
    }
}

/// 按要求的深度从（连续的）历史里取已完成的 GOP：`None` 全给；`Some(d)` 从最新往前取，
/// 取到第一个关键帧到达时刻早于 `now - d` 的 GOP 为止（含它，起播缓冲才不少于 `d`）。
fn history_for_depth(history: &[Gop], depth: Option<Duration>, now: Instant) -> &[Gop] {
    let Some(depth) = depth else { return history };
    if depth.is_zero() {
        return &[];
    }
    let start = history
        .iter()
        .rposition(|g| now.duration_since(g.started) >= depth)
        .unwrap_or(0);
    &history[start..]
}

impl Drop for PreviewSink {
    fn drop(&mut self) {
        let mut requests = self.hub.requests.write().unwrap();
        if matches!(&*requests, Some((generation, _)) if *generation == self.generation) {
            *requests = None;
        }
    }
}

/// MPEG-TS 分片起点的关键帧嗅探。
///
/// HLS 分片按惯例从关键帧开始，但不是所有打包器都保证。这里只看分片开头这一块字节里第一个
/// 视频 PES（stream_id 0xE0–0xEF）的前几个 NAL：有 IDR（5）或 SPS（7）→ 关键帧起点；
/// 只看到普通片（1）→ 不是；找不到视频 PES 或 NAL 不可辨 → 说不准。
pub mod ts {
    const PACKET: usize = 188;

    /// `Some(true)` 从关键帧起，`Some(false)` 不是，`None` 说不准（调用方按关键帧处理）。
    pub fn segment_starts_with_idr(chunk: &[u8]) -> Option<bool> {
        let mut es: Vec<u8> = Vec::new();
        let mut video_pid: Option<u16> = None;
        let mut offset = 0;
        while offset + PACKET <= chunk.len() {
            let pkt = &chunk[offset..offset + PACKET];
            offset += PACKET;
            if pkt[0] != 0x47 {
                return None;
            }
            let pusi = pkt[1] & 0x40 != 0;
            let pid = (u16::from(pkt[1] & 0x1f) << 8) | u16::from(pkt[2]);
            let afc = (pkt[3] >> 4) & 0x3;
            let mut payload = 4;
            if afc & 0x2 != 0 {
                payload += 1 + usize::from(pkt[4]);
            }
            if afc & 0x1 == 0 || payload > PACKET {
                continue;
            }
            let data = &pkt[payload..];
            match video_pid {
                None => {
                    // 还没锁定视频 PID：找 PES 起始且 stream_id 是视频的包
                    if pusi
                        && data.len() > 9
                        && data[0] == 0
                        && data[1] == 0
                        && data[2] == 1
                        && (0xe0..=0xef).contains(&data[3])
                    {
                        let header_len = usize::from(data[8]);
                        video_pid = Some(pid);
                        es.extend_from_slice(data.get(9 + header_len..).unwrap_or(&[]));
                    }
                }
                Some(v) if v == pid => {
                    if pusi {
                        break; // 第一个视频 PES 到此为止
                    }
                    es.extend_from_slice(data);
                }
                _ => {}
            }
            if es.len() > 64 * 1024 {
                break;
            }
        }
        video_pid?;
        let mut verdict = None;
        let mut i = 0;
        while i + 4 <= es.len() {
            if es[i] == 0 && es[i + 1] == 0 && es[i + 2] == 1 {
                let nal_type = es[i + 3] & 0x1f;
                match nal_type {
                    5 | 7 => return Some(true),
                    1 => verdict = Some(false),
                    _ => {}
                }
                i += 3;
            } else {
                i += 1;
            }
        }
        verdict
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
    /// 一个 box 的类型与载荷（不含 8 / 16 字节头），以及含头的总长度
    struct Box<'a> {
        kind: [u8; 4],
        body: &'a [u8],
        size: usize,
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
            Some(Box { kind, body, size })
        })
    }

    fn child<'a>(data: &'a [u8], kind: &[u8; 4]) -> Option<&'a [u8]> {
        boxes(data).find(|b| &b.kind == kind).map(|b| b.body)
    }

    /// `stsd` 是 full box：version/flags(4) + entry_count(4)，其后是 sample entry 列表
    fn sample_entries(stsd: &[u8]) -> impl Iterator<Item = Box<'_>> {
        boxes(stsd.get(8..).unwrap_or(&[]))
    }

    /// init segment 里与分片切分相关的信息：视频轨 ID 与 `trex` 里的默认 sample flags。
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    pub struct InitInfo {
        /// `hdlr.handler_type == "vide"` 的轨道 ID；纯音频流为 `None`
        pub video_track: Option<u32>,
        /// `mvex/trex` 给该视频轨的 default_sample_flags（分片自身没写 flags 时用）
        pub video_default_flags: Option<u32>,
    }

    /// 从 init segment 读视频轨 ID 与默认 sample flags。
    pub fn init_info(init: &[u8]) -> InitInfo {
        let Some(moov) = child(init, b"moov") else {
            return InitInfo::default();
        };
        let mut info = InitInfo::default();
        for trak in boxes(moov).filter(|b| &b.kind == b"trak") {
            let is_video = child(trak.body, b"mdia")
                .and_then(|mdia| child(mdia, b"hdlr"))
                .and_then(|hdlr| hdlr.get(8..12))
                .is_some_and(|handler| handler == b"vide");
            if !is_video {
                continue;
            }
            // tkhd 是 full box：version 0 时 track_ID 在偏移 12，version 1 在 20
            let track_id = child(trak.body, b"tkhd").and_then(|tkhd| {
                let offset = if tkhd.first() == Some(&1) { 20 } else { 12 };
                tkhd.get(offset..offset + 4)
                    .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
            });
            info.video_track = track_id;
            break;
        }
        if let (Some(video_track), Some(mvex)) = (info.video_track, child(moov, b"mvex")) {
            for trex in boxes(mvex).filter(|b| &b.kind == b"trex") {
                // full box：version/flags(4) track_ID(4) default_sample_description_index(4)
                // default_sample_duration(4) default_sample_size(4) default_sample_flags(4)
                let id = trex.body.get(4..8).map(be_u32);
                if id == Some(video_track) {
                    info.video_default_flags = trex.body.get(20..24).map(be_u32);
                }
            }
        }
        info
    }

    fn be_u32(b: &[u8]) -> u32 {
        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
    }

    /// 一个 moof + mdat 分片在分段字节里的位置，以及它的视频首帧是否为同步样本（关键帧）。
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Fragment {
        pub start: usize,
        pub end: usize,
        /// `Some(true)` 首帧是关键帧；`Some(false)` 不是；`None` 分片里没有视频轨或读不出 flags
        pub video_sync: Option<bool>,
    }

    /// 把一个 HLS 分段（可能含多个 moof/mdat 对）切成分片。`styp` / `sidx` / `prft` / `emsg`
    /// 之类的前导 box 归入紧随其后的分片；`moof` 之后到下一个 `moof`（或下一个前导 box）之前的
    /// 全部 box 都算这个分片。
    pub fn fragments(segment: &[u8], info: &InitInfo) -> Vec<Fragment> {
        let mut out: Vec<Fragment> = Vec::new();
        let mut pending_start: Option<usize> = None;
        let mut current: Option<Fragment> = None;
        let mut offset = 0usize;
        for b in boxes(segment) {
            let size = b.size;
            match &b.kind {
                b"moof" => {
                    if let Some(frag) = current.take() {
                        out.push(frag);
                    }
                    let start = pending_start.take().unwrap_or(offset);
                    current = Some(Fragment {
                        start,
                        end: offset + size,
                        video_sync: moof_video_sync(b.body, info),
                    });
                }
                b"styp" | b"sidx" | b"prft" | b"emsg" => {
                    if let Some(frag) = current.take() {
                        out.push(frag);
                    }
                    pending_start.get_or_insert(offset);
                }
                _ => match current.as_mut() {
                    Some(frag) => frag.end = offset + size,
                    None => {
                        pending_start.get_or_insert(offset);
                    }
                },
            }
            offset += size;
        }
        if let Some(frag) = current.take() {
            out.push(frag);
        }
        // 尾部只有前导 box 没有 moof（分段被截断）：并入最后一个分片
        if pending_start.is_some()
            && let Some(last) = out.last_mut()
        {
            last.end = offset;
        }
        out
    }

    /// 从 moof 里视频轨的 traf 读首个样本的 flags，判定是否关键帧。
    fn moof_video_sync(moof: &[u8], info: &InitInfo) -> Option<bool> {
        for traf in boxes(moof).filter(|b| &b.kind == b"traf") {
            let tfhd = child(traf.body, b"tfhd")?;
            let tfhd_flags = be_u32(tfhd.get(0..4)?) & 0x00ff_ffff;
            let track_id = be_u32(tfhd.get(4..8)?);
            if info.video_track.is_some_and(|v| v != track_id) {
                continue;
            }
            // tfhd 可选字段顺序：base_data_offset(8) sample_description_index(4)
            // default_sample_duration(4) default_sample_size(4) default_sample_flags(4)
            let mut pos = 8;
            if tfhd_flags & 0x1 != 0 {
                pos += 8;
            }
            if tfhd_flags & 0x2 != 0 {
                pos += 4;
            }
            if tfhd_flags & 0x8 != 0 {
                pos += 4;
            }
            if tfhd_flags & 0x10 != 0 {
                pos += 4;
            }
            let default_flags = if tfhd_flags & 0x20 != 0 {
                tfhd.get(pos..pos + 4).map(be_u32)
            } else {
                info.video_default_flags
            };

            let first_flags = child(traf.body, b"trun").and_then(|trun| {
                let trun_flags = be_u32(trun.get(0..4)?) & 0x00ff_ffff;
                let mut pos = 8; // version/flags + sample_count
                if trun_flags & 0x1 != 0 {
                    pos += 4; // data_offset
                }
                if trun_flags & 0x4 != 0 {
                    return trun.get(pos..pos + 4).map(be_u32); // first_sample_flags
                }
                if trun_flags & 0x400 != 0 {
                    // 第一个样本条目：duration(0x100) size(0x200) flags(0x400) cto(0x800)
                    if trun_flags & 0x100 != 0 {
                        pos += 4;
                    }
                    if trun_flags & 0x200 != 0 {
                        pos += 4;
                    }
                    return trun.get(pos..pos + 4).map(be_u32);
                }
                None
            });
            // sample_is_non_sync_sample 是 bit 16；sample_depends_on == 2 也表示 I 帧
            return first_flags.or(default_flags).map(|flags| {
                let non_sync = (flags >> 16) & 1 == 1;
                let depends_on = (flags >> 24) & 0x3;
                !non_sync || depends_on == 2
            });
        }
        None
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

    /// 造一个 moof：一个视频 traf（tfhd 带 default_sample_flags，trun 带 first_sample_flags）
    fn moof(track: u32, first_sample_flags: Option<u32>, default_flags: Option<u32>) -> Vec<u8> {
        use mp4::build::*;
        let mut tfhd = vec![0u8; 4];
        let mut tfhd_flags = 0u32;
        let mut tail = Vec::new();
        if let Some(d) = default_flags {
            tfhd_flags |= 0x20;
            tail.extend_from_slice(&d.to_be_bytes());
        }
        tfhd[1..4].copy_from_slice(&tfhd_flags.to_be_bytes()[1..]);
        tfhd.extend_from_slice(&track.to_be_bytes());
        tfhd.extend_from_slice(&tail);
        let mut trun = vec![0u8; 4];
        let mut trun_flags = 0x1u32 | 0x200; // data_offset + sample_size
        if first_sample_flags.is_some() {
            trun_flags |= 0x4;
        }
        trun[1..4].copy_from_slice(&trun_flags.to_be_bytes()[1..]);
        trun.extend_from_slice(&2u32.to_be_bytes()); // sample_count
        trun.extend_from_slice(&0u32.to_be_bytes()); // data_offset
        if let Some(f) = first_sample_flags {
            trun.extend_from_slice(&f.to_be_bytes());
        }
        trun.extend_from_slice(&[0, 0, 0, 10, 0, 0, 0, 20]);
        let traf = bx(b"traf", &[bx(b"tfhd", &tfhd), bx(b"trun", &trun)].concat());
        bx(b"moof", &[bx(b"mfhd", &[0u8; 8]), traf].concat())
    }

    /// 最后一个订阅者走后，广播缓冲里的分块要被放掉（换新通道），之后的订阅者照常工作。
    #[tokio::test]
    async fn broadcast_ring_is_released_when_the_last_subscriber_leaves() {
        let hub = PreviewHub::new(4);
        // 只看当前 GOP，快照历史另有测试
        let mut sink = hub
            .attach(PreviewFormat::Flv)
            .with_snapshot_window(Duration::ZERO);
        sink.push(ChunkKind::Header, Bytes::from_static(b"FLV"));
        sink.push(ChunkKind::Keyframe, Bytes::from_static(b"K0"));
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        let big = Bytes::from(vec![7u8; 1024 * 1024]);
        sink.push(ChunkKind::Keyframe, Bytes::from_static(b"K1"));
        sink.push(ChunkKind::Media, big.clone());
        let sub = pending.await.unwrap().unwrap();
        assert!(sink.ring_used);
        // 订阅者走了：下一次 push 发现没人收，换新通道；旧通道（及其 1 MB）随之释放
        drop(sub);
        sink.push(ChunkKind::Media, Bytes::from_static(b"m"));
        assert!(!sink.ring_used);
        assert_eq!(sink.tx.receiver_count(), 0);
        // 再来一个订阅者，走新通道照常收
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Keyframe, Bytes::from_static(b"K2"));
        sink.push(ChunkKind::Media, Bytes::from_static(b"m2"));
        let mut sub = pending.await.unwrap().unwrap();
        assert_eq!(
            sub.snapshot,
            vec![Bytes::from_static(b"FLV"), Bytes::from_static(b"K2")]
        );
        assert_eq!(sub.rx.recv().await.unwrap(), Bytes::from_static(b"m2"));
    }

    /// CDN 整 GOP 突发：请求在关键帧被回应后，同一突发里紧跟几百个 tag 在订阅者读之前就全部
    /// 推完。FLV 缓冲要装得下这种突发，订阅者随后逐个读到而不是 `Lagged`。
    #[tokio::test]
    async fn a_whole_gop_burst_does_not_lag_a_fresh_flv_subscriber() {
        let hub = PreviewHub::new(4);
        let mut sink = hub
            .attach(PreviewFormat::Flv)
            .with_snapshot_window(Duration::ZERO);
        sink.push(ChunkKind::Header, Bytes::from_static(b"FLV"));
        sink.push(ChunkKind::Keyframe, Bytes::from_static(b"K0"));
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        // 关键帧回应请求，然后同一突发里再来 600 个 tag（斗鱼实测一个 GOP ~320 个）
        sink.push(ChunkKind::Keyframe, Bytes::from_static(b"K1"));
        for i in 0..600u32 {
            sink.push(ChunkKind::Media, Bytes::from(i.to_be_bytes().to_vec()));
        }
        let mut sub = pending.await.unwrap().unwrap();
        assert_eq!(
            sub.snapshot,
            vec![Bytes::from_static(b"FLV"), Bytes::from_static(b"K1")]
        );
        for i in 0..600u32 {
            assert_eq!(
                sub.rx.recv().await.unwrap(),
                Bytes::from(i.to_be_bytes().to_vec())
            );
        }
        // TS / fMP4 的分块大而少，沿用 256 槽
        assert_eq!(
            PreviewFormat::MpegTs.broadcast_capacity(),
            BROADCAST_CAPACITY_SEGMENTED
        );
        assert_eq!(
            PreviewFormat::Fmp4.broadcast_capacity(),
            BROADCAST_CAPACITY_SEGMENTED
        );
    }

    #[test]
    fn fmp4_segments_are_split_into_fragments_with_keyframe_detection() {
        use mp4::build::*;
        // init：视频轨 1（vide）、音频轨 2（soun），trex 给视频默认 non-sync
        let hdlr = |kind: &[u8; 4]| {
            // full box：version/flags 由 full() 加，这里从 pre_defined(4) 开始
            let mut b = vec![0u8; 4];
            b.extend_from_slice(kind);
            b.extend_from_slice(&[0u8; 13]);
            full(b"hdlr", &b)
        };
        let tkhd = |id: u32| {
            let mut b = vec![0u8; 8];
            b.extend_from_slice(&id.to_be_bytes());
            b.extend_from_slice(&[0u8; 64]);
            full(b"tkhd", &b)
        };
        let trak =
            |id: u32, kind: &[u8; 4]| bx(b"trak", &[tkhd(id), bx(b"mdia", &hdlr(kind))].concat());
        let trex = |id: u32, flags: u32| {
            let mut b = vec![0u8; 4];
            b.extend_from_slice(&id.to_be_bytes());
            b.extend_from_slice(&[0u8; 12]);
            b.extend_from_slice(&flags.to_be_bytes());
            bx(b"trex", &b)
        };
        let moov = bx(
            b"moov",
            &[
                trak(1, b"vide"),
                trak(2, b"soun"),
                bx(
                    b"mvex",
                    &[trex(1, 0x0101_0000), trex(2, 0x0200_0000)].concat(),
                ),
            ]
            .concat(),
        );
        let init = [bx(b"ftyp", b"iso5"), moov].concat();
        let info = mp4::init_info(&init);
        assert_eq!(
            info,
            mp4::InitInfo {
                video_track: Some(1),
                video_default_flags: Some(0x0101_0000)
            }
        );

        // 分段：styp + [非同步分片] + [同步分片（depends_on=2）] + [没写 flags 的分片→用 trex 默认]
        // + 一个只有音频轨的分片（视频轨缺席 → None）
        let mdat = bx(b"mdat", &[0xaa; 16]);
        let non_sync = [moof(1, Some(0x0101_0000), None), mdat.clone()].concat();
        let sync = [moof(1, Some(0x0200_0000), None), mdat.clone()].concat();
        let by_default = [moof(1, None, None), mdat.clone()].concat();
        let audio_only = [moof(2, Some(0x0200_0000), None), mdat.clone()].concat();
        let styp = bx(b"styp", b"msdh");
        let segment = [
            styp.clone(),
            non_sync.clone(),
            sync.clone(),
            by_default.clone(),
            audio_only.clone(),
        ]
        .concat();
        let frags = mp4::fragments(&segment, &info);
        assert_eq!(frags.len(), 4);
        // 前导 styp 归入第一个分片
        assert_eq!(
            (frags[0].start, frags[0].end),
            (0, styp.len() + non_sync.len())
        );
        assert_eq!(frags[0].video_sync, Some(false));
        assert_eq!(frags[1].video_sync, Some(true));
        assert_eq!(frags[2].video_sync, Some(false), "trex 默认 non-sync");
        assert_eq!(frags[3].video_sync, None, "没有视频轨的分片说不准");
        assert_eq!(frags[3].end, segment.len());
        // 不知道视频轨时用第一个有 flags 的 traf
        let frags = mp4::fragments(&sync, &mp4::InitInfo::default());
        assert_eq!(frags[0].video_sync, Some(true));
        // 损坏 / 空数据不 panic
        assert!(mp4::fragments(&[0, 0, 0], &info).is_empty());
        assert!(mp4::fragments(b"", &info).is_empty());
    }

    /// 写入端按分片切分：新订阅者从最近的关键帧分片起，而不是从分段边界起。
    #[tokio::test]
    async fn fmp4_snapshot_starts_at_the_last_keyframe_fragment() {
        use mp4::build::*;
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Fmp4);
        let avcc = bx(b"avcC", &[1, 0x64, 0x00, 0x1f]);
        let init = Bytes::from(init(&[trak(&stsd(&[visual_entry(b"avc1", &avcc)]))]));
        sink.push(ChunkKind::Header, init.clone());
        let mdat = |n: u8| bx(b"mdat", &[n; 8]);
        let key = |n: u8| Bytes::from([moof(1, Some(0x0200_0000), None), mdat(n)].concat());
        let inter = |n: u8| Bytes::from([moof(1, Some(0x0101_0000), None), mdat(n)].concat());
        // 分段 A = [P1][P2][K3][P4]，分段 B = [P5][P6]
        let seg_a = Bytes::from([inter(1), inter(2), key(3), inter(4)].concat());
        let seg_b = Bytes::from([inter(5), inter(6)].concat());
        sink.push_fmp4_segment(seg_a);
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push_fmp4_segment(seg_b);
        let mut sub = pending.await.unwrap().unwrap();
        // 请求在分段 B 的第一个分片 P5 推送时被回应：快照 = init + K3 P4 P5——从关键帧分片起、
        // 跨过分段 A/B 的边界；P6 走实时接收端
        assert_eq!(sub.snapshot, vec![init, key(3), inter(4), inter(5)]);
        assert_eq!(sub.rx.recv().await.unwrap(), inter(6));
    }

    #[test]
    fn ts_segment_start_sniffs_the_first_video_pes() {
        // 造 TS 包：PAT 无关；视频 PID 0x100，PUSI 包里放 PES 头 + NAL
        fn packet(pid: u16, pusi: bool, cc: u8, payload: &[u8]) -> Vec<u8> {
            let mut p = vec![
                0x47,
                ((pid >> 8) as u8 & 0x1f) | if pusi { 0x40 } else { 0 },
                pid as u8,
                0x10 | (cc & 0xf),
            ];
            p.extend_from_slice(payload);
            p.resize(188, 0xff);
            p
        }
        fn pes(nals: &[&[u8]]) -> Vec<u8> {
            let mut es = Vec::new();
            for n in nals {
                es.extend_from_slice(&[0, 0, 0, 1]);
                es.extend_from_slice(n);
            }
            let mut p = vec![0, 0, 1, 0xe0, 0, 0, 0x80, 0x80, 5, 0x21, 0, 1, 0, 1];
            p.extend_from_slice(&es);
            p
        }
        // 音频 PES 先出现，再是视频 PES：SPS + PPS + IDR → 关键帧起点
        let audio = packet(
            0x101,
            true,
            0,
            &[
                0, 0, 1, 0xc0, 0, 10, 0x80, 0x80, 5, 0, 0, 0, 0, 0, 0xff, 0xf1,
            ],
        );
        let idr = packet(
            0x100,
            true,
            0,
            &pes(&[&[0x67, 1, 2], &[0x68, 3], &[0x65, 0x88]]),
        );
        let chunk = [audio.clone(), idr].concat();
        assert_eq!(ts::segment_starts_with_idr(&chunk), Some(true));
        // 只有普通片
        let p_frame = packet(0x100, true, 0, &pes(&[&[0x41, 0x9a]]));
        assert_eq!(
            ts::segment_starts_with_idr(&[audio.clone(), p_frame].concat()),
            Some(false)
        );
        // 普通片跨包，下一个 PUSI 之前继续读；再后面的 IDR 属于第二个 PES，不算
        let p1 = packet(0x100, true, 0, &pes(&[&[0x41, 0x9a]]));
        let p2 = packet(0x100, false, 1, &[0x9a; 100]);
        let k = packet(0x100, true, 2, &pes(&[&[0x65, 1]]));
        assert_eq!(
            ts::segment_starts_with_idr(&[p1, p2, k].concat()),
            Some(false)
        );
        // 没有视频 PES / 不是 TS → 说不准
        assert_eq!(ts::segment_starts_with_idr(&audio), None);
        assert_eq!(ts::segment_starts_with_idr(b"FLV\x01"), None);
        assert_eq!(ts::segment_starts_with_idr(&[]), None);
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
        // 快照 = init + 窗口内的关键帧分片（两个都刚到，都在）
        assert_eq!(sub.snapshot, vec![init.clone(), segment, segment2]);
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
                reason: None,
                subscribers: 0,
                max_subscribers: 4,
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
        assert_eq!(status.max_subscribers, 0);
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

    /// 快照 = 文件头 + 序列头 + 窗口内的已完成 GOP + 当前 GOP，随后与实时分块严格衔接。
    /// 新订阅者由此从起播就有几秒缓冲，而不是从零到一个 GOP 之间起步。
    #[tokio::test]
    async fn snapshot_keeps_recent_gops_within_the_window() {
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Flv);
        sink.push(ChunkKind::Header, Bytes::from_static(&flv::FILE_HEADER));
        sink.push(ChunkKind::SequenceHeader(9), Bytes::from_static(b"avc"));
        sink.push(ChunkKind::Media, inter(0)); // 关键帧之前的不算
        sink.push(ChunkKind::Keyframe, key(1));
        sink.push(ChunkKind::Media, inter(2));
        sink.push(ChunkKind::Keyframe, key(3));
        sink.push(ChunkKind::Media, inter(4));
        sink.push(ChunkKind::Keyframe, key(5));
        assert_eq!(sink.history.len(), 2);
        assert_eq!(sink.snapshot_bytes(), 3 * key(0).len() + 2 * inter(0).len());
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, inter(6));
        let mut sub = pending.await.unwrap().unwrap();
        assert_eq!(
            sub.snapshot,
            vec![
                Bytes::from_static(&flv::FILE_HEADER),
                Bytes::from_static(b"avc"),
                key(1),
                inter(2),
                key(3),
                inter(4),
                key(5),
                inter(6),
            ]
        );
        sink.push(ChunkKind::Keyframe, key(7));
        assert_eq!(sub.rx.recv().await.unwrap(), key(7));
    }

    /// 按深度取历史：从最新往前，取到第一个早于「现在 − 深度」的 GOP 为止（含）；
    /// 零深度只给当前 GOP；`None` 给全部；深度大于全部历史时也给全部。
    #[test]
    fn history_is_cut_by_requested_depth() {
        let now = Instant::now();
        let gop = |age_ms: u64, k: u8| Gop {
            chunks: vec![key(k)],
            bytes: key(k).len(),
            started: now - Duration::from_millis(age_ms),
        };
        // 关键帧分别在 5.0 / 3.0 / 1.0 s 前到达（当前 GOP 不在历史里）
        let history = vec![gop(5000, 1), gop(3000, 3), gop(1000, 5)];
        let picked = |depth: Option<Duration>| -> Vec<Bytes> {
            history_for_depth(&history, depth, now)
                .iter()
                .map(|g| g.chunks[0].clone())
                .collect()
        };
        assert_eq!(picked(None), vec![key(1), key(3), key(5)]);
        assert_eq!(picked(Some(Duration::ZERO)), Vec::<Bytes>::new());
        // 要 2 s：K5 只有 1 s，再往前 K3（3 s 前）跨过了 2 s 的线，取到它为止
        assert_eq!(picked(Some(Duration::from_secs(2))), vec![key(3), key(5)]);
        // 要 3 s：K3 正好 3 s 前到达（>=），取到它
        assert_eq!(picked(Some(Duration::from_secs(3))), vec![key(3), key(5)]);
        assert_eq!(
            picked(Some(Duration::from_millis(3500))),
            vec![key(1), key(3), key(5)]
        );
        assert_eq!(
            picked(Some(Duration::from_secs(30))),
            vec![key(1), key(3), key(5)]
        );
        assert_eq!(picked(Some(Duration::from_millis(500))), vec![key(5)]);
        assert!(history_for_depth(&[], Some(Duration::from_secs(2)), now).is_empty());
    }

    /// 端到端：带深度订阅拿到的快照只含回溯范围内的 GOP，掉队重对齐时沿用同一深度。
    #[tokio::test]
    async fn subscribe_with_depth_trims_the_snapshot() {
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Flv);
        sink.push(ChunkKind::Header, Bytes::from_static(&flv::FILE_HEADER));
        sink.push(ChunkKind::Keyframe, key(1));
        sink.push(ChunkKind::Media, inter(2));
        tokio::time::sleep(Duration::from_millis(60)).await;
        sink.push(ChunkKind::Keyframe, key(3));
        sink.push(ChunkKind::Media, inter(4));
        tokio::time::sleep(Duration::from_millis(60)).await;
        sink.push(ChunkKind::Keyframe, key(5));
        assert_eq!(sink.history.len(), 2);
        // 要 30 ms：K3 的 GOP（60 ms 前开始）是第一个跨过线的，K1 不给
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move {
                hub.subscribe_with_depth(Some(Duration::from_millis(30)), Duration::from_secs(5))
                    .await
            }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, inter(6));
        let sub = pending.await.unwrap().unwrap();
        assert_eq!(
            sub.snapshot,
            vec![
                Bytes::from_static(&flv::FILE_HEADER),
                key(3),
                inter(4),
                key(5),
                inter(6)
            ]
        );
        assert_eq!(sub.depth, Some(Duration::from_millis(30)));
        // 零深度：只有当前 GOP
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move {
                hub.subscribe_with_depth(Some(Duration::ZERO), Duration::from_secs(5))
                    .await
            }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, inter(7));
        let sub0 = pending.await.unwrap().unwrap();
        assert_eq!(
            sub0.snapshot,
            vec![
                Bytes::from_static(&flv::FILE_HEADER),
                key(5),
                inter(6),
                inter(7)
            ]
        );
        // 重对齐沿用深度
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.resubscribe(sub0, Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, inter(8));
        let again = pending.await.unwrap().unwrap();
        assert_eq!(again.depth, Some(Duration::ZERO));
        assert_eq!(
            again.snapshot_after_header(),
            &[key(5), inter(6), inter(7), inter(8)]
        );
    }

    /// 超出窗口时长的 GOP 在下一个关键帧到来时被丢掉（按关键帧到达时刻算）。
    #[test]
    fn gops_older_than_the_window_are_dropped_at_the_next_keyframe() {
        let hub = PreviewHub::new(4);
        let mut sink = hub
            .attach(PreviewFormat::Flv)
            .with_snapshot_window(Duration::from_millis(30));
        sink.push(ChunkKind::Keyframe, key(1));
        sink.push(ChunkKind::Media, inter(2));
        std::thread::sleep(Duration::from_millis(80));
        sink.push(ChunkKind::Keyframe, key(3));
        // K1 的 GOP 已经 80 ms 前开始，超出 30 ms 的窗口
        assert!(sink.history.is_empty());
        assert_eq!(sink.snapshot_bytes(), key(3).len());
        sink.push(ChunkKind::Keyframe, key(4));
        // K3 刚开始不久，留下
        assert_eq!(sink.history.len(), 1);
        assert_eq!(sink.history[0].chunks, vec![key(3)]);
    }

    /// 整份快照超过字节上限时先丢最旧的 GOP，当前 GOP 保留。
    #[test]
    fn snapshot_history_is_bounded_by_bytes() {
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Flv);
        let big = Bytes::from(vec![0u8; MAX_SNAPSHOT_BYTES / 4 + 1]);
        for i in 0..6u8 {
            sink.push(ChunkKind::Keyframe, key(i));
            sink.push(ChunkKind::Media, big.clone());
        }
        // 每个 GOP 略大于 1/4 上限：最多 3 个能同时在快照里
        assert!(sink.snapshot_bytes() <= MAX_SNAPSHOT_BYTES);
        assert_eq!(sink.history.len(), 2);
        assert_eq!(sink.history[0].chunks[0], key(3));
        assert_eq!(sink.gop[0], key(5));
    }

    /// 序列头内容变了（换分辨率）：之前的 GOP 与新序列头不配，从快照里去掉；
    /// 内容相同的重发（stream-gears 分段时重放 onMetaData / 序列头）不影响历史。
    #[test]
    fn changed_sequence_header_drops_older_gops_but_a_repeat_does_not() {
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Flv);
        sink.push(ChunkKind::SequenceHeader(9), Bytes::from_static(b"avc"));
        sink.push(ChunkKind::Keyframe, key(1));
        sink.push(ChunkKind::Keyframe, key(2));
        sink.push(ChunkKind::SequenceHeader(9), Bytes::from_static(b"avc"));
        assert_eq!(sink.history.len(), 1, "identical re-send keeps history");
        sink.push(
            ChunkKind::SequenceHeader(9),
            Bytes::from_static(b"avc-1080p"),
        );
        assert!(sink.history.is_empty(), "changed header drops history");
        assert_eq!(sink.gop, vec![key(2)], "current GOP is kept as before");
    }

    /// 当前 GOP 超限作废时历史与之不再连续，一并放掉，等下一个关键帧从头攒。
    #[test]
    fn oversized_gop_also_drops_the_history() {
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Flv);
        sink.push(ChunkKind::Keyframe, key(1));
        sink.push(ChunkKind::Keyframe, key(2));
        assert_eq!(sink.history.len(), 1);
        sink.push(ChunkKind::Media, Bytes::from(vec![0u8; MAX_GOP_BYTES + 1]));
        assert!(!sink.gop_ready);
        assert!(sink.history.is_empty());
        assert_eq!(sink.snapshot_bytes(), 0);
        sink.push(ChunkKind::Keyframe, key(3));
        assert!(sink.gop_ready);
        assert!(sink.history.is_empty());
        assert_eq!(sink.gop, vec![key(3)]);
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

    /// 掉队后 `resubscribe`：复用同一个许可（不占第二个名额），拿到从最近关键帧起的新快照
    /// 与新接收端；`snapshot_after_header` 去掉文件头供同一条响应续播。
    #[tokio::test]
    async fn resubscribe_reuses_the_permit_and_realigns_at_a_keyframe() {
        let hub = PreviewHub::new(1);
        let mut sink = hub
            .attach(PreviewFormat::Flv)
            .with_snapshot_window(Duration::ZERO);
        sink.push(ChunkKind::Header, Bytes::from_static(&flv::FILE_HEADER));
        sink.push(ChunkKind::SequenceHeader(9), Bytes::from_static(b"avc"));
        sink.push(ChunkKind::Keyframe, key(1));
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, inter(2));
        let mut sub = pending.await.unwrap().unwrap();
        assert_eq!(hub.subscribers(), 1);
        // 名额已满，第二个订阅被拒
        assert_eq!(
            hub.subscribe(Duration::from_secs(1)).await.err(),
            Some(SubscribeError::TooManySubscribers(1))
        );
        // 订阅者不读，写入端推满整个缓冲再多一些 → 掉队
        for i in 0..(BROADCAST_CAPACITY_FLV + 300) {
            sink.push(ChunkKind::Media, inter((i % 200) as u8));
        }
        assert!(matches!(sub.rx.recv().await, Err(RecvError::Lagged(_))));

        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.resubscribe(sub, Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Keyframe, key(9));
        let mut again = pending.await.unwrap().unwrap();
        assert_eq!(hub.subscribers(), 1, "same permit");
        assert_eq!(sink.receiver_count(), 1, "old receiver is gone");
        assert_eq!(
            again.snapshot,
            vec![
                Bytes::from_static(&flv::FILE_HEADER),
                Bytes::from_static(b"avc"),
                key(9)
            ]
        );
        assert_eq!(
            again.snapshot_after_header(),
            &[Bytes::from_static(b"avc"), key(9)]
        );
        sink.push(ChunkKind::Media, inter(10));
        assert_eq!(again.rx.recv().await.unwrap(), inter(10));
        drop(again);
        assert_eq!(hub.subscribers(), 0);
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

    /// 满员且允许挤：最早的那张票被挤掉（并得知是哪一级挤的），槽位数不变；
    /// 被挤掉的连接晚些 drop 槽位时不会把新来的那条算掉。不许挤时照旧报当前占用数。
    #[tokio::test]
    async fn full_slots_evict_the_oldest_ticket_and_stay_fixed_size() {
        let slots = PreviewSlots::new(SlotScope::Room, 2);
        let (a, b, c) = (
            PreviewTicket::new(),
            PreviewTicket::new(),
            PreviewTicket::new(),
        );
        let (slot_a, none) = slots.acquire(&a, true).unwrap();
        assert!(none.is_none());
        let (slot_b, _) = slots.acquire(&b, true).unwrap();
        assert_eq!(slots.occupied(), 2);

        assert_eq!(slots.acquire(&c, false).err(), Some(2));
        assert_eq!(a.evicted_by(), None, "a refusal evicts nobody");

        let (slot_c, evicted) = slots.acquire(&c, true).unwrap();
        assert_eq!(evicted.map(|e| e.id), Some(a.id()));
        assert_eq!(a.evicted_by(), Some(SlotScope::Room));
        assert_eq!(b.evicted_by(), None);
        assert_eq!(c.evicted_by(), None);
        assert_eq!(slots.occupied(), 2);
        // 被挤掉的那条随时能得知（已经被挤掉，立刻完成）
        let scope = tokio::time::timeout(Duration::from_secs(1), a.evicted())
            .await
            .unwrap();
        assert_eq!(scope, SlotScope::Room);
        // 被挤掉的连接收尾时归还槽位：空操作
        drop(slot_a);
        assert_eq!(slots.occupied(), 2);
        drop(slot_b);
        assert_eq!(slots.occupied(), 1);
        drop(slot_c);
        assert_eq!(slots.occupied(), 0);
    }

    /// 一张票同时占直播间与进程两级：直播间一级挤掉它以后，它在进程一级的位置立刻不算数，
    /// 新连接进进程池不必再挤掉另一个直播间的观众。
    #[test]
    fn a_ticket_evicted_by_one_pool_frees_its_place_in_the_other() {
        let room = PreviewSlots::new(SlotScope::Room, 1);
        let process = PreviewSlots::new(SlotScope::Process, 2);
        let (old, other_room, new) = (
            PreviewTicket::new(),
            PreviewTicket::new(),
            PreviewTicket::new(),
        );
        let _old_room = room.acquire(&old, true).unwrap();
        let _old_process = process.acquire(&old, true).unwrap();
        let _other = process.acquire(&other_room, true).unwrap();
        assert_eq!(process.occupied(), 2);

        let (_new_room, evicted) = room.acquire(&new, true).unwrap();
        assert_eq!(evicted.map(|e| e.id), Some(old.id()));
        assert_eq!(process.occupied(), 1, "the evicted ticket no longer counts");
        let (_new_process, evicted) = process.acquire(&new, true).unwrap();
        assert!(evicted.is_none());
        assert_eq!(other_room.evicted_by(), None);
        assert_eq!(process.occupied(), 2);
    }

    /// hub 一级：`reserve(evict = true)` 在满员时挤掉最早的订阅，新订阅照常拿到快照；
    /// 被挤掉的订阅 drop 后占用数不变，两条都 drop 后归零。`evict = false` 维持拒绝。
    #[tokio::test]
    async fn hub_reserve_evicts_the_oldest_subscription_when_full() {
        let hub = PreviewHub::new(1);
        let mut sink = hub.attach(PreviewFormat::Flv);
        sink.push(ChunkKind::Keyframe, key(1));
        let subscribe = |hub: PreviewHub, ticket: PreviewTicket| {
            tokio::spawn(async move {
                let (slot, evicted) = hub.reserve(&ticket, true)?;
                let sub = hub
                    .subscribe_reserved(slot, None, Duration::from_secs(5))
                    .await?;
                Ok::<_, SubscribeError>((sub, evicted))
            })
        };
        let first_ticket = PreviewTicket::new();
        let first = subscribe(hub.clone(), first_ticket.clone());
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, inter(2));
        let (first, evicted) = first.await.unwrap().unwrap();
        assert!(evicted.is_none());
        assert_eq!(hub.status().subscribers, 1);

        assert_eq!(
            hub.reserve(&PreviewTicket::new(), false).err(),
            Some(SubscribeError::TooManySubscribers(1))
        );

        let second = subscribe(hub.clone(), PreviewTicket::new());
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, inter(3));
        let (second, evicted) = second.await.unwrap().unwrap();
        assert_eq!(evicted.map(|e| e.id), Some(first_ticket.id()));
        assert_eq!(first_ticket.evicted_by(), Some(SlotScope::Room));
        assert_eq!(hub.subscribers(), 1);
        drop(first);
        assert_eq!(hub.subscribers(), 1);
        drop(second);
        assert_eq!(hub.subscribers(), 0);

        let unavailable = PreviewHub::unavailable("ffmpeg");
        assert!(matches!(
            unavailable.reserve(&PreviewTicket::new(), true),
            Err(SubscribeError::Unavailable(_))
        ));
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

        let total = PreviewFormat::Flv.broadcast_capacity() * 20;
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

    /// 真实 B 站 hls_fmp4 抓样：分段里多个 moof/mdat，只有约每 2 秒一个分片首帧是关键帧。
    /// 样本来自 `GET /v1/streamers/{id}/live` 抓的前 5 秒（本地有文件时才跑）。
    #[test]
    #[ignore]
    fn bilibili_fmp4_capture_has_keyframe_fragments() {
        let Ok(bytes) = std::fs::read("/tmp/lp-live3/cold/cold-3-1.bin") else {
            return;
        };
        // init = ftyp + moov：手工走两个顶层 box 拿到 moov 的结束位置
        let mut init_end = 0usize;
        for _ in 0..2 {
            let size =
                u32::from_be_bytes(bytes[init_end..init_end + 4].try_into().unwrap()) as usize;
            init_end += size;
        }
        let info = super::mp4::init_info(&bytes[..init_end]);
        assert_eq!(info.video_track, Some(1));
        let frags = super::mp4::fragments(&bytes[init_end..], &info);
        let syncs: Vec<_> = frags.iter().map(|f| f.video_sync).collect();
        assert!(frags.len() > 5, "{syncs:?}");
        assert!(syncs.contains(&Some(true)), "{syncs:?}");
        assert!(syncs.contains(&Some(false)), "{syncs:?}");
        assert!(syncs.iter().all(|s| s.is_some()), "{syncs:?}");
    }

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
