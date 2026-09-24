//! DVR 回看：从场次时间轴上任一点起，把已经录下的分段（含正在写的那一段）接成一条可播放的流。
//!
//! 1. [`super::locate`] 找到不晚于 `from` 的最近关键帧所在的分段与字节偏移；
//! 2. 先发头区里决定解码器配置的部分（FLV 文件头 + 序列头 / TS 的 PAT、PMT），再从关键帧偏移
//!    起按 tag / 包转发，改写时间戳：输出时间戳 = 场次时间 + [`TIMESTAMP_ORIGIN_MS`]；
//! 3. 读到分段末尾接下一段：解码器配置（FLV 序列头 / TS 的 PMT 与参数集）不变、中间没有断流
//!    缺口就接上；接不上就结束响应，播放器从下一段的起点重开；
//! 4. 读到还在写的分段就等通知（写盘计数器增长、分段表变更，见 [`super::live`]），不轮询；
//! 5. 回看已经写完的内容时不一口气全推给浏览器：最多领先实际时间 [`READ_AHEAD_MS`]，之后按 1 倍速发。
//!
//! 只接受场次 id 和场次时间，文件路径全部来自 `segments` 表。

mod flv;
mod ts;

use super::index::Container;
use super::live::{self, LiveWatch};
use super::store::{self, SegmentRow, SegmentState};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use bytes::{Bytes, BytesMut};
use std::io::{self, SeekFrom};
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tracing::{debug, info, warn};

/// 输出时间戳相对场次时间的偏移：起播关键帧之后的音频、B 帧的 DTS 可能略早于关键帧，
/// 留出余量免得场次开头出现负时间戳。
pub const TIMESTAMP_ORIGIN_MS: i64 = 1000;
/// 回看已写完的内容时，发出去的媒体时长最多比响应开始以来的实际时间多这么多。
pub const READ_AHEAD_MS: i64 = 15_000;
/// 一个响应块的大小上限。
pub const RESPONSE_CHUNK: usize = 64 * 1024;
const READ_CHUNK: usize = 256 * 1024;
/// 相邻两个单元的时间差超过这么多时按这么多记（源时间戳跳变不能让发送节奏停下来）。
const MAX_CLOCK_STEP_MS: i64 = 1000;
const DEFAULT_FRAME_MS: i64 = 33;
/// 等下一段出现第一个关键帧时，写盘计数器每增长这么多才重新看一次索引。
const NEXT_SEGMENT_PROBE_BYTES: u64 = 256 * 1024;
const TS_PARAM_SNIFF: usize = 256 * 1024;
/// 写盘计数器在数据进入写盘缓冲之前就累加（mesio 在修复管线之前逐 tag 计数，落盘却是成块的）。
/// 被唤醒却没读到新内容时，下一次等计数器多涨这么多再看；连续落空就翻倍到上限，读到数据后复位。
const EMPTY_WAKE_STEP_MIN: u64 = 16 * 1024;
const EMPTY_WAKE_STEP_MAX: u64 = 256 * 1024;

fn empty_wake_step(streak: u32) -> u64 {
    match streak {
        0 => 0,
        n => (EMPTY_WAKE_STEP_MIN << (n - 1).min(8)).min(EMPTY_WAKE_STEP_MAX),
    }
}

/// 容器原始时间戳 → 输出时间戳。
#[derive(Debug, Clone, Copy)]
pub(crate) struct TimeMap {
    /// 分段 t = 0（第一个关键帧）处的原始时间戳。
    base: i64,
    /// 每秒多少个单位。
    timescale: i64,
    /// `base` 对应的输出时刻（毫秒）。
    origin_ms: i64,
}

impl TimeMap {
    fn out_units(&self, delta_from_base: i64) -> i64 {
        delta_from_base + self.origin_ms * self.timescale / 1000
    }

    fn ms_of(&self, units: i64) -> i64 {
        units * 1000 / self.timescale
    }
}

/// 已发出的媒体时长（毫秒），给发送节奏用。
#[derive(Debug, Default)]
pub(crate) struct Clock {
    max_ms: Option<i64>,
    sent_ms: i64,
}

