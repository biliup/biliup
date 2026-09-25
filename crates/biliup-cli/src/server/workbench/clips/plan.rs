//! 场次时间上的入点、出点 → 要读的分段字节区间。
//!
//! 落刀规则与 `/v1/sessions/{id}/keyframes`、工作台吸附一致：
//!
//! - 入点取不晚于它的最近关键帧；入点落在分段开头第一个关键帧之前或断流空档里时，从之后第一个
//!   关键帧起；
//! - 出点取不早于它的最近关键帧（不含这个关键帧）或分段末尾，二者取先到的；出点落在还在写的分段里、
//!   后面还没有关键帧时等它写出来（[`resolve`]）；
//! - 中间跨过的分段整段带上，断流缺口不补黑帧，时间轴上直接接上（产物比所选范围短掉缺口的长度）。
//!
//! 范围内有读不到的分段（已被清理、文件丢失、容器不支持）或前后分段的编码参数不同时报错，
//! 不静默跳过。

use super::super::dvr::{self, flv, ts};
use super::super::index::{Container, KeyframeIndex};
use super::super::store::{self, SegmentRow, SegmentState};
use super::super::{readable, segment_index};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::fs::File;

/// 从一个分段里读的一段：`[from, to)`，`from` 是关键帧。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    pub segment_id: i64,
    pub path: PathBuf,
    pub header_len: u64,
    pub from: u64,
    pub to: u64,
    /// 起始关键帧的场次时间。
    pub start_ms: i64,
    /// 这一段在场次时间上结束的位置（出点关键帧或分段末尾）。
    pub end_ms: i64,
    /// 媒体时长（毫秒，按关键帧索引的段内时间算）。
    pub duration_ms: i64,
    /// `[from, to)` 内各关键帧的偏移与段内时间，写 FLV 的关键帧表用。
    pub keyframes: Vec<(u64, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub container: Container,
    pub pieces: Vec<Piece>,
    pub cut_in_ms: i64,
    pub cut_out_ms: i64,
}

impl Plan {
    /// 产物时长：各段时长之和（断流缺口不算）。
    pub fn duration_ms(&self) -> i64 {
        self.pieces.iter().map(|p| p.duration_ms).sum()
    }

    pub fn bytes(&self) -> u64 {
        self.pieces.iter().map(|p| p.to - p.from).sum()
    }

