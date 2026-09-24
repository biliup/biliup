//! 分段文件的关键帧索引：把「段内时间」对应到「文件字节偏移」。
//!
//! 索引不进 SQLite，缓存在分段旁边的 `<分段>.idx`。进程内写盘的下载器（stream-gears、mesio）
//! 录制时由 [`live`] 边写边建，录制热路径上只有一次非阻塞发送；外部进程下载器、旧文件、
//! 异常退出后不完整的缓存，在关段 / 启动收尾时只读地扫描分段文件的 tag 头 / TS 包头 /
//! fMP4 box 头，从缓存扫到的偏移续扫。两条路径用同一套扫描器。mesio 写在 FLV 文件头的
//! `onMetaData.keyframes` 会跳过间隔不到 1.9 s 的关键帧，所以不用它，一律逐 tag 扫。
//!
//! 段内时间 `t_ms` 以段内第一个关键帧为 0（[`KeyframeIndex::base_ts`] 记着它的容器原始时间戳），
//! 所以 stream-gears 的绝对 FLV 时间戳、B 站 `hls_fmp4` 保留的源站 `tfdt`、TS 的 PTS 都不用
//! 改写文件：读取方按 `原始时间戳 - base_ts` 换算即可。FLV 的原始时间戳是 tag 时间戳（DTS），
//! TS 是 PES 的 PTS，fMP4 是 `tfdt` 起算的解码时间。

mod flv;
mod fmp4;
pub mod live;
mod ts;

pub use flv::classify as classify_flv_tag;

use bytes::Bytes;
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::debug;

/// 索引缓存文件的扩展名，拼在分段文件名之后：`a.flv` → `a.flv.idx`。
pub const INDEX_EXTENSION: &str = "idx";

const MAGIC: &[u8; 8] = b"BLUPKIDX";
/// 缓存格式版本。读到别的版本一律当作没有缓存、重新扫描。
pub const FORMAT_VERSION: u16 = 2;
const FIXED_HEADER_SIZE: usize = 80;
const ENTRY_SIZE: usize = 12;
/// 扫描时读缓冲的大小。
const READ_BUFFER: usize = 256 * 1024;

/// 扫描器读的来源：盘上的分段文件，或 [`live`] 里录制时收到的已写字节。
trait Source: Read + Seek {
    /// 从当前位置前后跳 `n` 字节。
    fn skip(&mut self, n: i64) -> io::Result<()>;

    /// 读 `n` 个字节；来源本来就持有这段 [`Bytes`] 时切片返回，不复制。
    fn read_bytes(&mut self, n: usize) -> io::Result<Bytes> {
        let mut buf = vec![0u8; n];
        self.read_exact(&mut buf)?;
        Ok(buf.into())
    }
}

impl Source for BufReader<File> {
    fn skip(&mut self, n: i64) -> io::Result<()> {
        self.seek_relative(n)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    Flv,
    Ts,
    /// 分片 MP4（init + moof/mdat），mesio 录 B 站 `hls_fmp4` 的产物。
    Fmp4,
}

impl Container {
    /// 按扩展名判断（忽略录制中的 `.part` 后缀）。非分片 MP4 在扫描时才能认出来，会报 `Unsupported`。
    pub fn from_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?.to_ascii_lowercase();
        let name = name.strip_suffix(".part").unwrap_or(&name);
        match name.rsplit_once('.')?.1 {
            "flv" => Some(Self::Flv),
            "ts" => Some(Self::Ts),
            "mp4" | "m4s" => Some(Self::Fmp4),
            _ => None,
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::Flv => 1,
            Self::Ts => 2,
            Self::Fmp4 => 3,
        }
    }

    fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Flv),
            2 => Some(Self::Ts),
            3 => Some(Self::Fmp4),
            _ => None,
        }
    }
}

/// 一个可以落刀的位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keyframe {
    /// 段内时间（毫秒），段内第一个关键帧为 0。
    pub t_ms: u32,
    /// 从这个偏移起读，第一个单元就是该关键帧：FLV 为 tag 头，TS 为 PES 起始包，fMP4 为 `moof`。
    pub offset: u64,
}