impl Clock {
    fn observe(&mut self, ms: i64) {
        match self.max_ms {
            None => self.max_ms = Some(ms),
            Some(max) if ms > max => {
                self.sent_ms += (ms - max).min(MAX_CLOCK_STEP_MS);
                self.max_ms = Some(ms);
            }
            Some(_) => {}
        }
    }
}

/// 视频轨最后的输出时间与帧间隔，接段时保证时间戳严格递增。
#[derive(Debug, Default)]
pub(crate) struct Tracker {
    last: Option<i64>,
    frame_ms: Option<i64>,
}

impl Tracker {
    fn observe(&mut self, ms: i64) {
        if let Some(last) = self.last {
            let step = ms - last;
            if (1..=100).contains(&step) {
                self.frame_ms = Some(step);
            }
            self.last = Some(last.max(ms));
        } else {
            self.last = Some(ms);
        }
    }
}

enum Remux {
    Flv,
    Ts(ts::Ts),
}

impl Remux {
    fn process(
        &mut self,
        input: &[u8],
        out: &mut BytesMut,
        map: &TimeMap,
        clock: &mut Clock,
        video: &mut Tracker,
    ) -> io::Result<usize> {
        match self {
            Remux::Flv => flv::process(input, out, RESPONSE_CHUNK, map, clock, video),
            Remux::Ts(ts) => ts.process(input, out, RESPONSE_CHUNK, map, clock, video),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    #[error("场次不存在")]
    NotFound,
    #[error("这一场还没有可以回看的画面")]
    NoMedia,
    #[error("{0}")]
    Unsupported(String),
    #[error("database: {0}")]
    Db(#[from] sqlx::Error),
    #[error("io: {0}")]
    Io(#[from] io::Error),
}

impl From<super::Error> for OpenError {
    fn from(e: super::Error) -> Self {
        match e {
            super::Error::Db(e) => Self::Db(e),
            super::Error::Io(e) => Self::Io(e),
        }
    }
}

struct Cursor {
    row: SegmentRow,
    file: File,
    map: TimeMap,
    fingerprint: Vec<u8>,
    /// 分段表里的状态可能已经变了（收到变更通知之后），到文件末尾时要重新查。
    row_stale: bool,
    /// 看到 `finished` 之后又读过一次到末尾：关段前最后一次刷盘的内容已经读完。
    drained: bool,
}

#[derive(Debug, Default)]
struct Stats {
    bytes: u64,
    segments: u32,
    byte_wakes: u64,
    empty_wakes: u64,
    change_wakes: u64,
    paced: u64,
}

/// 一条 DVR 回看流。用 [`Dvr::into_stream`] 变成响应体；drop 即断开，不留后台任务。
pub struct Dvr {
    pub container: Container,
    /// 起播关键帧的场次时间（毫秒）。
    pub start_ms: i64,
    pub segment_id: i64,
    pool: ConnectionPool,
    session_id: i64,
    header: Option<Bytes>,
    remux: Remux,
    cursor: Cursor,
    pending: BytesMut,
    live: Option<LiveWatch>,
    /// 上次读文件前写盘计数器的值；读到末尾后等它变化。
    seen: u64,
    clock: Clock,
    video: Tracker,
    shift_ms: i64,
    started: Instant,
    /// 被唤醒之后还没读到新数据。
    woke: bool,
    /// 连续几次被唤醒都没读到新数据。
    empty_streak: u32,
    stats: Stats,
    end_reason: &'static str,
}

impl Dvr {
    pub fn content_type(&self) -> &'static str {
        match self.container {
            Container::Flv => "video/x-flv",
            Container::Ts => "video/mp2t",
            Container::Fmp4 => "video/mp4",
        }
    }
}

async fn read_at(file: &mut File, offset: u64, len: usize) -> io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(offset)).await?;
    let mut buf = Vec::with_capacity(len);
    (&mut *file).take(len as u64).read_to_end(&mut buf).await?;
    Ok(buf)
}

fn ts_fingerprint(program: &ts::Program, keyframe: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    for (pid, stream_type) in &program.streams {
        out.extend_from_slice(&pid.to_be_bytes());
        out.push(*stream_type);
    }
    out.extend(ts::parameter_sets(keyframe, program));
    out
}

fn timescale_of(container: Container, index_timescale: u32) -> i64 {
    match container {
        Container::Flv => 1000,
        Container::Ts => 90_000,
        Container::Fmp4 => index_timescale.max(1) as i64,
    }
}

/// 打开场次 `session_id` 从场次时间 `from_ms` 起的回看流。
pub async fn open(pool: &ConnectionPool, session_id: i64, from_ms: i64) -> Result<Dvr, OpenError> {
    if store::session(pool, session_id).await?.is_none() {
        return Err(OpenError::NotFound);
    }
    let located = super::locate(pool, session_id, from_ms.max(0))
        .await?
        .ok_or(OpenError::NoMedia)?;
    if located.container == Container::Fmp4 {
        return Err(OpenError::Unsupported(
            "分片 MP4（B 站 hls_fmp4）录像暂不支持回看，只能等录完后下载或切片".into(),
        ));
    }
    let row = store::segment(pool, located.segment_id)
        .await?
        .ok_or(OpenError::NoMedia)?;
    let mut file = File::open(&located.path).await?;
    let region = read_at(&mut file, 0, located.header_len as usize).await?;
    let map = TimeMap {
        base: located.base_ts.unwrap_or(0),
        timescale: timescale_of(located.container, located.timescale),
        origin_ms: row.start_ms + TIMESTAMP_ORIGIN_MS,
    };
    let (remux, header, fingerprint) = match located.container {
        Container::Flv => {
            let parsed = flv::Header::parse(&region)?;
            let start_raw = map.base + (located.keyframe_ms - row.start_ms);
            let header = parsed.stream_header(flv::out_ts(&map, start_raw as u32));
            (Remux::Flv, header, parsed.fingerprint())
        }
        _ => {
            let program = ts::Program::parse(&region);
            if program.video().is_none() {
                return Err(OpenError::Unsupported(
                    "TS 分段里没有找到 H.264 / H.265 视频流".into(),
                ));
            }
            let keyframe = read_at(&mut file, located.offset, TS_PARAM_SNIFF).await?;
            let fingerprint = ts_fingerprint(&program, &keyframe);
            let mut remux = ts::Ts::new(program);
            let header = remux.stream_header(&region, &map);
            (Remux::Ts(remux), header, fingerprint)
        }
    };
    file.seek(SeekFrom::Start(located.offset)).await?;
    let live = live::watch(session_id);
    Ok(Dvr {
        container: located.container,
        start_ms: located.keyframe_ms,
        segment_id: located.segment_id,
        pool: pool.clone(),
        session_id,
        header: Some(header.freeze()),
        remux,
        cursor: Cursor {
            row,
            file,
            map,
            fingerprint,
            row_stale: true,
            drained: false,
        },
        pending: BytesMut::with_capacity(READ_CHUNK),
        live,
        seen: 0,
        clock: Clock::default(),
        video: Tracker::default(),
        shift_ms: 0,
        started: Instant::now(),
        woke: false,
        empty_streak: 0,
        stats: Stats {
            segments: 1,
            ..Default::default()
        },
        end_reason: "客户端断开",
    })
}

enum Step {
    Continue,
    End(&'static str),
}

fn readable(segment: &SegmentRow) -> bool {
    matches!(
        segment.state,
        SegmentState::Recording | SegmentState::Finished
    ) && Container::from_path(Path::new(&segment.path)).is_some()
}

impl Dvr {
    pub fn into_stream(self) -> impl futures::Stream<Item = io::Result<Bytes>> + Send + 'static {
        futures::stream::unfold(self, |mut dvr| async move {
            match dvr.next_chunk().await {
                Ok(Some(chunk)) => {
                    dvr.stats.bytes += chunk.len() as u64;
                    Some((Ok(chunk), dvr))
                }
                Ok(None) => None,
                Err(e) => {
                    warn!(session = dvr.session_id, error = %e, "DVR 回看读取出错，结束响应");
                    dvr.ended("读取出错");
                    None
                }
            }
        })
    }

