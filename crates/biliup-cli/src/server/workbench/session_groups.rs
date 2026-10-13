//! Virtual workbench timelines. Original sessions, objects and media remain unchanged.
use super::{live, retention};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use serde::Serialize;
use sqlx::{Row, SqliteConnection};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, LazyLock, Mutex};

type GroupChanges = HashMap<(usize, i64), tokio::sync::watch::Sender<u64>>;
static CHANGES: LazyLock<Mutex<GroupChanges>> = LazyLock::new(|| Mutex::new(HashMap::new()));
fn namespace(pool: &ConnectionPool) -> usize {
    Arc::as_ptr(&pool.connect_options()) as usize
}
pub fn updates(pool: &ConnectionPool, id: i64) -> tokio::sync::watch::Receiver<u64> {
    let mut groups = CHANGES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    groups.retain(|_, sender| sender.receiver_count() > 0);
    groups
        .entry((namespace(pool), id))
        .or_insert_with(|| tokio::sync::watch::channel(0).0)
        .subscribe()
}
fn changed(pool: &ConnectionPool, id: i64) {
    let mut groups = CHANGES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    groups.retain(|_, sender| sender.receiver_count() > 0);
    if let Some(sender) = groups.get(&(namespace(pool), id)) {
        sender.send_modify(|value| *value = value.wrapping_add(1));
    }
}

