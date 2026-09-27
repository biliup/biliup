//! 索引缓存文件：文件头之后是只追加的记录流，落盘不经临时文件、不 rename。
//!
//! ```text
//! 文件头  "BLUPKIDX" | 版本 u16 | 容器 u8 | 保留 5 字节（共 16 字节）
//! 记录    类型 u8 | 内容长度 u8 | 内容 | 校验 u32（FNV-1a，覆盖类型、长度和内容）
//!   1 关键帧  t_ms u32 | offset u64
//!   2 元信息  header_len u64 | timescale u32 | base_ts i64 | 轨道 id u32 | aux u32 |
//!             default_flags u32 | codec u8 | 标志 u8（1 = 有 base_ts，2 = 头区已定）
//!   3 进度    duration_ms u32 | scanned_upto u64 | source_len u64 | 关键帧总数 u32 | 标志 u8（1 = 完整）
//! ```
//!
//! 一次落盘是一次 `write`：新增的关键帧、变了的元信息，最后一条进度记录。进度记录是提交点，回放时
//! 它之后的内容（崩溃时写了一半的那次落盘、文件系统在尾部补的零）一律丢弃，交给续扫补齐。
//! 关键帧按偏移去重；进度记录里的总数少于已回放的关键帧时截到这个数，多于时当作损坏停在上一个
//! 提交点——两次续扫先后写同一个文件时，后写的那份算数。文件被截断、被同名覆盖这类接不上的情形
//! 原地截断后整份重写。

use super::{Container, Keyframe, KeyframeIndex, Track};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::{Mutex, PoisonError};

const MAGIC: &[u8; 8] = b"BLUPKIDX";
/// 缓存格式版本。读到别的版本一律当作没有缓存、重新扫描。
pub(super) const FORMAT_VERSION: u16 = 3;
pub(super) const HEADER_SIZE: usize = 16;

const KEYFRAME: u8 = 1;
const META: u8 = 2;
const PROGRESS: u8 = 3;
const KEYFRAME_LEN: usize = 12;
const META_LEN: usize = 34;
const PROGRESS_LEN: usize = 25;
/// 一条记录除内容外的字节数：类型、长度、校验。
#[cfg(test)]
const RECORD_OVERHEAD: usize = 6;
#[cfg(test)]
pub(super) const PROGRESS_RECORD_SIZE: usize = PROGRESS_LEN + RECORD_OVERHEAD;
#[cfg(test)]
pub(super) const KEYFRAME_RECORD_SIZE: usize = KEYFRAME_LEN + RECORD_OVERHEAD;

/// 同一进程里写缓存文件的都串行：录制收尾和 DVR 请求可能同时续扫同一个分段。只在写的时候拿，
/// 扫描分段不拿。
static WRITES: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Meta {
    header_len: u64,
    header_final: bool,
    timescale: u32,
    base_ts: Option<i64>,
    track: Track,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Progress {
    duration_ms: u32,
    scanned_upto: u64,
    source_len: u64,
    keyframes: u32,
    complete: bool,
}

/// 文件里已经提交的内容，追加时只写与它不同的部分。
#[derive(Debug, Clone, Copy)]
struct Written {
    keyframes: usize,
    meta: Meta,
    progress: Progress,
}

impl Written {
    fn of(index: &KeyframeIndex) -> Self {
        Self {
            keyframes: index.keyframes.len(),
            meta: Meta {
                header_len: index.header_len,
                header_final: index.header_final,
                timescale: index.timescale,
                base_ts: index.base_ts,
                track: index.track,
            },
            progress: Progress {
                duration_ms: index.duration_ms,
                scanned_upto: index.scanned_upto,
                source_len: index.source_len,
                keyframes: index.keyframes.len() as u32,
                complete: index.complete,
            },
        }
    }
}

fn checksum(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5, |hash, &b| {
        (hash ^ u32::from(b)).wrapping_mul(0x0100_0193)
    })
}

fn record(out: &mut Vec<u8>, kind: u8, payload: &[u8]) {
    let start = out.len();
    out.push(kind);
    out.push(payload.len() as u8);
    out.extend_from_slice(payload);
    let check = checksum(&out[start..]);
    out.extend_from_slice(&check.to_le_bytes());
}