/// 增量续扫需要记住的容器状态。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Track {
    /// TS：视频 PID；fMP4：视频轨 track_ID。0 = 还没找到。
    pub id: u32,
    /// TS：PMT PID；fMP4：视频轨 trex 的 default_sample_duration。
    pub aux: u32,
    /// fMP4：视频轨 trex 的 default_sample_flags。
    pub default_flags: u32,
    /// TS：PMT 里的视频 stream_type（0x1B H.264 / 0x24 H.265）。
    pub codec: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyframeIndex {
    pub container: Container,
    /// 分段已经写完并且已经扫到文件末尾，索引不会再变。
    pub complete: bool,
    /// 文件开头的头区长度：FLV 头 + onMetaData + 序列头 / 第一个 PES 之前的 PAT、PMT /
    /// fMP4 的 init 段（`ftyp` + `moov`）。从关键帧偏移起读前先发这一段。
    pub header_len: u64,
    header_final: bool,
    /// 容器时间戳单位（每秒多少）：FLV 1000，TS 90000，fMP4 为视频轨 `mdhd` 的 timescale。
    pub timescale: u32,
    /// 段内 t = 0（第一个关键帧）的容器原始时间戳，单位 `timescale`。
    pub base_ts: Option<i64>,
    /// 段内目前见到的最大媒体时间（毫秒，相对 `base_ts`），即已写部分的时长。
    pub duration_ms: u32,
    /// 这个偏移之前的内容都已进索引，增量扫描从这里继续。
    pub scanned_upto: u64,
    /// 写缓存时分段文件的长度。
    pub source_len: u64,
    pub track: Track,
    pub keyframes: Vec<Keyframe>,
}

impl KeyframeIndex {
    fn new(container: Container) -> Self {
        Self {
            container,
            complete: false,
            header_len: 0,
            header_final: false,
            timescale: match container {
                Container::Flv => 1000,
                Container::Ts => 90_000,
                Container::Fmp4 => 0,
            },
            base_ts: None,
            duration_ms: 0,
            scanned_upto: 0,
            source_len: 0,
            track: Track::default(),
            keyframes: Vec::new(),
        }
    }

    /// `[from_ms, to_ms]` 之间（含两端）的关键帧。
    pub fn range(&self, from_ms: u32, to_ms: u32) -> &[Keyframe] {
        let start = self.keyframes.partition_point(|k| k.t_ms < from_ms);
        let end = self.keyframes.partition_point(|k| k.t_ms <= to_ms);
        &self.keyframes[start..end.max(start)]
    }

    /// 不晚于 `t_ms` 的最后一个关键帧。
    pub fn at_or_before(&self, t_ms: u32) -> Option<Keyframe> {
        let i = self.keyframes.partition_point(|k| k.t_ms <= t_ms);
        i.checked_sub(1).map(|i| self.keyframes[i])
    }

    /// 不早于 `t_ms` 的第一个关键帧。
    pub fn at_or_after(&self, t_ms: u32) -> Option<Keyframe> {
        let i = self.keyframes.partition_point(|k| k.t_ms < t_ms);
        self.keyframes.get(i).copied()
    }

    /// 把容器原始时间戳换算成段内毫秒（早于第一个关键帧的记为 0）。
    fn relative_ms(&self, raw: i64) -> Option<u32> {
        let base = self.base_ts?;
        if self.timescale == 0 {
            return None;
        }
        let ms = (raw - base).max(0) as i128 * 1000 / self.timescale as i128;
        Some(ms.min(u32::MAX as i128) as u32)
    }

    /// 记一个关键帧；段内时间不回退（时间戳回退时贴住上一个）。
    fn push_keyframe(&mut self, raw: i64, offset: u64) {
        self.base_ts.get_or_insert(raw);
        let mut t_ms = self.relative_ms(raw).unwrap_or(0);
        if let Some(last) = self.keyframes.last() {
            if offset <= last.offset {
                return;
            }
            t_ms = t_ms.max(last.t_ms);
        }
        self.keyframes.push(Keyframe { t_ms, offset });
        self.duration_ms = self.duration_ms.max(t_ms);
    }

    fn observe_media(&mut self, raw_end: i64) {
        if let Some(ms) = self.relative_ms(raw_end) {
            self.duration_ms = self.duration_ms.max(ms);
        }
    }

