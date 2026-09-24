//! `stream_sessions` / `session_streamerinfo` / `segments` 的读写。
//!
//! 只由录制侧（[`super::recorder`]）和启动收尾（[`super::recover`]）写，不经过上传的 UActor。

use crate::server::infrastructure::connection_pool::ConnectionPool;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;
use std::path::Path;

/// 分段状态，与迁移里 `segments.state` 的 CHECK 一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentState {
    Recording,
    Finished,
    Missing,
    Deleted,
    PendingDelete,
}

impl SegmentState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Recording => "recording",
            Self::Finished => "finished",
            Self::Missing => "missing",
            Self::Deleted => "deleted",
            Self::PendingDelete => "pending_delete",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "recording" => Self::Recording,
            "finished" => Self::Finished,
            "deleted" => Self::Deleted,
            "pending_delete" => Self::PendingDelete,
            _ => Self::Missing,
        }
    }
}

/// `segments.container` 的取值；按扩展名判断，忽略录制中的 `.part` 后缀。
/// 不在 CHECK 列表里的容器（如 yt-dlp 的 webm）不进表。
pub fn container_of(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    let name = name.strip_suffix(".part").unwrap_or(&name);
    match name.rsplit_once('.')?.1 {
        "flv" => Some("flv"),
        "ts" => Some("ts"),
        "mp4" | "m4s" => Some("mp4"),
        "mkv" => Some("mkv"),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRow {
    pub id: i64,
    pub streamer_id: Option<i64>,
    pub title: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentRow {
    pub id: i64,
    pub session_id: i64,
    pub path: String,
    pub container: String,
    pub state: SegmentState,
    pub start_ms: i64,
    pub end_ms: Option<i64>,
    pub bytes: Option<i64>,
    pub index_path: Option<String>,
    pub danmaku_path: Option<String>,
    pub gap_before_ms: i64,
}

impl SegmentRow {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            session_id: row.try_get("session_id")?,
            path: row.try_get("path")?,
            container: row.try_get("container")?,
            state: SegmentState::parse(row.try_get::<&str, _>("state")?),
            start_ms: row.try_get("start_ms")?,
            end_ms: row.try_get("end_ms")?,
            bytes: row.try_get("bytes")?,
            index_path: row.try_get("index_path")?,
            danmaku_path: row.try_get("danmaku_path")?,
            gap_before_ms: row.try_get("gap_before_ms")?,
        })
    }
}

const SEGMENT_COLUMNS: &str = "id, session_id, path, container, state, start_ms, end_ms, bytes, \
     index_path, danmaku_path, gap_before_ms";

/// [`open_session`] 的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenedSession {
    pub id: i64,
    /// 场次时间轴 0 点的 Unix 毫秒。
    pub started_at: i64,
    /// 复用了同一主播上一场（断流合并）。
    pub resumed: bool,
    /// 已有分段在时间轴上的最远位置，新分段从这之后接上。
    pub last_end_ms: i64,
    /// 复用时上一场的 `ended_at`（上一次录制停下的墙钟）。
    pub resumed_after: Option<i64>,
}