    async fn next_chunk(&mut self) -> io::Result<Option<Bytes>> {
        if let Some(header) = self.header.take() {
            return Ok(Some(header));
        }
        loop {
            let mut out = BytesMut::with_capacity(RESPONSE_CHUNK);
            let consumed = self.remux.process(
                &self.pending,
                &mut out,
                &self.cursor.map,
                &mut self.clock,
                &mut self.video,
            )?;
            let _ = self.pending.split_to(consumed);
            if !out.is_empty() {
                self.pace().await;
                return Ok(Some(out.freeze()));
            }
            if let Some(bytes) = self.live.as_ref().and_then(|l| l.bytes.as_ref()) {
                self.seen = bytes.total();
            }
            self.pending.reserve(READ_CHUNK);
            let n = (&mut self.cursor.file)
                .take(READ_CHUNK as u64)
                .read_buf(&mut self.pending)
                .await?;
            if n > 0 {
                self.woke = false;
                self.empty_streak = 0;
                continue;
            }
            match self.at_end_of_file().await? {
                Step::Continue => {}
                Step::End(reason) => {
                    self.ended(reason);
                    return Ok(None);
                }
            }
        }
    }

    fn ended(&mut self, reason: &'static str) {
        self.end_reason = reason;
    }

    async fn pace(&mut self) {
        let elapsed = self.started.elapsed().as_millis() as i64;
        let ahead = self.clock.sent_ms - elapsed - READ_AHEAD_MS;
        if ahead > 0 {
            self.stats.paced += 1;
            tokio::time::sleep(Duration::from_millis(ahead as u64)).await;
        }
    }

