//! 切片工作台的录制侧底座：场次（`stream_sessions`）、分段（`segments`）与关键帧索引。
//!
//! - [`store`]：开播时插入或复用场次行（断流合并），以及分段的读写；
//! - [`recorder`]：下载器开段 / 关段时写库，维护场次时间轴；
//! - [`index`]：分段文件旁的 `<分段>.idx` 关键帧索引（不进 SQLite）；
//! - [`locate`] / [`session_keyframes`]：按场次时间找到可以落刀 / 起播的分段与字节偏移；
//! - [`recover`]：启动时收尾上次异常退出留下的 `recording` 分段与没结束的场次。

pub mod index;
pub mod live;
pub mod recorder;
pub mod store;

use crate::server::infrastructure::connection_pool::ConnectionPool;
use index::Container;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use store::{SegmentRow, SegmentState};
use tracing::{info, warn};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database: {0}")]
    Db(#[from] sqlx::Error),
    #[error("io: {0}")]
    Io(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// [`locate`] 的结果：从 `path` 先读 `[0, header_len)` 再从 `offset` 起读，
/// 得到的就是一路从关键帧开始的可播放流。
#[derive(Debug, Clone, PartialEq)]
pub struct Located {
    pub segment_id: i64,
    pub path: PathBuf,
    pub container: Container,
    pub header_len: u64,
    pub offset: u64,
    /// 这个关键帧在场次时间轴上的位置（毫秒），不晚于请求的时刻（请求落在断流空档或
    /// 分段开头之前时，是之后第一个关键帧）。
    pub keyframe_ms: i64,
    /// 分段 t = 0 处的容器原始时间戳与其单位，读取方重写时间戳时用。
    pub base_ts: Option<i64>,
    pub timescale: u32,
}

/// 场次时间轴上的一个关键帧。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionKeyframe {
    pub t_ms: i64,
    pub segment_id: i64,
    pub offset: u64,
}

fn readable(segment: &SegmentRow) -> bool {
    matches!(
        segment.state,
        SegmentState::Recording | SegmentState::Finished
    ) && Container::from_path(Path::new(&segment.path)).is_some()
}

/// 录制中、正由索引任务边写边建的分段直接读它落盘的缓存（最多落后几秒），不扫盘；
/// 其余的按需续扫。
async fn segment_index(segment: &SegmentRow) -> io::Result<index::KeyframeIndex> {
    let path = PathBuf::from(&segment.path);
    let finished = segment.state == SegmentState::Finished;
    tokio::task::spawn_blocking(move || {
        if !finished
            && index::live::is_live(&path)
            && let Some(cached) = index::load(&path)
        {
            return Ok(on_disk(cached, &path));
        }
        index::refresh(&path, finished)
    })
    .await
    .map_err(io::Error::other)?
}

/// 缓存里偏移已写出、但可能还在写入端缓冲里没到盘上的关键帧先不给出去。
fn on_disk(mut cached: index::KeyframeIndex, path: &Path) -> index::KeyframeIndex {
    let len = std::fs::metadata(path).map_or(0, |m| m.len());
    cached.keyframes.retain(|k| k.offset < len);
    cached
}

/// 场次时间 `t_ms` 处（不晚于它的最近一个关键帧）对应的分段与字节偏移。
///
/// `t_ms` 落在断流空档、早于第一个分段、或所在分段在它之前没有关键帧时，前进到之后
/// 第一个关键帧；晚于最后一个分段末尾时取最后一个关键帧；场次里没有可读的关键帧返回
/// `None`。读不到的分段（文件没了、容器不支持）跳过。
pub async fn locate(pool: &ConnectionPool, session_id: i64, t_ms: i64) -> Result<Option<Located>> {
    let segments: Vec<SegmentRow> = store::session_segments(pool, session_id)
        .await?
        .into_iter()
        .filter(readable)
        .collect();
    let first = segments
        .partition_point(|s| s.start_ms <= t_ms)
        .saturating_sub(1);
    for (i, segment) in segments.iter().enumerate().skip(first) {
        let is_last = i + 1 == segments.len();
        if !is_last && segment.end_ms.is_some_and(|end| t_ms >= end) {
            continue;
        }
        let index = match segment_index(segment).await {
            Ok(index) => index,
            Err(e) => {
                warn!(path = segment.path, error = %e, "分段读不到关键帧索引，跳过");
                continue;
            }
        };
        let relative = (t_ms - segment.start_ms).clamp(0, u32::MAX as i64) as u32;
        let keyframe = if t_ms < segment.start_ms {
            index.keyframes.first().copied()
        } else {
            index
                .at_or_before(relative)
                .or_else(|| index.at_or_after(relative))
        };
        if let Some(keyframe) = keyframe {
            return Ok(Some(Located {
                segment_id: segment.id,
                path: PathBuf::from(&segment.path),
                container: index.container,
                header_len: index.header_len,
                offset: keyframe.offset,
                keyframe_ms: segment.start_ms + keyframe.t_ms as i64,
                base_ts: index.base_ts,
                timescale: index.timescale,
            }));
        }
    }
    Ok(None)
}

/// 场次时间 `[from_ms, to_ms]` 内的全部关键帧（跨分段，按时间排序）。
pub async fn session_keyframes(
    pool: &ConnectionPool,
    session_id: i64,
    from_ms: i64,
    to_ms: i64,
) -> Result<Vec<SessionKeyframe>> {
    let mut out = Vec::new();
    for segment in store::session_segments(pool, session_id).await? {
        if !readable(&segment)
            || segment.start_ms > to_ms
            || segment.end_ms.is_some_and(|end| end < from_ms)
        {
            continue;
        }
        let index = match segment_index(&segment).await {
            Ok(index) => index,
            Err(e) => {
                warn!(path = segment.path, error = %e, "分段读不到关键帧索引，跳过");
                continue;
            }
        };
        let rel = |t: i64| (t - segment.start_ms).clamp(0, u32::MAX as i64) as u32;
        out.extend(
            index
                .range(rel(from_ms), rel(to_ms))
                .iter()
                .map(|k| SessionKeyframe {
                    t_ms: segment.start_ms + k.t_ms as i64,
                    segment_id: segment.id,
                    offset: k.offset,
                }),
        );
    }
    out.sort_by_key(|k| k.t_ms);
    Ok(out)
}

/// 启动时（开始监控之前）收尾上次异常退出留下的状态：
///
/// - `recording` 分段：文件还在就按文件长度记为 `finished`，按索引扫出的时长（扫不出时用
///   文件修改时间）回填 `end_ms`，同时核对 / 截断 / 续扫索引缓存；文件没生成或是空的就删行；
/// - 没有 `ended_at` 的场次：记为最后一个分段文件的修改时间；文件不在了按时间轴上最后一个
///   分段的结束位置算，没有分段的记为开播时间。
pub async fn recover(pool: &ConnectionPool) -> Result<()> {
    let leftovers = store::segments_in_state(pool, SegmentState::Recording).await?;
    let mut started: HashMap<i64, i64> = HashMap::new();
    for segment in &leftovers {
        let started_at = match started.get(&segment.session_id) {
            Some(v) => *v,
            None => {
                let v = store::session(pool, segment.session_id)
                    .await?
                    .and_then(|s| s.started_at)
                    .unwrap_or(0);
                started.insert(segment.session_id, v);
                v
            }
        };
        recover_segment(pool, segment, started_at).await?;
    }
    for session_id in started.keys() {
        store::clear_started_at_if_empty(pool, *session_id).await?;
    }
    // 结束时间优先取最后一个分段文件的修改时间（最后一次写盘的墙钟）：时间轴按容器时长累加，
    // 和墙钟能差出几秒到几分钟，用它算下一次接上时的断流会失真
    let unended = store::unended_sessions(pool).await?;
    let sessions = unended.len();
    for (id, last_path) in unended {
        if let Some(mtime) = last_path.as_deref().and_then(|p| modified_ms(Path::new(p))) {
            store::close_session(pool, id, mtime).await?;
        }
    }
    store::close_unended_sessions(pool).await?;
    if !leftovers.is_empty() || sessions > 0 {
        info!(
            segments = leftovers.len(),
            sessions, "切片工作台：已收尾上次异常退出留下的分段与场次"
        );
    }
    Ok(())
}

async fn recover_segment(
    pool: &ConnectionPool,
    segment: &SegmentRow,
    started_at: i64,
) -> Result<()> {
    let recorded = PathBuf::from(&segment.path);
    let final_path = PathBuf::from(
        segment
            .path
            .strip_suffix(".part")
            .unwrap_or(&segment.path)
            .to_string(),
    );
    let found = [recorded.clone(), final_path].into_iter().find_map(|p| {
        let meta = std::fs::metadata(&p).ok()?;
        (meta.len() > 0).then_some((p, meta))
    });
    let Some((path, meta)) = found else {
        index::remove(&recorded);
        store::delete_segment(pool, segment.id).await?;
        return Ok(());
    };
    if path != recorded {
        let _ = std::fs::rename(index::index_path(&recorded), index::index_path(&path));
    }
    let scan = {
        let path = path.clone();
        tokio::task::spawn_blocking(move || index::refresh(&path, true))
            .await
            .map_err(io::Error::other)?
    };
    let index_path = match &scan {
        Ok(_) => Some(index::index_path(&path).to_string_lossy().into_owned()),
        Err(e) => {
            if e.kind() != io::ErrorKind::Unsupported {
                warn!(path = %path.display(), error = %e, "收尾分段时建关键帧索引失败");
            }
            index::remove(&path);
            None
        }
    };
    let mtime_ms = modified_ms(&path);
    let duration = match &scan {
        Ok(index) if index.duration_ms > 0 => index.duration_ms as i64,
        _ => mtime_ms.map_or(0, |m| m - started_at - segment.start_ms),
    }
    .max(0);
    let next_start: Option<i64> = sqlx::query_scalar(
        "SELECT MIN(start_ms) FROM segments WHERE session_id = ? AND start_ms > ? AND id != ?",
    )
    .bind(segment.session_id)
    .bind(segment.start_ms)
    .bind(segment.id)
    .fetch_one(pool)
    .await?;
    let mut end_ms = segment.start_ms + duration;
    if let Some(next) = next_start {
        end_ms = end_ms.min(next);
    }
    store::finish_segment(
        pool,
        segment.id,
        &store::FinishedSegment {
            path: path.to_string_lossy().into_owned(),
            state: SegmentState::Finished,
            end_ms,
            bytes: Some(meta.len() as i64),
            index_path,
            danmaku_path: None,
        },
    )
    .await?;
    Ok(())
}

fn modified_ms(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
}

#[cfg(test)]
mod tests;