fn header(container: Container) -> [u8; HEADER_SIZE] {
    let mut out = [0u8; HEADER_SIZE];
    out[..8].copy_from_slice(MAGIC);
    out[8..10].copy_from_slice(&FORMAT_VERSION.to_le_bytes());
    out[10] = container.code();
    out
}

/// `index` 相对已提交内容 `written` 要追加的记录；没有变化时为空。
fn records(index: &KeyframeIndex, written: Option<&Written>) -> Vec<u8> {
    let now = Written::of(index);
    let mut out = Vec::new();
    let from = written.map_or(0, |w| w.keyframes.min(index.keyframes.len()));
    for k in &index.keyframes[from..] {
        let mut payload = [0u8; KEYFRAME_LEN];
        payload[..4].copy_from_slice(&k.t_ms.to_le_bytes());
        payload[4..].copy_from_slice(&k.offset.to_le_bytes());
        record(&mut out, KEYFRAME, &payload);
    }
    if written.is_none_or(|w| w.meta != now.meta) {
        let m = now.meta;
        let mut payload = Vec::with_capacity(META_LEN);
        payload.extend_from_slice(&m.header_len.to_le_bytes());
        payload.extend_from_slice(&m.timescale.to_le_bytes());
        payload.extend_from_slice(&m.base_ts.unwrap_or(0).to_le_bytes());
        payload.extend_from_slice(&m.track.id.to_le_bytes());
        payload.extend_from_slice(&m.track.aux.to_le_bytes());
        payload.extend_from_slice(&m.track.default_flags.to_le_bytes());
        payload.push(m.track.codec);
        payload.push(u8::from(m.base_ts.is_some()) | (u8::from(m.header_final) << 1));
        record(&mut out, META, &payload);
    }
    if !out.is_empty() || written.is_none_or(|w| w.progress != now.progress) {
        let p = now.progress;
        let mut payload = Vec::with_capacity(PROGRESS_LEN);
        payload.extend_from_slice(&p.duration_ms.to_le_bytes());
        payload.extend_from_slice(&p.scanned_upto.to_le_bytes());
        payload.extend_from_slice(&p.source_len.to_le_bytes());
        payload.extend_from_slice(&p.keyframes.to_le_bytes());
        payload.push(u8::from(p.complete));
        record(&mut out, PROGRESS, &payload);
    }
    out
}

/// 整份内容：文件头加全部记录。
pub(super) fn encode(index: &KeyframeIndex) -> Vec<u8> {
    let mut out = header(index.container).to_vec();
    out.extend(records(index, None));
    out
}

/// `pos` 处一条完整、校验通过的记录：`(类型, 内容, 下一条的位置)`。
fn next_record(bytes: &[u8], pos: usize) -> Option<(u8, &[u8], usize)> {
    let len = *bytes.get(pos + 1)? as usize;
    let end = pos + 2 + len;
    let check = bytes.get(end..end + 4)?;
    (checksum(&bytes[pos..end]) == u32::from_le_bytes(check.try_into().ok()?))
        .then(|| (bytes[pos], &bytes[pos + 2..end], end + 4))
}