    /// 等写盘计数器增长或分段表变更。返回 `false`：这一路不能跟随（录制已停 / 没有写盘计数）。
    async fn wait_for_writes(&mut self) -> bool {
        if self.live.is_none() {
            self.live = live::watch(self.session_id);
        }
        let Some(LiveWatch { changes, bytes }) = self.live.as_mut() else {
            return false;
        };
        let Some(bytes) = bytes.as_ref() else {
            return false;
        };
        self.woke = true;
        let threshold = self.seen + empty_wake_step(self.empty_streak);
        tokio::select! {
            total = bytes.grown_since(threshold) => {
                self.seen = total;
                self.stats.byte_wakes += 1;
            }
            changed = changes.changed() => {
                self.stats.change_wakes += 1;
                self.cursor.row_stale = true;
                if changed.is_err() {
                    self.live = None;
                }
            }
        }
        true
    }

    async fn at_end_of_file(&mut self) -> io::Result<Step> {
        if self.cursor.row_stale {
            self.cursor.row_stale = false;
            match store::segment(&self.pool, self.cursor.row.id)
                .await
                .map_err(io::Error::other)?
            {
                Some(row) => {
                    if row.state != self.cursor.row.state {
                        self.cursor.drained = false;
                    }
                    self.cursor.row = row;
                }
                None => return Ok(Step::End("分段已被删除")),
            }
        }
        match self.cursor.row.state {
            SegmentState::Recording => {
                if self.woke {
                    // 计数器在数据进入写盘缓冲前就加了，新数据还没落盘
                    self.stats.empty_wakes += 1;
                    self.empty_streak += 1;
                }
                if self.wait_for_writes().await {
                    Ok(Step::Continue)
                } else {
                    // 登记没了但分段表还没更新：录制任务正在收尾，查一次最新状态
                    if self.live.is_none() && !self.cursor.drained {
                        self.cursor.drained = true;
                        self.cursor.row_stale = true;
                        return Ok(Step::Continue);
                    }
                    Ok(Step::End("这一路不能跟随正在写的分段"))
                }
            }
            SegmentState::Finished if !self.cursor.drained => {
                self.cursor.drained = true;
                Ok(Step::Continue)
            }
            SegmentState::Finished => self.next_segment().await,
            _ => Ok(Step::End("分段已不可读")),
        }
    }