    /// 场次时间 `t` 在产物时间轴上的位置（毫秒）：落在缺口里的算到下一段开头，早于第一段的算 0。
    pub fn output_ms(&self, t: i64) -> i64 {
        let mut offset = 0;
        for piece in &self.pieces {
            if t < piece.start_ms {
                return offset;
            }
            if t <= piece.end_ms {
                return offset + (t - piece.start_ms).min(piece.duration_ms);
            }
            offset += piece.duration_ms;
        }
        offset
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("{0}")]
    Unavailable(String),
    #[error("database: {0}")]
    Db(#[from] sqlx::Error),
    #[error("io: {0}")]
    Io(#[from] io::Error),
}

impl From<super::super::Error> for PlanError {
    fn from(e: super::super::Error) -> Self {
        match e {
            super::super::Error::Db(e) => Self::Db(e),
            super::super::Error::Io(e) => Self::Io(e),
        }
    }
}

/// 场次时间 `ms` 写成 `H:MM:SS`，报错时指位置用。
pub fn clock(ms: i64) -> String {
    let s = ms.max(0) / 1000;
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}

#[derive(Debug)]
pub enum Attempt {
    Ready(Plan),
    /// 出点落在还在写的分段里，后面还没有关键帧。
    Wait,
}

fn overlaps(segment: &SegmentRow, in_ms: i64, out_ms: i64) -> bool {
    segment.start_ms <= out_ms && segment.end_ms.is_none_or(|end| end > in_ms)
}

fn unavailable(segment: &SegmentRow) -> PlanError {
    let why = match segment.state {
        SegmentState::Deleted => "已经被清理",
        SegmentState::Missing => "文件丢失",
        _ => "的格式不支持切片",
    };
    PlanError::Unavailable(format!(
        "所选范围里 {} 起的那段录像{why}，剪不了；把范围挪开这一段后重试",
        clock(segment.start_ms)
    ))
}

async fn fingerprint(
    container: Container,
    path: &PathBuf,
    header_len: u64,
    keyframe: u64,
) -> io::Result<Vec<u8>> {
    let mut file = File::open(path).await?;
    let region = dvr::read_at(&mut file, 0, header_len as usize).await?;
    Ok(match container {
        Container::Flv => flv::Header::parse(&region)?.fingerprint(),
        Container::Ts => {
            let program = ts::Program::parse(&region);
            let sniff = dvr::read_at(&mut file, keyframe, dvr::TS_PARAM_SNIFF).await?;
            dvr::ts_fingerprint(&program, &sniff)
        }
        // init 段（ftyp + moov）不变才能共用一份写在文件开头
        Container::Fmp4 => region,
    })
}

async fn file_len(path: &PathBuf) -> io::Result<u64> {
    Ok(tokio::fs::metadata(path).await?.len())
}

/// 按分段表与关键帧索引的现状算一次。
pub async fn compute(
    pool: &ConnectionPool,
    session_id: i64,
    in_ms: i64,
    out_ms: i64,
) -> Result<Attempt, PlanError> {
    let segments: Vec<SegmentRow> = store::session_segments(pool, session_id)
        .await?
        .into_iter()
        .filter(|s| overlaps(s, in_ms, out_ms))
        // 空分段（开了就断）不占时间轴
        .filter(|s| s.end_ms != Some(s.start_ms) || readable(s))
        .collect();
    if let Some(bad) = segments.iter().find(|s| !readable(s)) {
        return Err(unavailable(bad));
    }
    let mut pieces: Vec<Piece> = Vec::new();
    let mut container: Option<Container> = None;
    let mut first_fingerprint: Option<Vec<u8>> = None;
    let mut cut_out_ms = None;
    for segment in &segments {
        let index: KeyframeIndex = match segment_index(segment).await {
            Ok(index) => index,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Err(PlanError::Unavailable(format!(
                    "所选范围里 {} 起的那段录像文件不见了，剪不了；把范围挪开这一段后重试",
                    clock(segment.start_ms)
                )));
            }
            Err(e) => return Err(e.into()),
        };
        let path = PathBuf::from(&segment.path);
        let rel = |t: i64| (t - segment.start_ms).clamp(0, u32::MAX as i64) as u32;
        let start = if pieces.is_empty() && in_ms >= segment.start_ms {
            index
                .at_or_before(rel(in_ms))
                .or_else(|| index.keyframes.first().copied())
        } else {
            index.keyframes.first().copied()
        };
        let Some(start) = start else {
            if segment.state == SegmentState::Recording {
                return Ok(Attempt::Wait);
            }
            continue;
        };
        match container {
            None => container = Some(index.container),
            Some(c) if c != index.container => {
                return Err(PlanError::Unavailable(format!(
                    "{} 前后两段录像的格式不同，不能接成一个文件；请以这里为界分成两个切片",
                    clock(segment.start_ms)
                )));
            }
            Some(_) => {}
        }
        let print = fingerprint(index.container, &path, index.header_len, start.offset).await?;
        match &first_fingerprint {
            None => first_fingerprint = Some(print),
            Some(first) if *first != print => {
                return Err(PlanError::Unavailable(format!(
                    "{} 前后两段录像的编码参数不同（换了分辨率或编码），不能接成一个文件；\
                     请以这里为界分成两个切片",
                    clock(segment.start_ms)
                )));
            }
            Some(_) => {}
        }
        let end = index.at_or_after(rel(out_ms).max(start.t_ms + 1));
        let (to, end_t, end_ms, done) = match end {
            Some(end) => (
                end.offset,
                end.t_ms,
                segment.start_ms + end.t_ms as i64,
                true,
            ),
            None => match segment.end_ms {
                Some(end_ms) if segment.state != SegmentState::Recording => (
                    file_len(&path).await?,
                    index.duration_ms.max(start.t_ms),
                    end_ms,
                    end_ms >= out_ms,
                ),
                _ => return Ok(Attempt::Wait),
            },
        };
        pieces.push(Piece {
            segment_id: segment.id,
            path,
            header_len: index.header_len,
            from: start.offset,
            to,
            start_ms: segment.start_ms + start.t_ms as i64,
            end_ms,
            duration_ms: (end_t - start.t_ms) as i64,
            keyframes: index
                .keyframes
                .iter()
                .filter(|k| k.offset >= start.offset && k.offset < to)
                .map(|k| (k.offset, k.t_ms))
                .collect(),
        });
        if done {
            cut_out_ms = Some(end_ms);
            break;
        }
    }
    let (Some(container), Some(first)) = (container, pieces.first()) else {
        return Err(PlanError::Unavailable(
            "所选范围里没有录像画面（可能整段落在断流空档里），换个范围再剪".into(),
        ));
    };
    let cut_in_ms = first.start_ms;
    let cut_out_ms = cut_out_ms.unwrap_or_else(|| pieces.last().map_or(out_ms, |p| p.end_ms));
    Ok(Attempt::Ready(Plan {
        container,
        pieces,
        cut_in_ms,
        cut_out_ms,
    }))
}

/// 算出要读的区间；出点之后的关键帧还没写到盘上时等（索引更新或每秒重试），最多等 `timeout`。
pub async fn resolve(
    pool: &ConnectionPool,
    session_id: i64,
    in_ms: i64,
    out_ms: i64,
    timeout: Duration,
    mut on_wait: impl FnMut(),
) -> Result<Plan, PlanError> {
    let deadline = Instant::now() + timeout;
    let mut updates = super::super::index::live::updates();
    loop {
        match compute(pool, session_id, in_ms, out_ms).await? {
            Attempt::Ready(plan) => return Ok(plan),
            Attempt::Wait => {
                if Instant::now() >= deadline {
                    return Err(PlanError::Unavailable(format!(
                        "等了 {} 秒，出点之后还没有写出关键帧（录制可能卡住了），\
                         把出点往前挪或稍后重试",
                        timeout.as_secs()
                    )));
                }
                on_wait();
                let _ = tokio::time::timeout(Duration::from_secs(1), updates.changed()).await;
            }
        }
    }
}
