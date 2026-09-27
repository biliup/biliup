//! `clip_suggestions` 的读写：自动切片给出的候选。
//!
//! - 任务跑完用 [`replace_pending`] 换掉这一场上一轮没处理的候选（已接受、已丢弃的留着，
//!   与它们重复的新候选不再给出）；
//! - `pending` 的候选以 `suggestion:<id>` 引用自己的区间，防止素材在用户看到之前被删；
//!   接受（改由切片自己引用）、丢弃、72 小时没处理（[`expire`]，由保留清理任务每分钟调用）时撤销；
//! - 接受时在同一个事务里建切片草稿（[`clips::insert_in`]），候选本身不是切片，进不了集中发布。

use super::candidates::{Candidate, Evidence, overlaps_too_much};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::clips::{self, Clip, MAX_CLIPS_PER_SESSION, NewClip};
use crate::server::workbench::retention;
use serde::Serialize;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;

/// 没处理的候选多久后过期、撤销引用。
pub const EXPIRE_AFTER_MS: i64 = 72 * 3_600_000;

/// 与迁移里 `clip_suggestions.state` 的 CHECK 一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SuggestionState {
    Pending,
    Accepted,
    Dismissed,
    Expired,
}

impl SuggestionState {
    fn parse(value: &str) -> Self {
        match value {
            "accepted" => SuggestionState::Accepted,
            "dismissed" => SuggestionState::Dismissed,
            "expired" => SuggestionState::Expired,
            _ => SuggestionState::Pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Suggestion {
    pub id: i64,
    pub session_id: i64,
    pub job_id: Option<i64>,
    /// 场次时间，已吸附到关键帧
    pub in_ms: i64,
    pub out_ms: i64,
    pub title: String,
    pub reason: String,
    /// 模型自评（0–1），没给时为空
    pub confidence: Option<f64>,
    pub tags: Vec<String>,
    pub evidence: Evidence,
    pub state: SuggestionState,
    /// 接受时建的切片草稿（切片被删后为空）
    pub clip_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    /// 没处理的候选到这个时刻过期
    pub expires_at: Option<i64>,
}

impl Suggestion {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        let state = SuggestionState::parse(row.try_get("state")?);
        let created_at: i64 = row.try_get("created_at")?;
        let tags: &str = row.try_get("tags")?;
        let evidence: &str = row.try_get("evidence")?;
        Ok(Suggestion {
            id: row.try_get("id")?,
            session_id: row.try_get("session_id")?,
            job_id: row.try_get("job_id")?,
            in_ms: row.try_get("in_ms")?,
            out_ms: row.try_get("out_ms")?,
            title: row.try_get("title")?,
            reason: row.try_get("reason")?,
            confidence: row.try_get("confidence")?,
            tags: serde_json::from_str(tags).unwrap_or_default(),
            evidence: serde_json::from_str(evidence).unwrap_or_default(),
            state,
            clip_id: row.try_get("clip_id")?,
            created_at,
            updated_at: row.try_get("updated_at")?,
            expires_at: (state == SuggestionState::Pending)
                .then(|| created_at.saturating_add(EXPIRE_AFTER_MS)),
        })
    }
}

const COLUMNS: &str = "id, session_id, job_id, in_ms, out_ms, title, reason, confidence, tags, \
     evidence, state, clip_id, created_at, updated_at";

/// 引用分段时用的名义。
pub fn pin_owner(id: i64) -> String {
    format!("suggestion:{id}")
}

/// 换掉这一场没处理的候选：删掉旧的 `pending`（撤销引用），与已接受、已丢弃的重叠率超过 0.5 的
/// 新候选不要，其余插入并引用各自的区间。同一个事务。返回插入的候选（按入点排序）。
pub async fn replace_pending(
    pool: &ConnectionPool,
    session_id: i64,
    job_id: Option<i64>,
    candidates: &[Candidate],
    now: i64,
) -> sqlx::Result<Vec<Suggestion>> {
    let mut tx = pool.begin().await?;
    let stale: Vec<i64> = sqlx::query_scalar(
        "DELETE FROM clip_suggestions WHERE session_id = ? AND state = 'pending' RETURNING id",
    )
    .bind(session_id)
    .fetch_all(&mut *tx)
    .await?;
    for id in stale {
        retention::unpin(&mut tx, &pin_owner(id)).await?;
    }
    let handled: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT in_ms, out_ms FROM clip_suggestions
         WHERE session_id = ? AND state IN ('accepted', 'dismissed')",
    )
    .bind(session_id)
    .fetch_all(&mut *tx)
    .await?;
    let sql = format!(
        "INSERT INTO clip_suggestions
             (session_id, job_id, in_ms, out_ms, title, reason, confidence, tags, evidence, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING {COLUMNS}"
    );
    let mut inserted = Vec::new();
    for candidate in candidates {
        let range = (candidate.in_ms, candidate.out_ms);
        if handled.iter().any(|h| overlaps_too_much(*h, range)) {
            continue;
        }
        let row = sqlx::query(&sql)
            .bind(session_id)
            .bind(job_id)
            .bind(candidate.in_ms)
            .bind(candidate.out_ms)
            .bind(&candidate.title)
            .bind(&candidate.reason)
            .bind(candidate.confidence)
            .bind(serde_json::json!(candidate.tags).to_string())
            .bind(serde_json::json!(candidate.evidence).to_string())
            .bind(now)
            .bind(now)
            .fetch_one(&mut *tx)
            .await?;
        let suggestion = Suggestion::from_row(&row)?;
        retention::pin(
            &mut tx,
            &pin_owner(suggestion.id),
            session_id,
            suggestion.in_ms,
            suggestion.out_ms,
        )
        .await?;
        inserted.push(suggestion);
    }
    tx.commit().await?;
    inserted.sort_by_key(|s| (s.in_ms, s.out_ms));
    Ok(inserted)
}

/// 这一场的全部候选（各种状态），按入点排序。
pub async fn list(pool: &ConnectionPool, session_id: i64) -> sqlx::Result<Vec<Suggestion>> {
    let sql = format!(
        "SELECT {COLUMNS} FROM clip_suggestions WHERE session_id = ? ORDER BY in_ms, out_ms, id"
    );
    sqlx::query(&sql)
        .bind(session_id)
        .fetch_all(pool)
        .await?
        .iter()
        .map(Suggestion::from_row)
        .collect()
}

pub async fn get(pool: &ConnectionPool, id: i64) -> sqlx::Result<Option<Suggestion>> {
    let sql = format!("SELECT {COLUMNS} FROM clip_suggestions WHERE id = ?");
    sqlx::query(&sql)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .as_ref()
        .map(Suggestion::from_row)
        .transpose()
}

/// 接受时可以改的地方；没给的用候选自己的。
#[derive(bon::Builder, Debug, Clone, Default, PartialEq, Eq)]
pub struct Acceptance {
    pub in_ms: Option<i64>,
    pub out_ms: Option<i64>,
    #[builder(into)]
    pub title: Option<String>,
    pub created_by: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AcceptOutcome {
    Accepted {
        suggestion: Box<Suggestion>,
        clip: Box<Clip>,
    },
    NotFound,
    /// 已经接受、丢弃或过期了
    NotPending(Box<Suggestion>),
    /// 这一场的切片已达上限
    TooManyClips,
}

/// 接受：建切片草稿、候选记为 `accepted` 并撤销它的引用（切片自己引用区间），同一个事务。
pub async fn accept(
    pool: &ConnectionPool,
    session_id: i64,
    id: i64,
    acceptance: &Acceptance,
    now: i64,
) -> sqlx::Result<AcceptOutcome> {
    let mut tx = pool.begin().await?;
    let sql = format!("SELECT {COLUMNS} FROM clip_suggestions WHERE id = ? AND session_id = ?");
    let Some(row) = sqlx::query(&sql)
        .bind(id)
        .bind(session_id)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Ok(AcceptOutcome::NotFound);
    };
    let suggestion = Suggestion::from_row(&row)?;
    if suggestion.state != SuggestionState::Pending {
        return Ok(AcceptOutcome::NotPending(Box::new(suggestion)));
    }
    let clips: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM clips WHERE session_id = ?")
        .bind(session_id)
        .fetch_one(&mut *tx)
        .await?;
    if clips >= MAX_CLIPS_PER_SESSION {
        return Ok(AcceptOutcome::TooManyClips);
    }
    let new = NewClip::builder()
        .in_ms(acceptance.in_ms.unwrap_or(suggestion.in_ms))
        .out_ms(acceptance.out_ms.unwrap_or(suggestion.out_ms))
        .title(
            acceptance
                .title
                .clone()
                .unwrap_or_else(|| suggestion.title.clone()),
        )
        .maybe_created_by(acceptance.created_by)
        .created_at(now)
        .build();
    let clip = clips::insert_in(&mut tx, session_id, &new).await?;
    let sql = format!(
        "UPDATE clip_suggestions SET state = 'accepted', clip_id = ?, updated_at = ?
         WHERE id = ? RETURNING {COLUMNS}"
    );
    let row = sqlx::query(&sql)
        .bind(clip.id)
        .bind(now)
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    retention::unpin(&mut tx, &pin_owner(id)).await?;
    tx.commit().await?;
    Ok(AcceptOutcome::Accepted {
        suggestion: Box::new(Suggestion::from_row(&row)?),
        clip: Box::new(clip),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum DismissOutcome {
    Dismissed(Box<Suggestion>),
    NotFound,
    NotPending(Box<Suggestion>),
}

/// 丢弃：记为 `dismissed` 并撤销引用。之后重跑时与它重复的候选不再给出。
pub async fn dismiss(
    pool: &ConnectionPool,
    session_id: i64,
    id: i64,
    now: i64,
) -> sqlx::Result<DismissOutcome> {
    let mut tx = pool.begin().await?;
    let sql = format!(
        "UPDATE clip_suggestions SET state = 'dismissed', updated_at = ?
         WHERE id = ? AND session_id = ? AND state = 'pending' RETURNING {COLUMNS}"
    );
    let row = sqlx::query(&sql)
        .bind(now)
        .bind(id)
        .bind(session_id)
        .fetch_optional(&mut *tx)
        .await?;
    let outcome = match row {
        Some(row) => {
            retention::unpin(&mut tx, &pin_owner(id)).await?;
            DismissOutcome::Dismissed(Box::new(Suggestion::from_row(&row)?))
        }
        None => {
            let sql =
                format!("SELECT {COLUMNS} FROM clip_suggestions WHERE id = ? AND session_id = ?");
            match sqlx::query(&sql)
                .bind(id)
                .bind(session_id)
                .fetch_optional(&mut *tx)
                .await?
            {
                Some(row) => DismissOutcome::NotPending(Box::new(Suggestion::from_row(&row)?)),
                None => DismissOutcome::NotFound,
            }
        }
    };
    tx.commit().await?;
    Ok(outcome)
}

/// 建了 72 小时还没处理的候选记为 `expired` 并撤销引用。返回几条。
pub async fn expire(pool: &ConnectionPool, now: i64) -> sqlx::Result<u64> {
    let mut tx = pool.begin().await?;
    let expired: Vec<i64> = sqlx::query_scalar(
        "UPDATE clip_suggestions SET state = 'expired', updated_at = ?
         WHERE state = 'pending' AND created_at <= ? RETURNING id",
    )
    .bind(now)
    .bind(now.saturating_sub(EXPIRE_AFTER_MS))
    .fetch_all(&mut *tx)
    .await?;
    for id in &expired {
        retention::unpin(&mut tx, &pin_owner(*id)).await?;
    }
    tx.commit().await?;
    Ok(expired.len() as u64)
}

#[cfg(test)]
mod tests;
