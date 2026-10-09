use super::model::*;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::retention;
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqliteConnection};
use std::path::PathBuf;

const COLS: &str = "id,session_id,clip_id,state,phase,ratio,spec_json,output_path,output_bytes,duration_ms,error,created_at";
fn json_error(e: serde_json::Error) -> sqlx::Error {
    sqlx::Error::Decode(Box::new(e))
}
fn job(row: &SqliteRow) -> sqlx::Result<JobRecord> {
    let id = row.try_get("id")?;
    let state: String = row.try_get("state")?;
    Ok(JobRecord {
        view: JobView {
            id,
            session_id: row.try_get("session_id")?,
            clip_id: row.try_get("clip_id")?,
            download_url: (state == "ready").then(|| format!("/v1/renders/{id}/download")),
            state,
            phase: row.try_get("phase")?,
            ratio: row.try_get("ratio")?,
            error: row.try_get("error")?,
            output_bytes: row.try_get("output_bytes")?,
            duration_ms: row.try_get("duration_ms")?,
            created_at: row.try_get("created_at")?,
        },
        spec: serde_json::from_str(row.try_get::<&str, _>("spec_json")?).map_err(json_error)?,
        output_path: row
            .try_get::<Option<String>, _>("output_path")?
            .map(PathBuf::from),
    })
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
    sqlx::query("INSERT INTO render_recipes(session_id,recipe_json,updated_at) VALUES(?,?,?) ON CONFLICT(session_id) DO UPDATE SET recipe_json=excluded.recipe_json,updated_at=excluded.updated_at")
        .bind(session_id).bind(serde_json::to_string(recipe).map_err(json_error)?).bind(now).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM render_recipe_assets WHERE session_id=?")
        .bind(session_id)
        .execute(&mut *tx)
        .await?;
    for id in asset_ids(recipe) {
        let belongs: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM render_assets WHERE id=? AND session_id=?)",
        )
        .bind(id)
        .bind(session_id)
        .fetch_one(&mut *tx)
        .await?;
        if !belongs {
            return Err(sqlx::Error::Protocol(
                "遮挡图片不存在或不属于当前场次".into(),
            ));
        }
        sqlx::query("INSERT INTO render_recipe_assets(session_id,asset_id) VALUES(?,?)")
            .bind(session_id)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await
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
        let available: Option<(String, String, Option<String>)> = sqlx::query_as(
            "SELECT state,path,danmaku_path FROM segments WHERE id=? AND session_id=?",
        )
        .bind(source.segment_id)
        .bind(spec.session_id)
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
            "SELECT EXISTS(SELECT 1 FROM render_assets WHERE id=? AND session_id=? AND sha256=?)",
        )
        .bind(a.id)
        .bind(spec.session_id)
        .bind(&a.sha256)
        .fetch_one(&mut *tx)
        .await?;
        if !matching {
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
    sqlx::query(&format!("SELECT {COLS} FROM render_jobs WHERE id=?"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .as_ref()
        .map(job)
        .transpose()
}
pub async fn list(pool: &ConnectionPool, session_id: i64) -> sqlx::Result<Vec<JobView>> {
    sqlx::query(&format!(
        "SELECT {COLS} FROM render_jobs WHERE session_id=? ORDER BY id DESC LIMIT 100"
    ))
    .bind(session_id)
    .fetch_all(pool)
    .await?
    .iter()
    .map(|r| job(r).map(|j| j.view))
    .collect()
}
pub async fn running(pool: &ConnectionPool, id: i64, now: i64) -> sqlx::Result<bool> {
    Ok(sqlx::query("UPDATE render_jobs SET state='running',phase='preparing',updated_at=? WHERE id=? AND state='queued'")
        .bind(now).bind(id).execute(pool).await?.rows_affected()>0)
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
pub async fn finish(
    pool: &ConnectionPool,
    id: i64,
    path: &str,
    bytes: i64,
    duration: i64,
    now: i64,
) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;
    let result=sqlx::query("UPDATE render_jobs SET state='ready',phase='ready',ratio=1,output_path=?,output_bytes=?,duration_ms=?,error=NULL,updated_at=? WHERE id=? AND state='running'")
        .bind(path).bind(bytes).bind(duration).bind(now).bind(id).execute(&mut *tx).await?;
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
    let result=sqlx::query("UPDATE render_jobs SET state=?,phase=?,error=?,updated_at=? WHERE id=? AND state IN ('queued','running')")
        .bind(state).bind(state).bind(error).bind(now).bind(id).execute(&mut *tx).await?;
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
