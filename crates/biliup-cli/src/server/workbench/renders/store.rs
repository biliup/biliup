use super::model::*;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::{retention, session_groups};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqliteConnection};
use std::path::PathBuf;

const COLS: &str = "id,session_id,clip_id,state,phase,ratio,spec_json,output_path,output_bytes,duration_ms,error,created_at,updated_at,encoder,encoder_device,fallback_reason,fps,speed,processed_ms,total_ms,started_at,finished_at";
const SESSION_COLS: &str = "(SELECT name FROM stream_sessions WHERE id=render_jobs.session_id) AS session_name,(SELECT title FROM stream_sessions WHERE id=render_jobs.session_id) AS session_title";
fn json_error(e: serde_json::Error) -> sqlx::Error {
    sqlx::Error::Decode(Box::new(e))
}
fn job(row: &SqliteRow) -> sqlx::Result<JobRecord> {
    let id = row.try_get("id")?;
    let state: String = row.try_get("state")?;
    let spec: RenderSpec =
        serde_json::from_str(row.try_get::<&str, _>("spec_json")?).map_err(json_error)?;
    let started_at: Option<i64> = row.try_get("started_at")?;
    let finished_at: Option<i64> = row.try_get("finished_at")?;
    let elapsed_ms = started_at
        .map(|start| {
            finished_at
                .unwrap_or_else(crate::server::workbench::recorder::now_ms)
                .saturating_sub(start)
                .max(0)
        })
        .unwrap_or(0);
    let processed_ms: Option<i64> = row.try_get("processed_ms")?;
    let total_ms: Option<i64> = row.try_get::<Option<i64>, _>("total_ms")?.or_else(|| {
        Some(
            spec.sources
                .iter()
                .map(|s| s.trim_end_ms - s.trim_start_ms)
                .sum(),
        )
    });
    let phase: String = row.try_get("phase")?;
    let encoding_sample_visible = state != "running" || phase.starts_with("合成分段 ");
    let speed: Option<f64> = if encoding_sample_visible {
        row.try_get("speed")?
    } else {
        None
    };
    let fps: Option<f64> = if encoding_sample_visible {
        row.try_get("fps")?
    } else {
        None
    };
    let eta_ms = estimate_remaining(&state, processed_ms, total_ms, speed);
    let requested_encoder = spec.recipe.encoding.encoder.name().to_owned();
    let output_path = row
        .try_get::<Option<String>, _>("output_path")?
        .map(PathBuf::from);
    let file_name = output_path
        .as_ref()
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned());
    Ok(JobRecord {
        view: JobView {
            id,
            session_id: row.try_get("session_id")?,
            clip_id: row.try_get("clip_id")?,
            download_url: (state == "ready").then(|| format!("/v1/renders/{id}/download")),
            file_name,
            state,
            phase,
            ratio: row.try_get("ratio")?,
            error: row.try_get("error")?,
            output_bytes: row.try_get("output_bytes")?,
            duration_ms: row.try_get("duration_ms")?,
            created_at: row.try_get("created_at")?,
            session_name: row.try_get("session_name").unwrap_or_default(),
            session_title: row.try_get("session_title").unwrap_or_default(),
            requested_encoder,
            requested_device: spec.recipe.encoding.device.clone(),
            encoder: row.try_get("encoder")?,
            encoder_device: row.try_get("encoder_device")?,
            fallback_reason: row.try_get("fallback_reason")?,
            fps,
            speed,
            processed_ms,
            total_ms,
            started_at,
            finished_at,
            updated_at: row.try_get("updated_at")?,
            elapsed_ms,
            eta_ms,
        },
        spec,
        output_path,
    })
}
/// FFmpeg speed is media seconds per wall-clock second. Preparation and final
/// publication have no reliable ETA; invalid speed samples stay unknown unless
/// media processing has reached its total duration.
fn estimate_remaining(
    state: &str,
    processed: Option<i64>,
    total: Option<i64>,
    speed: Option<f64>,
) -> Option<i64> {
    if state != "running" {
        return None;
    }
    let processed = processed?;
    let total = total?;
    if processed < 0 || total <= 0 || processed > total {
        return None;
    }
    if processed == total {
        return Some(0);
    }
    let speed = speed.filter(|s| s.is_finite() && *s > 0.)?;
    Some(
        ((total - processed) as f64 / speed)
            .ceil()
            .min(i64::MAX as f64) as i64,
    )
}
pub fn pin_owner(id: i64) -> String {
    format!("render:{id}")
}
pub async fn recipe(pool: &ConnectionPool, session_id: i64) -> sqlx::Result<RenderRecipe> {
    let text: Option<String> =
        sqlx::query_scalar("SELECT recipe_json FROM render_recipes WHERE session_id=?")
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
    text.map(|s| serde_json::from_str(&s).map_err(json_error))
        .transpose()
        .map(|r| r.unwrap_or_default())
}
pub async fn save_recipe(
    pool: &ConnectionPool,
    session_id: i64,
    recipe: &RenderRecipe,
    now: i64,
) -> sqlx::Result<()> {
    let mut tx = pool.begin().await?;
    // Check inherited references before replacing the recipe's reference set.
    for id in asset_ids(recipe) {
        if !asset_authorized_connection(&mut tx, session_id, id).await? {
            return Err(sqlx::Error::Protocol(
                "遮挡图片不存在或不属于当前场次".into(),
            ));
        }
    }
    sqlx::query("INSERT INTO render_recipes(session_id,recipe_json,updated_at) VALUES(?,?,?) ON CONFLICT(session_id) DO UPDATE SET recipe_json=excluded.recipe_json,updated_at=excluded.updated_at")
        .bind(session_id).bind(serde_json::to_string(recipe).map_err(json_error)?).bind(now).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM render_recipe_assets WHERE session_id=?")
        .bind(session_id)
        .execute(&mut *tx)
        .await?;
    for id in asset_ids(recipe) {
        sqlx::query("INSERT INTO render_recipe_assets(session_id,asset_id) VALUES(?,?)")
            .bind(session_id)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await
}
const ASSET_AUTHORIZED: &str = "SELECT EXISTS(SELECT 1 FROM render_assets a WHERE a.id=?1 AND (a.session_id=?2 OR EXISTS(SELECT 1 FROM render_recipe_assets r WHERE r.session_id=?2 AND r.asset_id=a.id) OR EXISTS(SELECT 1 FROM render_job_assets ja JOIN render_jobs j ON j.id=ja.job_id WHERE j.session_id=?2 AND ja.asset_id=a.id)))";
pub async fn asset_authorized(
    pool: &ConnectionPool,
    session_id: i64,
    asset_id: i64,
) -> sqlx::Result<bool> {
    sqlx::query_scalar(ASSET_AUTHORIZED)
        .bind(asset_id)
        .bind(session_id)
        .fetch_one(pool)
        .await
}
async fn asset_authorized_connection(
    conn: &mut SqliteConnection,
    session_id: i64,
    asset_id: i64,
) -> sqlx::Result<bool> {
    sqlx::query_scalar(ASSET_AUTHORIZED)
        .bind(asset_id)
        .bind(session_id)
        .fetch_one(conn)
        .await
}
pub fn asset_ids(recipe: &RenderRecipe) -> std::collections::BTreeSet<i64> {
    recipe
        .regions
        .iter()
        .filter(|r| r.effect_type == EffectType::Image)
        .filter_map(|r| r.asset_id)
        .collect()
}
pub async fn insert(pool: &ConnectionPool, spec: &RenderSpec, now: i64) -> sqlx::Result<JobRecord> {
    let _lifecycle = retention::MEDIA_LIFECYCLE.read().await;
    let members = session_groups::timeline(pool, spec.session_id).await?;
    let mut tx = pool.begin().await?;
    // Hold a write transaction while checking all sources, so cleanup cannot race enqueue.
    let sql = format!(
        "INSERT INTO render_jobs(session_id,clip_id,state,phase,spec_json,created_at,updated_at) VALUES(?,?,'queued','queued',?,?,?) RETURNING {COLS}"
    );
    let row = sqlx::query(&sql)
        .bind(spec.session_id)
        .bind(spec.clip_id)
        .bind(serde_json::to_string(spec).map_err(json_error)?)
        .bind(now)
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;
    let record = job(&row)?;
    for source in &spec.sources {
        let source_session_id = if source.source_session_id == 0 {
            spec.session_id
        } else {
            source.source_session_id
        };
        if !members
            .iter()
            .any(|member| member.source_session_id == source_session_id)
        {
            return Err(sqlx::Error::Protocol(format!(
                "分段 {} 的来源不属于当前场次",
                source.segment_id
            )));
        }
        let available: Option<(String, String, Option<String>)> = sqlx::query_as(
            "SELECT state,path,danmaku_path FROM segments WHERE id=? AND session_id=?",
        )
        .bind(source.segment_id)
        .bind(source_session_id)
        .fetch_optional(&mut *tx)
        .await?;
        if !available.is_some_and(|(state, path, xml)| {
            matches!(state.as_str(), "finished" | "pending_delete")
                && PathBuf::from(path) == source.path
                && xml.map(PathBuf::from) == source.danmaku_path
        }) {
            return Err(sqlx::Error::Protocol(format!(
                "分段 {} 已变化或正在清理，请刷新后重试",
                source.segment_id
            )));
        }
        sqlx::query("INSERT INTO render_sources(job_id,segment_id) VALUES(?,?)")
            .bind(record.view.id)
            .bind(source.segment_id)
            .execute(&mut *tx)
            .await?;
    }
    for a in &spec.assets {
        let matching: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM render_assets WHERE id=? AND sha256=?)",
        )
        .bind(a.id)
        .bind(&a.sha256)
        .fetch_one(&mut *tx)
        .await?;
        if !matching || !asset_authorized_connection(&mut tx, spec.session_id, a.id).await? {
            return Err(sqlx::Error::Protocol(
                "合成任务图片快照已变化或不属于当前场次".into(),
            ));
        }
        sqlx::query("INSERT INTO render_job_assets(job_id,asset_id) VALUES(?,?)")
            .bind(record.view.id)
            .bind(a.id)
            .execute(&mut *tx)
            .await?;
    }
    retention::pin(
        &mut tx,
        &pin_owner(record.view.id),
        spec.session_id,
        spec.in_ms,
        spec.out_ms,
    )
    .await?;
    tx.commit().await?;
    Ok(record)
}
pub async fn get(pool: &ConnectionPool, id: i64) -> sqlx::Result<Option<JobRecord>> {
    sqlx::query(&format!(
        "SELECT {COLS},{SESSION_COLS} FROM render_jobs WHERE id=?"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .as_ref()
    .map(job)
    .transpose()
}
pub async fn list(pool: &ConnectionPool, session_id: i64) -> sqlx::Result<Vec<JobView>> {
    sqlx::query(&format!(
        "SELECT {COLS},{SESSION_COLS} FROM render_jobs WHERE session_id=?1 OR session_id IN
         (SELECT source_session_id FROM session_group_members WHERE group_session_id=?1)
         ORDER BY id DESC LIMIT 100"
    ))
    .bind(session_id)
    .fetch_all(pool)
    .await?
    .iter()
    .map(|r| job(r).map(|j| j.view))
    .collect()
}
#[derive(Debug, serde::Serialize)]
pub struct JobPage {
    pub jobs: Vec<JobView>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}
pub async fn list_all(
    pool: &ConnectionPool,
    state: Option<&str>,
    session_id: Option<i64>,
    limit: i64,
    offset: i64,
) -> sqlx::Result<JobPage> {
    let filter = "(?1 IS NULL OR state=?1) AND (?2 IS NULL OR session_id=?2)";
    let mut tx = pool.begin().await?;
    let total: i64 =
        sqlx::query_scalar(&format!("SELECT COUNT(*) FROM render_jobs WHERE {filter}"))
            .bind(state)
            .bind(session_id)
            .fetch_one(&mut *tx)
            .await?;
    let rows = sqlx::query(&format!("SELECT {COLS},{SESSION_COLS} FROM render_jobs WHERE {filter} ORDER BY id DESC LIMIT ?3 OFFSET ?4"))
        .bind(state).bind(session_id).bind(limit).bind(offset).fetch_all(&mut *tx).await?;
    let jobs = rows
        .iter()
        .map(|row| job(row).map(|r| r.view))
        .collect::<sqlx::Result<Vec<_>>>()?;
    tx.commit().await?;
    Ok(JobPage {
        jobs,
        total,
        limit,
        offset,
    })
}
pub async fn running(pool: &ConnectionPool, id: i64, now: i64) -> sqlx::Result<bool> {
    Ok(sqlx::query("UPDATE render_jobs SET state='running',phase='preparing',started_at=?,updated_at=? WHERE id=? AND state='queued'")
        .bind(now).bind(now).bind(id).execute(pool).await?.rows_affected()>0)
}
pub async fn progress(
    pool: &ConnectionPool,
    id: i64,
    phase: &str,
    ratio: Option<f64>,
    now: i64,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE render_jobs SET phase=?,ratio=?,updated_at=? WHERE id=? AND state='running'",
    )
    .bind(phase)
    .bind(ratio)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
pub async fn progress_snapshot(
    pool: &ConnectionPool,
    id: i64,
    progress: &super::engine::EngineProgress,
    now: i64,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE render_jobs SET phase=?,ratio=?,encoder=COALESCE(?,encoder),encoder_device=COALESCE(?,encoder_device),fallback_reason=COALESCE(?,fallback_reason),fps=COALESCE(?,fps),speed=COALESCE(?,speed),processed_ms=?,total_ms=COALESCE(?,total_ms),updated_at=? WHERE id=? AND state='running'")
        .bind(&progress.phase).bind(progress.ratio).bind(&progress.encoder).bind(&progress.encoder_device).bind(&progress.fallback_reason)
        .bind(progress.fps).bind(progress.speed).bind(progress.processed_ms).bind(progress.total_ms).bind(now).bind(id).execute(pool).await?;
    Ok(())
}
/// Keep a database error visible while the worker retries publishing its completed output.
pub async fn finalization_error(
    pool: &ConnectionPool,
    id: i64,
    error: &str,
    now: i64,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE render_jobs SET phase='保存合成结果失败，等待数据库恢复',error=?,updated_at=? WHERE id=? AND state='running'")
        .bind(error)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
pub async fn finish(
    pool: &ConnectionPool,
    id: i64,
    path: &str,
    bytes: i64,
    duration: i64,
    now: i64,
) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;
    let result=sqlx::query("UPDATE render_jobs SET state='ready',phase='ready',ratio=1,output_path=?,output_bytes=?,duration_ms=?,error=NULL,updated_at=?,finished_at=?,processed_ms=total_ms WHERE id=? AND state='running'")
        .bind(path).bind(bytes).bind(duration).bind(now).bind(now).bind(id).execute(&mut *tx).await?;
    if result.rows_affected() > 0 {
        // Publish the clip result with the job in the same transaction. A crash
        // must not leave a finished product paired with an interrupted clip.
        sqlx::query("UPDATE clips SET state='ready', cut_in_ms=(SELECT json_extract(spec_json,'$.in_ms') FROM render_jobs WHERE id=?1),
            cut_out_ms=(SELECT json_extract(spec_json,'$.out_ms') FROM render_jobs WHERE id=?1),
            output_path=?2, output_bytes=?3, duration_ms=?4, error=NULL, active_render_id=NULL, updated_at=?5
            WHERE active_render_id=?1 AND state='exporting'")
            .bind(id).bind(path).bind(bytes).bind(duration).bind(now).execute(&mut *tx).await?;
        release(&mut tx, id).await?;
    }
    tx.commit().await?;
    Ok(result.rows_affected() > 0)
}
async fn release(conn: &mut SqliteConnection, id: i64) -> sqlx::Result<()> {
    retention::unpin(conn, &pin_owner(id)).await?;
    Ok(())
}
pub async fn terminal(
    pool: &ConnectionPool,
    id: i64,
    state: &str,
    error: Option<&str>,
    now: i64,
) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;
    let result=sqlx::query("UPDATE render_jobs SET state=?,phase=?,error=?,updated_at=?,finished_at=? WHERE id=? AND state IN ('queued','running')")
        .bind(state).bind(state).bind(error).bind(now).bind(now).bind(id).execute(&mut *tx).await?;
    if result.rows_affected() > 0 {
        sqlx::query(
            "UPDATE clips SET state='failed', error=?, active_render_id=NULL, updated_at=?
            WHERE active_render_id=? AND state='exporting'",
        )
        .bind(error)
        .bind(now)
        .bind(id)
        .execute(&mut *tx)
        .await?;
        release(&mut tx, id).await?;
    }
    tx.commit().await?;
    Ok(result.rows_affected() > 0)
}
pub async fn recover(pool: &ConnectionPool, now: i64) -> sqlx::Result<()> {
    let ids: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM render_jobs WHERE state IN ('queued','running')")
            .fetch_all(pool)
            .await?;
    for id in ids {
        let record = get(pool, id).await?;
        terminal(pool, id, "failed", Some("合成被服务重启打断，请重试"), now).await?;
        if let Some(cid) = record.and_then(|r| r.spec.clip_id) {
            crate::server::workbench::clips::fail_render_export(
                pool,
                cid,
                id,
                "合成被服务重启打断，请重试",
                now,
            )
            .await?;
        }
    }
    Ok(())
}
/// Freeze paths of job-private supporting files before the worker starts.
pub async fn update_snapshot(
    pool: &ConnectionPool,
    id: i64,
    spec: &RenderSpec,
) -> sqlx::Result<bool> {
    let result = sqlx::query("UPDATE render_jobs SET spec_json=? WHERE id=? AND state='queued'")
        .bind(serde_json::to_string(spec).map_err(json_error)?)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod telemetry_tests {
    use super::estimate_remaining;
    #[test]
    fn eta_requires_active_encoding_and_advancing_valid_speed() {
        assert_eq!(
            estimate_remaining("running", Some(2500), Some(10000), Some(2.)),
            Some(3750)
        );
        for speed in [
            None,
            Some(0.),
            Some(-1.),
            Some(f64::NAN),
            Some(f64::INFINITY),
        ] {
            assert_eq!(
                estimate_remaining("running", Some(2500), Some(10000), speed),
                None
            );
        }
        for state in ["queued", "ready", "failed", "cancelled"] {
            assert_eq!(
                estimate_remaining(state, Some(2500), Some(10000), Some(2.)),
                None
            );
        }
        assert_eq!(
            estimate_remaining("running", Some(10000), Some(10000), Some(2.)),
            Some(0)
        );
        assert_eq!(
            estimate_remaining("running", Some(10000), Some(10000), None),
            Some(0)
        );
        assert_eq!(
            estimate_remaining("running", None, Some(10000), Some(2.)),
            None
        );
    }
}