/// 回放记录流，返回最后一个提交点的索引和到提交点为止的字节数。
pub(super) fn decode(bytes: &[u8]) -> io::Result<(KeyframeIndex, usize)> {
    let bad = |what: &str| io::Error::new(io::ErrorKind::InvalidData, what.to_string());
    if bytes.len() < HEADER_SIZE || &bytes[..8] != MAGIC {
        return Err(bad("not a keyframe index"));
    }
    if u16::from_le_bytes([bytes[8], bytes[9]]) != FORMAT_VERSION {
        return Err(bad("unsupported keyframe index version"));
    }
    let container = Container::from_code(bytes[10]).ok_or_else(|| bad("bad container"))?;
    let u32_at = |b: &[u8], i: usize| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
    let u64_at = |b: &[u8], i: usize| u64::from_le_bytes(b[i..i + 8].try_into().unwrap());

    let mut keyframes: Vec<Keyframe> = Vec::new();
    let mut meta: Option<Meta> = None;
    let mut committed: Option<(Meta, Progress, usize)> = None;
    let mut pos = HEADER_SIZE;
    while let Some((kind, p, next)) = next_record(bytes, pos) {
        match (kind, p.len()) {
            (KEYFRAME, KEYFRAME_LEN) => {
                let k = Keyframe {
                    t_ms: u32_at(p, 0),
                    offset: u64_at(p, 4),
                };
                if keyframes.last().is_none_or(|last| k.offset > last.offset) {
                    keyframes.push(k);
                }
            }
            (META, META_LEN) => {
                meta = Some(Meta {
                    header_len: u64_at(p, 0),
                    timescale: u32_at(p, 8),
                    base_ts: (p[33] & 1 != 0).then(|| u64_at(p, 12) as i64),
                    track: Track {
                        id: u32_at(p, 20),
                        aux: u32_at(p, 24),
                        default_flags: u32_at(p, 28),
                        codec: p[32],
                    },
                    header_final: p[33] & 2 != 0,
                });
            }
            (PROGRESS, PROGRESS_LEN) => {
                let progress = Progress {
                    duration_ms: u32_at(p, 0),
                    scanned_upto: u64_at(p, 4),
                    source_len: u64_at(p, 12),
                    keyframes: u32_at(p, 20),
                    complete: p[24] & 1 != 0,
                };
                let Some(meta) = meta else { break };
                if progress.keyframes as usize > keyframes.len() {
                    break;
                }
                keyframes.truncate(progress.keyframes as usize);
                committed = Some((meta, progress, next));
            }
            _ => break,
        }
        pos = next;
    }
    let (meta, progress, len) = committed.ok_or_else(|| bad("no committed keyframe index"))?;
    keyframes.truncate(progress.keyframes as usize);
    let mut index = KeyframeIndex::new(container);
    index.complete = progress.complete;
    index.header_len = meta.header_len;
    index.header_final = meta.header_final;
    index.timescale = meta.timescale;
    index.base_ts = meta.base_ts;
    index.duration_ms = progress.duration_ms;
    index.scanned_upto = progress.scanned_upto;
    index.source_len = progress.source_len;
    index.track = meta.track;
    index.keyframes = keyframes;
    Ok((index, len))
}

fn lock() -> std::sync::MutexGuard<'static, ()> {
    WRITES.lock().unwrap_or_else(PoisonError::into_inner)
}

/// 新建或原地截断后整份写入。
fn rewrite(path: &Path, index: &KeyframeIndex) -> io::Result<File> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.write_all(&encode(index))?;
    Ok(file)
}

/// 录制中的索引任务手上的缓存文件：开着不关，每次落盘只追加新内容。
pub(super) struct Appender {
    file: File,
    written: Written,
}

impl Appender {
    /// 新建缓存文件（已有的同名缓存清空重写），写入 `index` 的全部内容。
    pub(super) fn create(path: &Path, index: &KeyframeIndex) -> io::Result<Self> {
        let _guard = lock();
        let file = rewrite(path, index)?;
        Ok(Self {
            file,
            written: Written::of(index),
        })
    }

    /// 追加 `index` 相对上次写入的新内容，一次 `write`；没有变化时不写。
    pub(super) fn append(&mut self, index: &KeyframeIndex) -> io::Result<()> {
        let bytes = records(index, Some(&self.written));
        if bytes.is_empty() {
            return Ok(());
        }
        let _guard = lock();
        self.file.write_all(&bytes)?;
        self.written = Written::of(index);
        Ok(())
    }
}

/// 续扫之后写回缓存：文件里已提交的内容与 `index` 接得上（关键帧一个是另一个的前缀）就从提交点
/// 接着追加，先截掉提交点之后没写完的尾巴；接不上或读不出来就整份重写。
pub(super) fn store(path: &Path, index: &KeyframeIndex) -> io::Result<()> {
    let _guard = lock();
    let current = fs::read(path).ok().and_then(|bytes| {
        let (current, committed) = decode(&bytes).ok()?;
        Some((current, committed, bytes.len()))
    });
    match current {
        Some((current, committed, len))
            if current.container == index.container
                && shares_prefix(&current.keyframes, &index.keyframes) =>
        {
            let bytes = records(index, Some(&Written::of(&current)));
            if bytes.is_empty() && len == committed {
                return Ok(());
            }
            let mut file = OpenOptions::new().write(true).open(path)?;
            if len > committed {
                file.set_len(committed as u64)?;
            }
            file.seek(SeekFrom::Start(committed as u64))?;
            file.write_all(&bytes)
        }
        _ => rewrite(path, index).map(drop),
    }
}

fn shares_prefix(a: &[Keyframe], b: &[Keyframe]) -> bool {
    let n = a.len().min(b.len());
    a[..n] == b[..n]
}
