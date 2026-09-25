//! 切片（`clips`）：在场次时间轴上选出的一段，导出成文件。
//!
//! - 这里是表的读写；
//! - [`plan`]：把场次时间上的入点、出点换算成「从哪些分段的哪个字节读到哪个字节」。

pub mod plan;

use crate::server::infrastructure::connection_pool::ConnectionPool;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

/// 一个切片最长多久（毫秒）。
pub const MAX_CLIP_MS: i64 = 6 * 3_600_000;
pub const MAX_TITLE_CHARS: usize = 80;
/// 一场最多这么多个切片。
pub const MAX_CLIPS_PER_SESSION: i64 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// 按关键帧切，不转码。
    Quick,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Quick => "quick",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "quick" => Some(Mode::Quick),
            _ => None,
        }
    }
}

/// 与迁移里 `clips.state` 的 CHECK 一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Draft,
    Exporting,
    Ready,
    Failed,
    Published,
    Discarded,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::Draft => "draft",
            State::Exporting => "exporting",
            State::Ready => "ready",
            State::Failed => "failed",
            State::Published => "published",
            State::Discarded => "discarded",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "exporting" => State::Exporting,
            "ready" => State::Ready,
            "failed" => State::Failed,
            "published" => State::Published,
            "discarded" => State::Discarded,
            _ => State::Draft,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Clip {
    pub id: i64,
    pub session_id: i64,
    pub marker_id: Option<i64>,
    pub in_ms: i64,
    pub out_ms: i64,
    pub cut_in_ms: Option<i64>,
    pub cut_out_ms: Option<i64>,
    pub mode: Option<Mode>,
    pub title: String,
    pub state: State,
    /// 相对服务工作目录：`clips/<场次>/<切片>.<扩展名>`。
    pub output_path: Option<String>,
    pub output_bytes: Option<i64>,
    pub duration_ms: Option<i64>,
    pub error: Option<String>,
    pub created_by: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Clip {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        Ok(Self {
            id: row.try_get("id")?,
            session_id: row.try_get("session_id")?,
            marker_id: row.try_get("marker_id")?,
            in_ms: row.try_get("in_ms")?,
            out_ms: row.try_get("out_ms")?,
            cut_in_ms: row.try_get("cut_in_ms")?,
            cut_out_ms: row.try_get("cut_out_ms")?,
            mode: row
                .try_get::<Option<&str>, _>("mode")?
                .and_then(Mode::parse),
            title: row.try_get("title")?,
            state: State::parse(row.try_get::<&str, _>("state")?),
            output_path: row.try_get("output_path")?,
            output_bytes: row.try_get("output_bytes")?,
            duration_ms: row.try_get("duration_ms")?,
            error: row.try_get("error")?,
            created_by: row.try_get("created_by")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    /// 产物的文件名（不含目录）。
    pub fn file_name(&self) -> Option<&str> {
        self.output_path
            .as_deref()
            .and_then(|p| p.rsplit(['/', '\\']).next())
    }
}

const COLUMNS: &str = "id, session_id, marker_id, in_ms, out_ms, cut_in_ms, cut_out_ms, mode, \
     title, state, output_path, output_bytes, duration_ms, error, created_by, created_at, updated_at";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewClip {
    pub marker_id: Option<i64>,
    pub in_ms: i64,
    pub out_ms: i64,
    pub title: String,
    pub created_by: Option<i64>,
    pub created_at: i64,
}

pub async fn insert(pool: &ConnectionPool, session_id: i64, clip: &NewClip) -> sqlx::Result<Clip> {
    let sql = format!(
        "INSERT INTO clips (session_id, marker_id, in_ms, out_ms, title, created_by, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING {COLUMNS}"
    );
    let row = sqlx::query(&sql)
        .bind(session_id)
        .bind(clip.marker_id)
        .bind(clip.in_ms)
        .bind(clip.out_ms)
        .bind(&clip.title)
        .bind(clip.created_by)
        .bind(clip.created_at)
        .bind(clip.created_at)
        .fetch_one(pool)
        .await?;
    Clip::from_row(&row)
}

pub async fn get(pool: &ConnectionPool, id: i64) -> sqlx::Result<Option<Clip>> {
    let sql = format!("SELECT {COLUMNS} FROM clips WHERE id = ?");
    sqlx::query(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .as_ref()
        .map(Clip::from_row)
        .transpose()
}

/// 场次的切片，按入点排序。
pub async fn list(pool: &ConnectionPool, session_id: i64) -> sqlx::Result<Vec<Clip>> {
    let sql = format!("SELECT {COLUMNS} FROM clips WHERE session_id = ? ORDER BY in_ms, id");
    sqlx::query(&sql)
        .bind(session_id)
        .fetch_all(pool)
        .await?
        .iter()
        .map(Clip::from_row)
        .collect()
}

pub async fn count(pool: &ConnectionPool, session_id: i64) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM clips WHERE session_id = ?")
        .bind(session_id)
        .fetch_one(pool)
        .await
}

/// 只改给出的字段。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClipChanges {
    pub in_ms: Option<i64>,
    pub out_ms: Option<i64>,
    pub title: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UpdateOutcome {
    Updated(Box<Clip>),
    NotFound,
    /// 正在导出时不能改范围。
    Busy,
    /// 改完之后出点不在入点之后。
    BadRange,
}

/// 改切片。范围变了的话，之前导出的结果作废（行回到 `draft`，产物由调用方删）。
pub async fn update(
    pool: &ConnectionPool,
    session_id: i64,
    id: i64,
    changes: &ClipChanges,
    now: i64,
) -> sqlx::Result<UpdateOutcome> {
    let mut tx = pool.begin().await?;
    let sql = format!("SELECT {COLUMNS} FROM clips WHERE id = ? AND session_id = ?");
    let Some(current) = sqlx::query(&sql)
        .bind(id)
        .bind(session_id)
        .fetch_optional(&mut *tx)
        .await?
        .as_ref()
        .map(Clip::from_row)
        .transpose()?
    else {
        return Ok(UpdateOutcome::NotFound);
    };
    let in_ms = changes.in_ms.unwrap_or(current.in_ms);
    let out_ms = changes.out_ms.unwrap_or(current.out_ms);
    if out_ms <= in_ms || out_ms - in_ms > MAX_CLIP_MS {
        return Ok(UpdateOutcome::BadRange);
    }
    let range_changed = (in_ms, out_ms) != (current.in_ms, current.out_ms);
    if range_changed && current.state == State::Exporting {
        return Ok(UpdateOutcome::Busy);
    }
    let reset = range_changed && current.state != State::Published;
    let sql = format!(
        "UPDATE clips SET in_ms = ?, out_ms = ?, title = COALESCE(?, title), updated_at = ?,
             state = CASE WHEN ?5 THEN 'draft' ELSE state END,
             cut_in_ms = CASE WHEN ?5 THEN NULL ELSE cut_in_ms END,
             cut_out_ms = CASE WHEN ?5 THEN NULL ELSE cut_out_ms END,
             mode = CASE WHEN ?5 THEN NULL ELSE mode END,
             output_path = CASE WHEN ?5 THEN NULL ELSE output_path END,
             output_bytes = CASE WHEN ?5 THEN NULL ELSE output_bytes END,
             duration_ms = CASE WHEN ?5 THEN NULL ELSE duration_ms END,
             error = CASE WHEN ?5 THEN NULL ELSE error END
         WHERE id = ?6 RETURNING {COLUMNS}"
    );
    let row = sqlx::query(&sql)
        .bind(in_ms)
        .bind(out_ms)
        .bind(&changes.title)
        .bind(now)
        .bind(reset)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    let clip = Clip::from_row(&row)?;
    tx.commit().await?;
    Ok(UpdateOutcome::Updated(Box::new(clip)))
}

/// 删切片，返回被删的行。
pub async fn delete(pool: &ConnectionPool, session_id: i64, id: i64) -> sqlx::Result<Option<Clip>> {
    let sql = format!("DELETE FROM clips WHERE id = ? AND session_id = ? RETURNING {COLUMNS}");
    sqlx::query(&sql)
        .bind(id)
        .bind(session_id)
        .fetch_optional(pool)
        .await?
        .as_ref()
        .map(Clip::from_row)
        .transpose()
}

#[cfg(test)]
mod tests;