    fn finalize_header(&mut self, offset: u64) {
        if !self.header_final {
            self.header_len = offset;
            self.header_final = true;
        }
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(FIXED_HEADER_SIZE + self.keyframes.len() * ENTRY_SIZE);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&(FIXED_HEADER_SIZE as u16).to_le_bytes());
        out.push(self.container.code());
        out.push(0);
        let flags = u8::from(self.complete)
            | (u8::from(self.base_ts.is_some()) << 1)
            | (u8::from(self.header_final) << 2);
        out.push(flags);
        out.push(0);
        out.extend_from_slice(&self.header_len.to_le_bytes());
        out.extend_from_slice(&self.timescale.to_le_bytes());
        out.extend_from_slice(&self.duration_ms.to_le_bytes());
        out.extend_from_slice(&self.base_ts.unwrap_or(0).to_le_bytes());
        out.extend_from_slice(&self.scanned_upto.to_le_bytes());
        out.extend_from_slice(&self.source_len.to_le_bytes());
        out.extend_from_slice(&self.track.id.to_le_bytes());
        out.extend_from_slice(&self.track.aux.to_le_bytes());
        out.extend_from_slice(&self.track.default_flags.to_le_bytes());
        out.push(self.track.codec);
        out.extend_from_slice(&[0; 7]);
        out.extend_from_slice(&(self.keyframes.len() as u32).to_le_bytes());
        debug_assert_eq!(out.len(), FIXED_HEADER_SIZE);
        for k in &self.keyframes {
            out.extend_from_slice(&k.t_ms.to_le_bytes());
            out.extend_from_slice(&k.offset.to_le_bytes());
        }
        out
    }

    fn decode(bytes: &[u8]) -> io::Result<Self> {
        let bad = |what: &str| io::Error::new(io::ErrorKind::InvalidData, what.to_string());
        if bytes.len() < FIXED_HEADER_SIZE || &bytes[..8] != MAGIC {
            return Err(bad("not a keyframe index"));
        }
        let u16_at = |i: usize| u16::from_le_bytes(bytes[i..i + 2].try_into().unwrap());
        let u32_at = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
        let u64_at = |i: usize| u64::from_le_bytes(bytes[i..i + 8].try_into().unwrap());
        if u16_at(8) != FORMAT_VERSION {
            return Err(bad("unsupported keyframe index version"));
        }
        let header_size = u16_at(10) as usize;
        if header_size < FIXED_HEADER_SIZE || header_size > bytes.len() {
            return Err(bad("bad keyframe index header size"));
        }
        let container = Container::from_code(bytes[12]).ok_or_else(|| bad("bad container"))?;
        let flags = bytes[14];
        let count = u32_at(76) as usize;
        let entries = &bytes[header_size..];
        if entries.len() != count * ENTRY_SIZE {
            return Err(bad("keyframe index entry count mismatch"));
        }
        let keyframes = entries
            .as_chunks::<ENTRY_SIZE>()
            .0
            .iter()
            .map(|e| Keyframe {
                t_ms: u32::from_le_bytes(e[..4].try_into().unwrap()),
                offset: u64::from_le_bytes(e[4..].try_into().unwrap()),
            })
            .collect();
        Ok(Self {
            container,
            complete: flags & 1 != 0,
            base_ts: (flags & 2 != 0).then(|| u64_at(32) as i64),
            header_final: flags & 4 != 0,
            header_len: u64_at(16),
            timescale: u32_at(24),
            duration_ms: u32_at(28),
            scanned_upto: u64_at(40),
            source_len: u64_at(48),
            track: Track {
                id: u32_at(56),
                aux: u32_at(60),
                default_flags: u32_at(64),
                codec: bytes[68],
            },
            keyframes,
        })
    }
}

/// 分段对应的索引缓存路径：`a.flv` → `a.flv.idx`。
pub fn index_path(segment: &Path) -> PathBuf {
    let mut name = segment.as_os_str().to_os_string();
    name.push(".");
    name.push(INDEX_EXTENSION);
    PathBuf::from(name)
}

/// 读缓存；没有、读不了或版本不对都返回 `None`。
pub fn load(segment: &Path) -> Option<KeyframeIndex> {
    let bytes = fs::read(index_path(segment)).ok()?;
    KeyframeIndex::decode(&bytes).ok()
}