pub const SUGGESTION_WINDOW_MS: i64 = 30 * 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow, Serialize)]
pub struct TimelineMember {
    pub source_session_id: i64,
    pub offset_ms: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MemberView {
    pub source_session_id: i64,
    pub offset_ms: i64,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub title: String,
    pub duration_ms: i64,
    pub gap_before_ms: i64,
}

pub async fn timeline(pool: &ConnectionPool, id: i64) -> sqlx::Result<Vec<TimelineMember>> {
    sqlx::query_as(
        "SELECT source_session_id,offset_ms,ordinal FROM session_group_members WHERE group_session_id=?1
         UNION ALL SELECT id,0,0 FROM stream_sessions WHERE id=?1 AND kind='source'
         ORDER BY ordinal",
    )
    .bind(id)
    .fetch_all(pool)
    .await
}

pub async fn view_offset(
    pool: &ConnectionPool,
    view: i64,
    owner: i64,
) -> sqlx::Result<Option<i64>> {
    if view == owner {
        return Ok(Some(0));
    }
    sqlx::query_scalar("SELECT offset_ms FROM session_group_members WHERE group_session_id=? AND source_session_id=?")
        .bind(view).bind(owner).fetch_optional(pool).await
}

pub async fn active_source(pool: &ConnectionPool, id: i64) -> sqlx::Result<Option<TimelineMember>> {
    Ok(timeline(pool, id)
        .await?
        .last()
        .copied()
        .filter(|m| live::is_recording(m.source_session_id)))
}

pub async fn is_recording(pool: &ConnectionPool, id: i64) -> sqlx::Result<bool> {
    Ok(active_source(pool, id).await?.is_some())
}

pub async fn members(pool: &ConnectionPool, id: i64) -> sqlx::Result<Vec<MemberView>> {
    let mut result: Vec<MemberView> = sqlx::query_as(
        "SELECT m.source_session_id,m.offset_ms,s.started_at,s.ended_at,s.title,
         COALESCE((SELECT MAX(COALESCE(g.end_ms,g.start_ms)) FROM segments g WHERE g.session_id=s.id),0) AS duration_ms,
         0 AS gap_before_ms FROM session_group_members m JOIN stream_sessions s ON s.id=m.source_session_id
         WHERE m.group_session_id=? ORDER BY m.ordinal"
    ).bind(id).fetch_all(pool).await?;
    let mut previous_end = None;
    for member in &mut result {
        member.gap_before_ms = previous_end.map_or(0, |end: i64| (member.offset_ms - end).max(0));
        previous_end = Some(member.offset_ms + member.duration_ms);
    }
    Ok(result)
}

#[derive(Debug, thiserror::Error)]
pub enum MergeError {
    #[error("{0}")]
    Invalid(String),
    #[error("候选不存在或已处理")]
    NotFound,
    #[error("database: {0}")]
    Db(#[from] sqlx::Error),
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MergeCandidate {
    pub id: i64,
    pub streamer_id: Option<i64>,
    pub streamer_name: String,
    pub previous_session_id: i64,
    pub source_session_id: i64,
    pub group_session_id: Option<i64>,
    pub gap_ms: i64,
    pub previous_title: String,
    pub source_title: String,
    pub previous_started_at: i64,
    pub source_started_at: i64,
    pub recipe_diff: bool,
}

pub async fn candidates(pool: &ConnectionPool) -> sqlx::Result<Vec<MergeCandidate>> {
    // Persist the suggestion identity before returning it, including declined suggestions.
    sqlx::query(
        "INSERT OR IGNORE INTO session_merge_candidates(previous_session_id,source_session_id,created_at)
         SELECT p.id,s.id,?1 FROM stream_sessions s JOIN stream_sessions p ON p.id=(
             SELECT x.id FROM stream_sessions x WHERE x.kind='source' AND x.streamer_id=s.streamer_id
              AND x.started_at<s.started_at ORDER BY x.started_at DESC,x.id DESC LIMIT 1)
         WHERE s.kind='source' AND s.started_at IS NOT NULL AND p.ended_at IS NOT NULL
           AND s.started_at>=p.ended_at AND s.started_at-p.ended_at<=?2
           AND NOT EXISTS(SELECT 1 FROM session_group_members m WHERE m.source_session_id=s.id)",
    ).bind(super::recorder::now_ms()).bind(SUGGESTION_WINDOW_MS).execute(pool).await?;
    let mut candidates:Vec<MergeCandidate> = sqlx::query_as(
        "SELECT c.id,s.streamer_id,s.name AS streamer_name,c.previous_session_id,c.source_session_id,
         m.group_session_id,s.started_at-p.ended_at AS gap_ms,p.title AS previous_title,s.title AS source_title,
         p.started_at AS previous_started_at,s.started_at AS source_started_at,
         0 AS recipe_diff
         FROM session_merge_candidates c JOIN stream_sessions p ON p.id=c.previous_session_id
         JOIN stream_sessions s ON s.id=c.source_session_id
         LEFT JOIN session_group_members m ON m.source_session_id=p.id
         WHERE c.state='pending' AND NOT EXISTS(SELECT 1 FROM session_group_members n WHERE n.source_session_id=s.id)
           AND (m.group_session_id IS NULL OR m.ordinal=(SELECT MAX(t.ordinal) FROM session_group_members t WHERE t.group_session_id=m.group_session_id))
         ORDER BY s.started_at DESC,c.id DESC",
    ).fetch_all(pool).await?;
    for candidate in &mut candidates {
        let previous = super::renders::store::recipe(
            pool,
            candidate
                .group_session_id
                .unwrap_or(candidate.previous_session_id),
        )
        .await?;
        let source = super::renders::store::recipe(pool, candidate.source_session_id).await?;
        candidate.recipe_diff = previous != source;
    }
    Ok(candidates)
}

pub async fn dismiss(pool: &ConnectionPool, id: i64) -> sqlx::Result<bool> {
    Ok(sqlx::query("UPDATE session_merge_candidates SET state='dismissed',resolved_at=? WHERE id=? AND state='pending'")
        .bind(super::recorder::now_ms()).bind(id).execute(pool).await?.rows_affected()>0)
}

pub async fn confirm(pool: &ConnectionPool, id: i64) -> Result<i64, MergeError> {
    let _lifecycle = retention::MEDIA_LIFECYCLE.read().await;
    let mut tx = pool.begin().await?;
    let pair: Option<(i64,i64)> = sqlx::query_as("SELECT previous_session_id,source_session_id FROM session_merge_candidates WHERE id=? AND state='pending'")
        .bind(id).fetch_optional(&mut *tx).await?;
    let Some((previous, source)) = pair else {
        return Err(MergeError::NotFound);
    };
    let group = create_in(&mut tx, &[previous, source]).await?;
    sqlx::query("UPDATE session_merge_candidates SET state='confirmed',group_session_id=?,resolved_at=? WHERE id=? AND state='pending'")
        .bind(group).bind(super::recorder::now_ms()).bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    changed(pool, group);
    Ok(group)
}

pub async fn create(pool: &ConnectionPool, ids: &[i64]) -> Result<i64, MergeError> {
    let _lifecycle = retention::MEDIA_LIFECYCLE.read().await;
    let mut tx = pool.begin().await?;
    let id = create_in(&mut tx, ids).await?;
    tx.commit().await?;
    changed(pool, id);
    Ok(id)
}

async fn create_in(conn: &mut SqliteConnection, ids: &[i64]) -> Result<i64, MergeError> {
    if ids.len() < 2
        || ids.len() > 100
        || ids.iter().copied().collect::<BTreeSet<_>>().len() != ids.len()
    {
        return Err(MergeError::Invalid(
            "请选择 2 到 100 个不重复的来源场次".into(),
        ));
    }
    let mut group = None;
    let mut all = BTreeSet::new();
    for &id in ids {
        let existing: Option<i64> = sqlx::query_scalar(
            "SELECT group_session_id FROM session_group_members WHERE source_session_id=?",
        )
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?;
        if let Some(existing) = existing {
            if group.is_some_and(|g| g != existing) {
                return Err(MergeError::Invalid("来源已属于不同合并场次".into()));
            }
            group = Some(existing);
        }
        all.insert(id);
    }
    let existing_members: Vec<(i64, i64)> = if let Some(group) = group {
        sqlx::query_as("SELECT source_session_id,offset_ms FROM session_group_members WHERE group_session_id=? ORDER BY ordinal")
            .bind(group).fetch_all(&mut *conn).await?
    } else {
        Vec::new()
    };
    for &(id, _) in &existing_members {
        all.insert(id);
    }
    let mut sources = Vec::new();
    for id in all {
        let row=sqlx::query("SELECT id,streamer_id,started_at,ended_at,kind,
            COALESCE((SELECT MAX(COALESCE(end_ms,start_ms)) FROM segments WHERE session_id=s.id),0) AS duration_ms
            FROM stream_sessions s WHERE id=?").bind(id).fetch_optional(&mut *conn).await?;
        let Some(row) = row else {
            return Err(MergeError::Invalid("来源场次不存在".into()));
        };
        if row.try_get::<&str, _>("kind")? != "source" {
            return Err(MergeError::Invalid("合并场次不能嵌套".into()));
        }
        let started: Option<i64> = row.try_get("started_at")?;
        let streamer: Option<i64> = row.try_get("streamer_id")?;
        let Some(started) = started else {
            return Err(MergeError::Invalid("来源场次还没有录像时间轴".into()));
        };
        let Some(streamer) = streamer else {
            return Err(MergeError::Invalid("来源场次没有关联主播".into()));
        };
        sources.push((
            id,
            streamer,
            started,
            row.try_get::<Option<i64>, _>("ended_at")?,
            row.try_get::<i64, _>("duration_ms")?,
        ));
    }
    sources.sort_by_key(|s| (s.2, s.0));
    if sources.iter().any(|s| s.1 != sources[0].1) {
        return Err(MergeError::Invalid("只能合并同一主播的场次".into()));
    }
    for pair in sources.windows(2) {
        let Some(ended) = pair[0].3 else {
            return Err(MergeError::Invalid(
                "只有最后一个来源场次可以仍在录制".into(),
            ));
        };
        if ended > pair[1].2 {
            return Err(MergeError::Invalid("来源场次时间重叠，无法合并".into()));
        }
    }
    if !existing_members.is_empty()
        && sources
            .iter()
            .take(existing_members.len())
            .map(|s| s.0)
            .ne(existing_members.iter().map(|s| s.0))
    {
        return Err(MergeError::Invalid("只能在已有合并场次末尾追加来源".into()));
    }
    let id = if let Some(id) = group {
        id
    } else {
        let id: i64 =
            sqlx::query_scalar("INSERT INTO stream_session_ids DEFAULT VALUES RETURNING id")
                .fetch_one(&mut *conn)
                .await?;
        sqlx::query("INSERT INTO stream_sessions(id,name,url,title,date,live_cover_path,streamer_id,started_at,ended_at,kind)
           SELECT ?1,name,url,title,date,live_cover_path,streamer_id,started_at,ended_at,'group' FROM stream_sessions WHERE id=?2")
            .bind(id).bind(sources[0].0).execute(&mut *conn).await?;
        sqlx::query("INSERT INTO render_recipes(session_id,recipe_json,updated_at) SELECT ?,recipe_json,updated_at FROM render_recipes WHERE session_id=?")
            .bind(id).bind(sources[0].0).execute(&mut *conn).await?;
        sqlx::query("INSERT INTO render_recipe_assets(session_id,asset_id) SELECT ?,asset_id FROM render_recipe_assets WHERE session_id=?")
            .bind(id).bind(sources[0].0).execute(&mut *conn).await?;
        id
    };
    let mut offset = 0;
    for (ordinal, source) in sources.iter().enumerate() {
        if ordinal > 0 {
            let previous = &sources[ordinal - 1];
            offset += previous.4 + (source.2 - previous.3.unwrap()).max(0);
        }
        if let Some((_, fixed)) = existing_members.get(ordinal) {
            offset = *fixed;
            continue;
        }
        sqlx::query("INSERT INTO session_group_members(group_session_id,source_session_id,ordinal,offset_ms) VALUES(?,?,?,?)")
            .bind(id).bind(source.0).bind(ordinal as i64).bind(offset).execute(&mut *conn).await?;
    }
    for source in &sources {
        retention::refresh_pin_counts(&mut *conn, source.0).await?;
    }
    sqlx::query(
        "UPDATE session_merge_candidates SET state='confirmed',group_session_id=?,resolved_at=?
         WHERE state='pending' AND source_session_id IN
         (SELECT source_session_id FROM session_group_members WHERE group_session_id=?)",
    )
    .bind(id)
    .bind(super::recorder::now_ms())
    .bind(id)
    .execute(&mut *conn)
    .await?;
    Ok(id)
}

#[cfg(test)]
mod tests;
