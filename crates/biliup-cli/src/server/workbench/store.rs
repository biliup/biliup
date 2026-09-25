//! `stream_sessions`（场次）的工作台列与 `segments` 的读写。
//!
//! 场次行由监控循环开播时经 [`open_session`] 插入或复用；其余由录制侧（[`super::recorder`]）
//! 和启动收尾（[`super::recover`]）写，不经过上传的 UActor。

use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::StreamerInfo;
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
    /// 时间轴 0 点的 Unix 毫秒；还没有分段时为 `None`。
    pub started_at: Option<i64>,
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

/// `stream_sessions.date`（开播检测时刻，DATETIME 文本）换算成 Unix 毫秒；解析不了为 NULL。
const DATE_MS: &str = "CAST(ROUND((julianday(date) - 2440587.5) * 86400000) AS INTEGER)";

/// [`open_session`] 的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenedSession {
    pub id: i64,
    /// 复用了同一主播上一场（断流合并）。
    pub resumed: bool,
}

/// 开播检测到时取这一场的行：同一主播最近一场已经结束、且 `now_ms - ended_at <= merge_window_ms`
/// 就复用那一行（`merge_window_ms <= 0` 表示从不合并），否则插一行新的。
///
/// 复用时不改标题和开播时间（直播历史里仍显示第一次开播），`ended_at` 由这次录制的
/// [`begin_recording`] 清空。最近一场还没结束（另一个任务在录，或上次崩溃还没收尾）时不去抢它。
pub async fn open_session(
    pool: &ConnectionPool,
    streamer_id: i64,
    info: &StreamerInfo,
    now_ms: i64,
    merge_window_ms: i64,
) -> sqlx::Result<OpenedSession> {
    let mut tx = pool.begin().await?;
    let previous: Option<(i64, Option<i64>)> = sqlx::query_as(
        "SELECT id, ended_at FROM stream_sessions WHERE streamer_id = ? ORDER BY id DESC LIMIT 1",
    )
    .bind(streamer_id)
    .fetch_optional(&mut *tx)
    .await?;
    let reusable = previous.and_then(|(id, ended_at)| {
        let ended_at = ended_at?;
        (merge_window_ms > 0 && now_ms - ended_at <= merge_window_ms).then_some(id)
    });
    let opened = match reusable {
        Some(id) => OpenedSession { id, resumed: true },
        None => {
            let id = sqlx::query_scalar(
                "INSERT INTO stream_sessions (name, url, title, date, live_cover_path, streamer_id)
                 VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
            )
            .bind(&info.name)
            .bind(&info.url)
            .bind(&info.title)
            .bind(info.date)
            .bind(&info.live_cover_path)
            .bind(streamer_id)
            .fetch_one(&mut *tx)
            .await?;
            OpenedSession { id, resumed: false }
        }
    };
    tx.commit().await?;
    Ok(opened)
}

/// [`begin_recording`] 的结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RecordingStart {
    /// 已有的时间轴 0 点；`None` 表示这一场还没有分段，第一个分段开写时再定。
    pub started_at: Option<i64>,
    /// 已有分段在时间轴上的最远位置，新分段从这之后接上。
    pub last_end_ms: i64,
    /// 断流合并接上的一场：上一次录制停下的墙钟（清空前的 `ended_at`）。
    pub resumed_after: Option<i64>,
}

/// 下载任务开始录这一场：读出已有的时间轴，并清空 `ended_at`（标记为正在录）。
pub async fn begin_recording(pool: &ConnectionPool, id: i64) -> sqlx::Result<RecordingStart> {
    let mut tx = pool.begin().await?;
    let row: Option<(Option<i64>, Option<i64>)> =
        sqlx::query_as("SELECT started_at, ended_at FROM stream_sessions WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    let Some((started_at, ended_at)) = row else {
        return Ok(RecordingStart::default());
    };
    let last_end_ms: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(COALESCE(end_ms, start_ms)), 0) FROM segments WHERE session_id = ?",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("UPDATE stream_sessions SET ended_at = NULL WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(RecordingStart {
        started_at,
        last_end_ms,
        resumed_after: started_at.and(ended_at),
    })
}

/// 定下时间轴 0 点（已经有了就不动），返回实际的 0 点。
pub async fn set_started_at(pool: &ConnectionPool, id: i64, at: i64) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "UPDATE stream_sessions SET started_at = COALESCE(started_at, ?) WHERE id = ?
         RETURNING started_at",
    )
    .bind(at)
    .bind(id)
    .fetch_optional(pool)
    .await
    .map(|v| v.unwrap_or(at))
}