/// 取（或新建）这次录制所属的场次，并把 `streamerinfo_id` 挂上去。
///
/// 同一主播上一场已经结束且 `now_ms - ended_at <= merge_window_ms` 时复用上一场
/// （`merge_window_ms <= 0` 表示从不合并）；上一场还没结束（另一个任务仍在录，或上次
/// 崩溃还没收尾）时不去抢它，新开一场。
pub async fn open_session(
    pool: &ConnectionPool,
    streamer_id: i64,
    streamerinfo_id: Option<i64>,
    title: &str,
    now_ms: i64,
    merge_window_ms: i64,
) -> sqlx::Result<OpenedSession> {
    let mut tx = pool.begin().await?;
    let previous: Option<(i64, i64, Option<i64>)> = sqlx::query_as(
        "SELECT id, started_at, ended_at FROM stream_sessions
         WHERE streamer_id = ? ORDER BY id DESC LIMIT 1",
    )
    .bind(streamer_id)
    .fetch_optional(&mut *tx)
    .await?;

    let reusable = previous.and_then(|(id, started_at, ended_at)| {
        let ended_at = ended_at?;
        (merge_window_ms > 0 && now_ms - ended_at <= merge_window_ms)
            .then_some((id, started_at, ended_at))
    });
    let opened = match reusable {
        Some((id, started_at, ended_at)) => {
            sqlx::query("UPDATE stream_sessions SET ended_at = NULL WHERE id = ?")
                .bind(id)
                .execute(&mut *tx)
                .await?;
            let last_end_ms: i64 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(COALESCE(end_ms, start_ms)), 0) FROM segments
                 WHERE session_id = ?",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
            OpenedSession {
                id,
                started_at,
                resumed: true,
                last_end_ms,
                resumed_after: Some(ended_at),
            }
        }
        None => {
            let id: i64 = sqlx::query_scalar(
                "INSERT INTO stream_sessions (streamer_id, title, started_at, created_at)
                 VALUES (?, ?, ?, ?) RETURNING id",
            )
            .bind(streamer_id)
            .bind(title)
            .bind(now_ms)
            .bind(now_ms)
            .fetch_one(&mut *tx)
            .await?;
            OpenedSession {
                id,
                started_at: now_ms,
                resumed: false,
                last_end_ms: 0,
                resumed_after: None,
            }
        }
    };
    if let Some(streamerinfo_id) = streamerinfo_id {
        sqlx::query(
            "INSERT OR IGNORE INTO session_streamerinfo (session_id, streamerinfo_id)
             SELECT ?, id FROM streamerinfo WHERE id = ?",
        )
        .bind(opened.id)
        .bind(streamerinfo_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(opened)
}

pub async fn close_session(pool: &ConnectionPool, id: i64, ended_at: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE stream_sessions SET ended_at = ? WHERE id = ?")
        .bind(ended_at)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 一个分段都没留下的场次（下载器没写出文件）不保留；返回是否删掉了。
pub async fn delete_session_if_empty(pool: &ConnectionPool, id: i64) -> sqlx::Result<bool> {
    let done = sqlx::query(
        "DELETE FROM stream_sessions
         WHERE id = ? AND NOT EXISTS (SELECT 1 FROM segments WHERE session_id = ?)",
    )
    .bind(id)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(done.rows_affected() > 0)
}

pub async fn session(pool: &ConnectionPool, id: i64) -> sqlx::Result<Option<SessionRow>> {
    sqlx::query_as::<_, (i64, Option<i64>, String, i64, Option<i64>)>(
        "SELECT id, streamer_id, title, started_at, ended_at FROM stream_sessions WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map(|row| {
        row.map(
            |(id, streamer_id, title, started_at, ended_at)| SessionRow {
                id,
                streamer_id,
                title,
                started_at,
                ended_at,
            },
        )
    })
}

/// 还没有 `ended_at` 的场次（正在录，或进程异常退出没来得及收尾）。
pub async fn unended_sessions(pool: &ConnectionPool) -> sqlx::Result<Vec<(i64, i64)>> {
    sqlx::query_as("SELECT id, started_at FROM stream_sessions WHERE ended_at IS NULL")
        .fetch_all(pool)
        .await
}

pub async fn insert_segment(
    pool: &ConnectionPool,
    session_id: i64,
    path: &str,
    container: &str,
    start_ms: i64,
    gap_before_ms: i64,
) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "INSERT INTO segments (session_id, path, container, state, start_ms, gap_before_ms)
         VALUES (?, ?, ?, 'recording', ?, ?) RETURNING id",
    )
    .bind(session_id)
    .bind(path)
    .bind(container)
    .bind(start_ms)
    .bind(gap_before_ms)
    .fetch_one(pool)
    .await
}

/// 分段收尾时写入的字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedSegment {
    pub path: String,
    pub state: SegmentState,
    pub end_ms: i64,
    pub bytes: Option<i64>,
    pub index_path: Option<String>,
    pub danmaku_path: Option<String>,
}

pub async fn finish_segment(
    pool: &ConnectionPool,
    id: i64,
    segment: &FinishedSegment,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE segments SET path = ?, state = ?, end_ms = ?, bytes = ?, index_path = ?,
             danmaku_path = COALESCE(?, danmaku_path)
         WHERE id = ?",
    )
    .bind(&segment.path)
    .bind(segment.state.as_str())
    .bind(segment.end_ms)
    .bind(segment.bytes)
    .bind(&segment.index_path)
    .bind(&segment.danmaku_path)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_segment_state(
    pool: &ConnectionPool,
    id: i64,
    state: SegmentState,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE segments SET state = ? WHERE id = ?")
        .bind(state.as_str())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 从没写出过数据的分段（文件没生成或是空的）直接删行。
pub async fn delete_segment(pool: &ConnectionPool, id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM segments WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 场次的全部分段，按时间轴排序。
pub async fn session_segments(
    pool: &ConnectionPool,
    session_id: i64,
) -> sqlx::Result<Vec<SegmentRow>> {
    let sql = format!(
        "SELECT {SEGMENT_COLUMNS} FROM segments WHERE session_id = ? ORDER BY start_ms, id"
    );
    sqlx::query(&sql)
        .bind(session_id)
        .fetch_all(pool)
        .await?
        .iter()
        .map(SegmentRow::from_row)
        .collect()
}

pub async fn segments_in_state(
    pool: &ConnectionPool,
    state: SegmentState,
) -> sqlx::Result<Vec<SegmentRow>> {
    let sql = format!("SELECT {SEGMENT_COLUMNS} FROM segments WHERE state = ? ORDER BY id");
    sqlx::query(&sql)
        .bind(state.as_str())
        .fetch_all(pool)
        .await?
        .iter()
        .map(SegmentRow::from_row)
        .collect()
}