    async fn next_segment(&mut self) -> io::Result<Step> {
        let segments = store::session_segments(&self.pool, self.session_id)
            .await
            .map_err(io::Error::other)?;
        let after = segments
            .iter()
            .position(|s| s.id == self.cursor.row.id)
            .map_or(segments.len(), |i| i + 1);
        let Some(next) = segments[after..].iter().find(|s| readable(s)).cloned() else {
            // 场次还在录：等新分段开出来
            if self.live.is_none() {
                self.live = live::watch(self.session_id);
            }
            let Some(live) = self.live.as_mut() else {
                return Ok(Step::End("没在录了，已回放到最新"));
            };
            self.stats.change_wakes += 1;
            if live.changes.changed().await.is_err() {
                self.live = None;
            }
            return Ok(Step::Continue);
        };
        if next.gap_before_ms > 0 {
            return Ok(Step::End("遇到断流缺口"));
        }
        if Container::from_path(Path::new(&next.path)) != Some(self.container) {
            return Ok(Step::End("下一段容器不同"));
        }
        let index = super::segment_index(&next).await?;
        let (Some(base), Some(first)) = (index.base_ts, index.keyframes.first().copied()) else {
            // 下一段刚开，还没写到第一个关键帧
            if next.state != SegmentState::Recording {
                return Ok(Step::End("下一段没有关键帧"));
            }
            let start = self.seen;
            loop {
                if !self.wait_for_writes().await {
                    return Ok(Step::End("这一路不能跟随正在写的分段"));
                }
                if self.cursor.row_stale
                    || self.seen.saturating_sub(start) >= NEXT_SEGMENT_PROBE_BYTES
                {
                    self.cursor.row_stale = false;
                    return Ok(Step::Continue);
                }
            }
        };
        let mut file = File::open(&next.path).await?;
        let region = read_at(&mut file, 0, index.header_len as usize).await?;
        let fingerprint = match &self.remux {
            Remux::Flv => flv::Header::parse(&region)?.fingerprint(),
            Remux::Ts(_) => {
                let program = ts::Program::parse(&region);
                let keyframe = read_at(&mut file, first.offset, TS_PARAM_SNIFF).await?;
                ts_fingerprint(&program, &keyframe)
            }
        };
        if fingerprint != self.cursor.fingerprint {
            return Ok(Step::End("编码参数变化"));
        }
        let origin = next.start_ms + TIMESTAMP_ORIGIN_MS;
        if let Some(last) = self.video.last {
            let frame = self.video.frame_ms.unwrap_or(DEFAULT_FRAME_MS);
            let first_out = origin + self.shift_ms;
            if last + frame > first_out {
                self.shift_ms += last + frame - first_out;
            }
        }
        file.seek(SeekFrom::Start(index.header_len)).await?;
        if let Remux::Ts(ts) = &mut self.remux {
            ts.start_segment();
        }
        debug!(
            session = self.session_id,
            from = self.cursor.row.id,
            to = next.id,
            shift_ms = self.shift_ms,
            "DVR 回看接上下一段"
        );
        self.pending.clear();
        self.cursor = Cursor {
            map: TimeMap {
                base,
                timescale: timescale_of(self.container, index.timescale),
                origin_ms: origin + self.shift_ms,
            },
            row: next,
            file,
            fingerprint,
            row_stale: false,
            drained: false,
        };
        self.stats.segments += 1;
        Ok(Step::Continue)
    }
}

impl Drop for Dvr {
    fn drop(&mut self) {
        info!(
            session = self.session_id,
            start_ms = self.start_ms,
            reason = self.end_reason,
            bytes = self.stats.bytes,
            segments = self.stats.segments,
            byte_wakes = self.stats.byte_wakes,
            empty_wakes = self.stats.empty_wakes,
            change_wakes = self.stats.change_wakes,
            paced = self.stats.paced,
            secs = self.started.elapsed().as_secs(),
            "DVR 回看连接结束"
        );
    }
}

#[cfg(test)]
mod tests;