pub async fn close_session(pool: &ConnectionPool, id: i64, ended_at: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE stream_sessions SET ended_at = ? WHERE id = ?")
        .bind(ended_at)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn session(pool: &ConnectionPool, id: i64) -> sqlx::Result<Option<SessionRow>> {
    sqlx::query_as::<_, (i64, Option<i64>, String, Option<i64>, Option<i64>)>(
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

/// 还没有 `ended_at` 的场次（正在录，或进程异常退出没来得及收尾），带上时间轴上最后一个
/// 分段的路径（没有分段为 `None`）。
pub async fn unended_sessions(pool: &ConnectionPool) -> sqlx::Result<Vec<(i64, Option<String>)>> {
    sqlx::query_as(
        "SELECT s.id, (SELECT g.path FROM segments g
                       WHERE g.session_id = s.id AND g.state IN ('recording', 'finished')
                       ORDER BY g.start_ms DESC, g.id DESC LIMIT 1)
         FROM stream_sessions s WHERE s.ended_at IS NULL",
    )
    .fetch_all(pool)
    .await
}

/// 给还没有 `ended_at` 的场次补上：有分段的记为时间轴上最后一个分段的结束位置，没有的记为
/// 开播时间。返回补了几行。
pub async fn close_unended_sessions(pool: &ConnectionPool) -> sqlx::Result<u64> {
    let sql = format!(
        "UPDATE stream_sessions SET ended_at = COALESCE(
             started_at + (SELECT MAX(COALESCE(end_ms, start_ms)) FROM segments
                           WHERE session_id = stream_sessions.id),
             {DATE_MS}, 0)
         WHERE ended_at IS NULL"
    );
    Ok(sqlx::query(&sql).execute(pool).await?.rows_affected())
}

pub async fn insert_segment(
    pool: &ConnectionPool,
    session_id: i64,
    path: &str,
    container: &str,
    start_ms: i64,
    gap_before_ms: i64,
) -> sqlx::Result<i64> {
    let id = sqlx::query_scalar(
        "INSERT INTO segments (session_id, path, container, state, start_ms, gap_before_ms)
         VALUES (?, ?, ?, 'recording', ?, ?) RETURNING id",
    )
    .bind(session_id)
    .bind(path)
    .bind(container)
    .bind(start_ms)
    .bind(gap_before_ms)
    .fetch_one(pool)
    .await?;
    super::retention::refresh_pin_counts(pool, session_id).await?;
    Ok(id)
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

/// 关段写入。删除点可能赶在关段事件落库之前处理了这个分段（过滤删除与关段几乎同时发生），
/// 这时它已定下的 `pending_delete` / `deleted` 不被覆盖。
pub async fn finish_segment(
    pool: &ConnectionPool,
    id: i64,
    segment: &FinishedSegment,
) -> sqlx::Result<()> {
    let session_id: Option<i64> = sqlx::query_scalar(
        "UPDATE segments SET path = ?, end_ms = ?, bytes = ?, index_path = ?,
             state = CASE WHEN state IN ('pending_delete', 'deleted') THEN state ELSE ? END,
             danmaku_path = COALESCE(?, danmaku_path)
         WHERE id = ? RETURNING session_id",
    )
    .bind(&segment.path)
    .bind(segment.end_ms)
    .bind(segment.bytes)
    .bind(&segment.index_path)
    .bind(segment.state.as_str())
    .bind(&segment.danmaku_path)
    .bind(id)
    .fetch_optional(pool)
    .await?;
    if let Some(session_id) = session_id {
        super::retention::refresh_pin_counts(pool, session_id).await?;
    }
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

/// 场次一个分段都不剩时撤掉时间轴 0 点，返回是否撤掉了。
pub async fn clear_started_at_if_empty(pool: &ConnectionPool, id: i64) -> sqlx::Result<bool> {
    let done = sqlx::query(
        "UPDATE stream_sessions SET started_at = NULL
         WHERE id = ? AND NOT EXISTS (SELECT 1 FROM segments WHERE session_id = ?)",
    )
    .bind(id)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(done.rows_affected() > 0)
}

/// 场次列表里的一行（分段按可读的 `recording` / `finished` 汇总）。
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct SessionListRow {
    pub id: i64,
    pub streamer_id: Option<i64>,
    pub streamer_name: String,
    pub title: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub retain_until: Option<i64>,
    pub segment_count: i64,
    pub bytes: i64,
    /// 可读分段在时间轴上的最远位置（正在写的分段按开段位置算）。
    pub end_ms: i64,
}

const SESSION_LIST_SELECT: &str = "SELECT s.id, s.streamer_id, s.name AS streamer_name, s.title,
        s.started_at, s.ended_at, s.retain_until,
        COUNT(g.id) AS segment_count,
        COALESCE(SUM(g.bytes), 0) AS bytes,
        COALESCE(MAX(COALESCE(g.end_ms, g.start_ms)), 0) AS end_ms
     FROM stream_sessions s
     LEFT JOIN segments g ON g.session_id = s.id AND g.state IN ('recording', 'finished')";

/// 只列有时间轴的场次：老版本留下的行、开播后还没写出分段的行都没有分段，不是切片工作台的场次。
const HAS_TIMELINE: &str = "s.started_at IS NOT NULL
     AND EXISTS (SELECT 1 FROM segments x WHERE x.session_id = s.id)";

/// 场次列表，新的在前；`streamer_id` 为 `None` 时不过滤。返回 `(这一页, 总数)`。
pub async fn list_sessions(
    pool: &ConnectionPool,
    streamer_id: Option<i64>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<(Vec<SessionListRow>, i64)> {
    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM stream_sessions s
         WHERE (?1 IS NULL OR s.streamer_id = ?1) AND {HAS_TIMELINE}"
    ))
    .bind(streamer_id)
    .fetch_one(pool)
    .await?;
    let sql = format!(
        "{SESSION_LIST_SELECT} WHERE (?1 IS NULL OR s.streamer_id = ?1) AND {HAS_TIMELINE}
         GROUP BY s.id ORDER BY s.id DESC LIMIT ?2 OFFSET ?3"
    );
    let rows = sqlx::query_as(&sql)
        .bind(streamer_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    Ok((rows, total))
}

pub async fn session_summary(
    pool: &ConnectionPool,
    id: i64,
) -> sqlx::Result<Option<SessionListRow>> {
    let sql = format!("{SESSION_LIST_SELECT} WHERE s.id = ?1 AND {HAS_TIMELINE} GROUP BY s.id");
    sqlx::query_as(&sql).bind(id).fetch_optional(pool).await
}

pub async fn segment(pool: &ConnectionPool, id: i64) -> sqlx::Result<Option<SegmentRow>> {
    let sql = format!("SELECT {SEGMENT_COLUMNS} FROM segments WHERE id = ?");
    sqlx::query(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .as_ref()
        .map(SegmentRow::from_row)
        .transpose()
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
