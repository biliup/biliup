//! 录制时边写边建关键帧索引。
//!
//! 进程内写盘的下载器（stream-gears、mesio）经 [`IndexTap`] 把每个分段写了什么、从哪个偏移开始
//! 交过来，`<分段>.idx` 每 [`SAVE_INTERVAL`] 最多落一次盘。录制中的分段查索引直接读这个缓存
//!（[`is_live`]），不扫盘。
//!
//! FLV 收到的是写盘处用扫盘同一套判定（[`super::classify_flv_tag`]）就地得出的结论，直接记进索引；
//! TS / fMP4 收到写入端持有的原样字节（引用计数，不复制），按偏移拼成一个稀疏的内存窗口，用与扫盘
//! 同一套扫描器（[`super::scan`]）续扫，扫过的字节随即释放。
//!
//! 写入端丢了事件（[`TapFile::lost`]）、偏移对不上或扫描出错时，保存已建好的部分、不再跟踪这个
//! 文件，分段关闭时由 [`super::refresh`] 从那里扫盘补齐。外部进程下载器没有旁路，同样在关段时扫。

use super::{Container, KeyframeIndex, Source, flv, save};
use biliup::downloader::index_tap::{IndexEvent, IndexTap, TapFile};
use bytes::Bytes;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, watch};
use tracing::{debug, info};

/// 写入端到索引任务的事件队列长度。FLV 每个 tag 一个事件，一路 8 Mbps 的流每秒约百来个。
const CHANNEL_CAPACITY: usize = 4096;
const SAVE_INTERVAL: Duration = Duration::from_secs(2);
/// 没有新事件时隔这么久检查一次被写入端放弃的文件。
const SWEEP_INTERVAL: Duration = Duration::from_secs(1);
/// 一次最多连续处理这么多事件再扫一轮。
const MAX_BATCH: usize = 512;
/// TS / fMP4 攒够这么多未扫字节再扫（没扫完的 PES 每轮会从头再看一遍）。
const SCAN_STEP: u64 = 64 * 1024;
/// 两个写入端写的 FLV 文件头都是 9 字节头 + PreviousTagSize0，第一个 tag 从这里开始。
const FLV_FIRST_TAG: u64 = 13;

static LIVE: LazyLock<Mutex<HashSet<PathBuf>>> = LazyLock::new(Mutex::default);
static UPDATES: LazyLock<watch::Sender<u64>> = LazyLock::new(|| watch::channel(0).0);

/// 流式索引的进展：任一分段的 `.idx` 缓存落了一次盘，或某个分段不再边写边建（之后查询改走
/// 扫盘）时变一次。录制中的读取方等一个分段出现新关键帧时订阅它，醒来再读缓存，不扫盘。
/// 所有分段共用一个，醒来的读取方自己看它关心的那个分段。
pub fn updates() -> watch::Receiver<u64> {
    UPDATES.subscribe()
}

fn updated() {
    UPDATES.send_modify(|n| *n = n.wrapping_add(1));
}

/// `path` 正由某个索引任务边写边建索引，它的 `.idx` 缓存就是最新的，不用扫盘。
pub fn is_live(path: &Path) -> bool {
    LIVE.lock().unwrap().contains(path)
}

fn register(path: &Path) {
    LIVE.lock().unwrap().insert(path.to_path_buf());
}

fn unregister(path: &Path) {
    LIVE.lock().unwrap().remove(path);
    updated();
}

/// 起一个索引任务（阻塞线程池里的一个线程），返回写入端用的句柄。
/// 所有句柄都释放后任务保存手上的索引并退出。
pub fn spawn() -> IndexTap {
    let (tap, rx) = IndexTap::channel(CHANNEL_CAPACITY, super::classify_flv_tag);
    let runtime = tokio::runtime::Handle::current();
    tokio::task::spawn_blocking(move || Indexer::default().run(&runtime, rx));
    tap
}

/// 已写字节里扫描器会读到的部分，按文件偏移排列（FLV 不存字节，只记写到哪里）。
#[derive(Default)]
struct Window {
    chunks: VecDeque<(u64, Bytes)>,
    /// 已写到的文件长度。
    end: u64,
    pos: u64,
}

impl Window {
    /// 追加从 `offset` 开始、到 `extent` 为止的一次写入，`data` 为其中要扫描的字节。
    fn push(&mut self, offset: u64, data: Bytes, extent: u64) -> bool {
        if offset != self.end || extent < offset + data.len() as u64 {
            return false;
        }
        if !data.is_empty() {
            self.chunks.push_back((offset, data));
        }
        self.end = extent;
        true
    }