fn save(segment: &Path, index: &KeyframeIndex) -> io::Result<()> {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let path = index_path(segment);
    // 同一分段可能同时被录制收尾和 DVR 请求续扫：各写各的临时文件，rename 后到者覆盖，
    // 两份都是完整的索引
    let tmp = {
        let mut name = path.as_os_str().to_os_string();
        name.push(format!(
            ".{}-{}.tmp",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        PathBuf::from(name)
    };
    let written = File::create(&tmp).and_then(|mut file| {
        file.write_all(&index.encode())?;
        file.flush()
    });
    if let Err(e) = written.and_then(|()| fs::rename(&tmp, &path)) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

/// 取分段的关键帧索引：有可用缓存就续扫，没有就从头建，扫完写回缓存。
///
/// `finished` = 分段已经写完（关段之后）。
/// 缓存与文件对不上（文件被截断、被同名覆盖）时按文件长度截断缓存或整个重建。
/// 读 `t_ms` 所在位置用 [`KeyframeIndex::at_or_before`]。
pub fn refresh(segment: &Path, finished: bool) -> io::Result<KeyframeIndex> {
    let container = indexable(segment)?;
    let file = File::open(segment)?;
    let file_len = file.metadata()?.len();
    let mut reader = BufReader::with_capacity(READ_BUFFER, file);

    let cached = load(segment)
        .filter(|c| c.container == container)
        .and_then(|c| validate_cache(c, &mut reader, file_len));
    if let Some(cached) = &cached
        && cached.complete
        && cached.source_len == file_len
    {
        return Ok(cached.clone());
    }

    let mut index = cached.unwrap_or_else(|| KeyframeIndex::new(container));
    let from = index.scanned_upto;
    scan(&mut reader, file_len, &mut index)?;
    if file_len > from {
        debug!(
            path = %segment.display(),
            from,
            to = file_len,
            "关键帧索引扫盘"
        );
    }
    index.complete = finished;
    index.source_len = file_len;
    save(segment, &index)?;
    Ok(index)
}

/// 和 [`refresh`] 一样续扫到文件末尾，但从调用方手里的索引续扫、不读写缓存文件：录制中旁路观测
/// 正在写的分段用，免得和场次记录器关段时对缓存的处理（改名、删除、标记完整）打架。
pub fn rescan(segment: &Path, previous: Option<KeyframeIndex>) -> io::Result<KeyframeIndex> {
    let container = indexable(segment)?;
    let file = File::open(segment)?;
    let file_len = file.metadata()?.len();
    let mut reader = BufReader::with_capacity(READ_BUFFER, file);
    let mut index = previous
        .filter(|c| c.container == container)
        .and_then(|c| validate_cache(c, &mut reader, file_len))
        .unwrap_or_else(|| KeyframeIndex::new(container));
    scan(&mut reader, file_len, &mut index)?;
    index.source_len = file_len;
    Ok(index)
}

fn indexable(segment: &Path) -> io::Result<Container> {
    Container::from_path(segment).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            format!("{} 不是可建索引的容器", segment.display()),
        )
    })
}

/// 从 `index.scanned_upto` 扫到 `len` 为止最后一个完整的单元。
fn scan(reader: &mut impl Source, len: u64, index: &mut KeyframeIndex) -> io::Result<()> {
    match index.container {
        Container::Flv => flv::scan(reader, len, index),
        Container::Ts => ts::scan(reader, len, index),
        Container::Fmp4 => fmp4::scan(reader, len, index),
    }
}

/// 段内 `[from_ms, to_ms]` 之间的关键帧（先按需续扫）。
pub fn keyframes(
    segment: &Path,
    from_ms: u32,
    to_ms: u32,
    finished: bool,
) -> io::Result<Vec<Keyframe>> {
    Ok(refresh(segment, finished)?.range(from_ms, to_ms).to_vec())
}

/// 删掉分段的索引缓存（分段被删时一起删）。
pub fn remove(segment: &Path) {
    let _ = fs::remove_file(index_path(segment));
}

/// 缓存与当前文件核对：文件变短就截掉越界的关键帧并从最后一个保留的关键帧续扫；
/// 抽查最后一个关键帧的偏移处不是该容器的关键帧单元（文件被同名覆盖）就整个作废。
fn validate_cache(
    mut index: KeyframeIndex,
    reader: &mut BufReader<File>,
    file_len: u64,
) -> Option<KeyframeIndex> {
    if index.scanned_upto > file_len || index.source_len > file_len {
        // 截断点之前的最后一个关键帧自己可能也不完整，一并丢掉，从再前一个重扫
        index.keyframes.retain(|k| k.offset < file_len);
        index.keyframes.pop();
        index.complete = false;
        rewind_to_last_keyframe(&mut index);
    }
    if let Some(last) = index.keyframes.last() {
        let ok = match index.container {
            Container::Flv => flv::is_keyframe_at(reader, last.offset, file_len),
            Container::Ts => ts::is_unit_start_at(reader, last.offset, file_len, index.track.id),
            Container::Fmp4 => fmp4::is_moof_at(reader, last.offset, file_len),
        };
        if !ok {
            return None;
        }
    }
    Some(index)
}

/// 把续扫起点与时长退回到最后一个关键帧处；续扫会再次遇到它，按偏移去重。
/// 一个关键帧都没有时整个重来。
fn rewind_to_last_keyframe(index: &mut KeyframeIndex) {
    match index.keyframes.last() {
        Some(last) => {
            index.scanned_upto = last.offset;
            index.duration_ms = last.t_ms;
        }
        None => {
            let container = index.container;
            *index = KeyframeIndex::new(container);
        }
    }
}

fn read_at(reader: &mut impl Source, offset: u64, buf: &mut [u8]) -> io::Result<()> {
    reader.seek(SeekFrom::Start(offset))?;
    reader.read_exact(buf)
}

#[cfg(test)]
pub(crate) mod tests;