    /// 丢掉 `upto` 之前的字节。
    fn trim(&mut self, upto: u64) {
        while let Some((offset, data)) = self.chunks.front() {
            if offset + data.len() as u64 > upto {
                break;
            }
            self.chunks.pop_front();
        }
    }

    fn unscanned(&self, scanned_upto: u64) -> u64 {
        self.end.saturating_sub(scanned_upto)
    }

    /// 当前位置所在的块，及当前位置在块内的下标。
    fn at_pos(&self) -> io::Result<(&Bytes, usize)> {
        let i = self
            .chunks
            .partition_point(|(offset, _)| *offset <= self.pos);
        i.checked_sub(1)
            .map(|i| &self.chunks[i])
            .and_then(|(offset, data)| {
                let at = (self.pos - offset) as usize;
                (at < data.len()).then_some((data, at))
            })
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("没有收到偏移 {} 处的字节", self.pos),
                )
            })
    }
}

impl Read for Window {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() || self.pos >= self.end {
            return Ok(0);
        }
        let (data, at) = self.at_pos()?;
        let n = buf.len().min(data.len() - at);
        buf[..n].copy_from_slice(&data[at..at + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for Window {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let pos = match to {
            SeekFrom::Start(n) => Some(n),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
            SeekFrom::End(d) => self.end.checked_add_signed(d),
        };
        self.pos = pos.ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
        Ok(self.pos)
    }
}

impl Source for Window {
    fn skip(&mut self, n: i64) -> io::Result<()> {
        self.seek(SeekFrom::Current(n)).map(|_| ())
    }

    /// 整段落在一个块里时（跨块的只有块边界处那一个单元）切片返回，不复制。
    fn read_bytes(&mut self, n: usize) -> io::Result<Bytes> {
        if self.pos + n as u64 <= self.end {
            let (data, at) = self.at_pos()?;
            if at + n <= data.len() {
                let slice = data.slice(at..at + n);
                self.pos += n as u64;
                return Ok(slice);
            }
        }
        let mut buf = vec![0u8; n];
        self.read_exact(&mut buf)?;
        Ok(buf.into())
    }
}

struct LiveFile {
    file: Arc<TapFile>,
    index: KeyframeIndex,
    window: Window,
    last_save: Option<Instant>,
    saved_upto: u64,
}

impl LiveFile {
    fn new(file: Arc<TapFile>, container: Container) -> Self {
        let mut window = Window::default();
        let mut index = KeyframeIndex::new(container);
        if container == Container::Flv {
            window.push(0, Bytes::new(), FLV_FIRST_TAG);
            index.scanned_upto = FLV_FIRST_TAG;
        }
        Self {
            file,
            index,
            window,
            last_save: None,
            saved_upto: 0,
        }
    }

    fn path(&self) -> &Path {
        self.file.path()
    }

    fn scan(&mut self) -> io::Result<()> {
        if self.index.container == Container::Flv {
            return Ok(());
        }
        let end = self.window.end;
        super::scan(&mut self.window, end, &mut self.index)?;
        self.window.trim(self.index.scanned_upto);
        Ok(())
    }

    fn save(&mut self) {
        self.index.complete = false;
        self.index.source_len = self.window.end;
        if let Err(e) = save(self.file.path(), &self.index) {
            debug!(path = %self.path().display(), error = %e, "保存流式关键帧索引失败");
        }
        self.last_save = Some(Instant::now());
        self.saved_upto = self.index.scanned_upto;
        updated();
    }

    /// 录制中定时落盘：第一个关键帧出现后立即存一次，之后每 [`SAVE_INTERVAL`] 最多一次。
    fn maybe_save(&mut self) {
        if self.index.scanned_upto == self.saved_upto {
            return;
        }
        let due = match self.last_save {
            None => !self.index.keyframes.is_empty(),
            Some(at) => at.elapsed() >= SAVE_INTERVAL,
        };
        if due {
            self.save();
        }
    }
}

#[derive(Default)]
struct Indexer {
    /// 以 [`TapFile`] 的地址为键：同一路径被重开时是另一个文件。
    files: HashMap<usize, LiveFile>,
    dirty: HashSet<usize>,
}

fn key(file: &Arc<TapFile>) -> usize {
    Arc::as_ptr(file) as usize
}

impl Indexer {
    fn run(mut self, runtime: &tokio::runtime::Handle, mut rx: mpsc::Receiver<IndexEvent>) {
        loop {
            let next = runtime.block_on(tokio::time::timeout(SWEEP_INTERVAL, rx.recv()));
            match next {
                Ok(Some(event)) => {
                    self.handle(event);
                    for _ in 0..MAX_BATCH {
                        match rx.try_recv() {
                            Ok(event) => self.handle(event),
                            Err(_) => break,
                        }
                    }
                }
                Ok(None) => break,
                Err(_) => {}
            }
            self.scan_dirty();
            self.sweep_lost();
        }
        let keys: Vec<usize> = self.files.keys().copied().collect();
        for key in keys {
            self.abandon(key, "录制任务结束");
        }
    }

    fn handle(&mut self, event: IndexEvent) {
        match event {
            IndexEvent::Opened(file) => {
                let Some(container) = Container::from_path(file.path()) else {
                    return;
                };
                register(file.path());
                self.files
                    .insert(key(&file), LiveFile::new(file, container));
            }
            IndexEvent::FlvTag {
                file,
                offset,
                timestamp,
                data_size,
                kind,
            } => {
                if let Some(live) =
                    self.push(&file, offset, Bytes::new(), flv::tag_end(offset, data_size))
                {
                    flv::record(&mut live.index, offset, timestamp, kind);
                    live.index.scanned_upto = live.window.end;
                }
            }
            IndexEvent::Bytes { file, offset, data } => {
                let extent = offset + data.len() as u64;
                self.push(&file, offset, data, extent);
            }
            IndexEvent::Closed { file, len } => self.close(key(&file), len),
            IndexEvent::Sync(reply) => {
                self.sweep_lost();
                let _ = reply.send(());
            }
        }
    }

    /// 记下一次写入；偏移不连续时放弃这个文件，返回 `None`。
    fn push(
        &mut self,
        file: &Arc<TapFile>,
        offset: u64,
        data: Bytes,
        extent: u64,
    ) -> Option<&mut LiveFile> {
        let key = key(file);
        let live = self.files.get_mut(&key)?;
        if !live.window.push(offset, data, extent) {
            let expected = live.window.end;
            debug!(path = %live.path().display(), offset, expected, "流式关键帧索引偏移不连续");
            self.abandon(key, "偏移不连续");
            return None;
        }
        self.dirty.insert(key);
        self.files.get_mut(&key)
    }

    fn scan_dirty(&mut self) {
        for key in std::mem::take(&mut self.dirty) {
            let Some(live) = self.files.get_mut(&key) else {
                continue;
            };
            if live.index.container != Container::Flv
                && live.window.unscanned(live.index.scanned_upto) < SCAN_STEP
            {
                self.dirty.insert(key);
                continue;
            }
            if let Err(e) = live.scan() {
                debug!(path = %live.path().display(), error = %e, "流式关键帧索引扫描出错");
                self.abandon(key, "扫描出错");
            }
        }
        for live in self.files.values_mut() {
            live.maybe_save();
        }
    }

    fn close(&mut self, key: usize, len: u64) {
        let Some(live) = self.files.get_mut(&key) else {
            return;
        };
        if live.window.end != len {
            debug!(path = %live.path().display(), len, seen = live.window.end, "流式关键帧索引长度不符");
            self.abandon(key, "长度不符");
            return;
        }
        let Some(mut live) = self.files.remove(&key) else {
            return;
        };
        self.dirty.remove(&key);
        match live.scan() {
            Ok(()) => {
                live.save();
                info!(
                    path = %live.path().display(),
                    keyframes = live.index.keyframes.len(),
                    bytes = len,
                    "流式关键帧索引已建好"
                );
            }
            Err(e) => debug!(path = %live.path().display(), error = %e, "流式关键帧索引扫描出错"),
        }
        unregister(live.path());
    }

    fn sweep_lost(&mut self) {
        let lost: Vec<usize> = self
            .files
            .iter()
            .filter(|(_, live)| live.file.lost())
            .map(|(key, _)| *key)
            .collect();
        for key in lost {
            self.abandon(key, "写入端丢了事件");
        }
    }

    /// 不再跟踪这个文件：扫完手上连续的部分、保存，查询与关段改走扫盘。
    fn abandon(&mut self, key: usize, reason: &str) {
        let Some(mut live) = self.files.remove(&key) else {
            return;
        };
        self.dirty.remove(&key);
        if live.scan().is_ok() && live.index.scanned_upto > live.saved_upto {
            live.save();
        }
        info!(
            path = %live.path().display(),
            reason,
            scanned_upto = live.index.scanned_upto,
            "流式关键帧索引中止，关段时扫盘补齐"
        );
        unregister(live.path());
    }
}

#[cfg(test)]
pub(crate) mod tests;
