//! Application-wide output history, independent of the currently opened editor project.
use crate::server::{
    common::upload_observation::UploadObservations,
    config::Config,
    infrastructure::connection_pool::ConnectionPool,
    workbench::{
        clips,
        clips::export::ClipExports,
        clips::publish::queue::BiliBackend,
        output_publish,
        renders::{RenderJobs, store},
    },
};
use axum::{
    Json,
    body::Bytes,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use biliup::client::StatelessClient;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::Row;
use std::sync::{Arc, RwLock};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessingQuery {
    state: Option<String>,
    task: Option<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}
pub async fn processing(
    State(observations): State<Arc<UploadObservations>>,
    Query(query): Query<ProcessingQuery>,
) -> Response {
    if query
        .state
        .as_deref()
        .is_some_and(|state| !matches!(state, "running" | "ready" | "failed" | "interrupted"))
    {
        return (StatusCode::BAD_REQUEST, "后处理状态无效").into_response();
    }
    match observations
        .processing(
            query.state.as_deref(),
            query.task.as_deref(),
            query.limit.unwrap_or(25).clamp(1, 100),
            query.offset.unwrap_or(0),
        )
        .await
    {
        Ok(page) => Json(page).into_response(),
        Err(error) => {
            tracing::warn!(%error,"读取后处理记录失败");
            (StatusCode::INTERNAL_SERVER_ERROR, "读取后处理记录失败").into_response()
        }
    }
}

fn publication_response<T: serde::Serialize>(result: Result<T, output_publish::Error>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(output_publish::Error::Invalid(message)) => {
            (StatusCode::BAD_REQUEST, message).into_response()
        }
        Err(output_publish::Error::Conflict(message)) => {
            (StatusCode::CONFLICT, message).into_response()
        }
        Err(output_publish::Error::NotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!(%error,"成片投稿状态读取或保存失败");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "成片投稿状态读取或保存失败",
            )
                .into_response()
        }
    }
}
pub async fn publish_preview(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path((kind, id)): Path<(String, i64)>,
    Json(input): Json<output_publish::Preview>,
) -> Response {
    publication_response(output_publish::preview(&pool, &jobs, &kind, id, input).await)
}
pub async fn publish(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    State(observations): State<Arc<UploadObservations>>,
    State(config): State<Arc<RwLock<Config>>>,
    State(client): State<StatelessClient>,
    Path((kind, id)): Path<(String, i64)>,
    Json(input): Json<output_publish::Submit>,
) -> Response {
    publication_response(
        output_publish::submit(
            pool,
            jobs,
            observations,
            Arc::new(BiliBackend::new(config, client).preserve_files()),
            kind,
            id,
            input,
        )
        .await,
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverUpload {
    name: String,
}
pub async fn cover_upload(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path((kind, id)): Path<(String, i64)>,
    Query(input): Query<CoverUpload>,
    body: Result<Bytes, axum::extract::rejection::BytesRejection>,
) -> Response {
    let bytes = match body {
        Ok(bytes) => bytes,
        Err(_) => return (StatusCode::PAYLOAD_TOO_LARGE, "封面图片最大 5 MiB").into_response(),
    };
    publication_response(
        output_publish::covers::upload(&pool, &jobs, &kind, id, &input.name, bytes.to_vec()).await,
    )
}
pub async fn cover_frame(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path((kind, id)): Path<(String, i64)>,
    Json(input): Json<output_publish::covers::Frame>,
) -> Response {
    publication_response(output_publish::covers::frame(&pool, &jobs, &kind, id, input.at_ms).await)
}
pub async fn cover_image(
    State(pool): State<ConnectionPool>,
    Path((kind, id, cover)): Path<(String, i64, String)>,
) -> Response {
    match output_publish::covers::bytes(&pool, &kind, id, &cover).await {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "image/jpeg"),
                (header::CACHE_CONTROL, "no-store"),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            bytes,
        )
            .into_response(),
        Err(error) => publication_response::<()>(Err(error)),
    }
}

#[derive(Deserialize, Default)]
pub struct List {
    state: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    kind: Option<String>,
    id: Option<i64>,
}

pub async fn list(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    State(exports): State<Arc<ClipExports>>,
    Query(query): Query<List>,
) -> Response {
    if query
        .kind
        .as_deref()
        .is_some_and(|kind| !matches!(kind, "editor" | "render" | "clip"))
        || query.id.is_some_and(|id| id <= 0)
    {
        return (StatusCode::BAD_REQUEST, "成片标识无效").into_response();
    }
    if query
        .state
        .as_deref()
        .is_some_and(|s| !matches!(s, "queued" | "running" | "ready" | "failed" | "cancelled"))
    {
        return (StatusCode::BAD_REQUEST, "任务状态无效").into_response();
    }
    match read(&pool, &jobs, &exports, query).await {
        Ok(data) => Json(data).into_response(),
        Err(error) => {
            tracing::warn!(%error,"读取成片失败");
            (StatusCode::INTERNAL_SERVER_ERROR, "读取成片失败").into_response()
        }
    }
}
async fn read(
    pool: &ConnectionPool,
    jobs: &RenderJobs,
    exports: &ClipExports,
    query: List,
) -> Result<Value, String> {
    // Clip-backed renders already have a task row. Do not list that same file or attempt twice.
    let union = "SELECT 'editor' kind,id,project_id,created_at,CASE phase WHEN 'done' THEN 'ready' WHEN 'preparing' THEN 'running' WHEN 'rendering' THEN 'running' ELSE phase END state,job_json, json_extract(document_json,'$.project.name') name,EXISTS(SELECT 1 FROM editor_projects p WHERE p.id=editor_exports.project_id) project_exists FROM editor_exports
        UNION ALL SELECT 'render' kind,id,NULL project_id,created_at,state,NULL job_json,NULL name,0 project_exists FROM render_jobs
        UNION ALL SELECT 'clip' kind,c.id,NULL project_id,c.created_at,CASE c.state WHEN 'exporting' THEN 'running' WHEN 'published' THEN 'ready' ELSE c.state END state,NULL job_json,(SELECT title FROM stream_sessions s WHERE s.id=c.session_id) name,0 project_exists FROM clips c
        WHERE c.state IN ('exporting','ready','failed','published') AND NOT EXISTS(SELECT 1 FROM render_jobs r WHERE r.clip_id=c.id AND (r.id=c.active_render_id OR (c.output_path IS NOT NULL AND r.output_path=c.output_path) OR (c.state='failed' AND r.state IN ('failed','cancelled') AND r.finished_at>=c.updated_at)))";
    let selected = format!(
        "SELECT * FROM ({union}) WHERE (?1 IS NULL OR state=?1) AND (?2 IS NULL OR kind=?2) AND (?3 IS NULL OR id=?3)"
    );
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM ({selected})"))
        .bind(&query.state)
        .bind(&query.kind)
        .bind(query.id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    let rows = sqlx::query(&format!(
        "SELECT *,CASE kind WHEN 'editor' THEN (SELECT retry.id FROM editor_exports retry WHERE retry.retry_of=selected.id) WHEN 'render' THEN (SELECT retry.id FROM render_jobs retry WHERE retry.retry_of=selected.id) END retry_id FROM ({selected}) selected ORDER BY created_at DESC,kind,id DESC LIMIT ?4 OFFSET ?5"
    ))
    .bind(&query.state)
    .bind(&query.kind)
    .bind(query.id)
    .bind(query.limit.unwrap_or(25).clamp(1, 100))
    .bind(query.offset.unwrap_or(0).max(0))
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())?;
    let mut items = Vec::new();
    for row in rows {
        let id: i64 = row.get("id");
        let kind: &str = row.get("kind");
        let mut publish_clip = None;
        let job = if kind == "editor" {
            match jobs.editor_exports.live_snapshot(id) {
                Some(job) => job,
                None => serde_json::from_str(row.get::<&str, _>("job_json"))
                    .map_err(|e| e.to_string())?,
            }
        } else if kind == "render" {
            let Some(job) = store::get(pool, id).await.map_err(|e| e.to_string())? else {
                continue;
            };
            if job.view.state == "ready"
                && let Some(clip_id) = job.view.clip_id
                && let Some(clip) = clips::get(pool, clip_id).await.map_err(|e| e.to_string())?
                && matches!(clip.state, clips::State::Ready | clips::State::Published)
                && job.output_path.is_some()
                && clip.output_path.as_deref().map(std::path::Path::new)
                    == job.output_path.as_deref()
            {
                publish_clip = Some(super::clips::view(clip, exports));
            }
            serde_json::to_value(job.view).map_err(|e| e.to_string())?
        } else {
            let Some(clip) = clips::get(pool, id).await.map_err(|e| e.to_string())? else {
                continue;
            };
            serde_json::to_value(super::clips::view(clip, exports)).map_err(|e| e.to_string())?
        };
        let publication = if kind == "editor" || kind == "render" {
            output_publish::latest(pool, kind, id)
                .await
                .map_err(|error| error.to_string())?
        } else {
            None
        };
        items.push(json!({"id":id,"kind":kind,"project_id":row.get::<Option<i64>,_>("project_id"),"project_name":row.get::<Option<String>,_>("name"),"project_exists":row.get::<bool,_>("project_exists"),"retry_id":row.get::<Option<i64>,_>("retry_id"),"created_at":row.get::<i64,_>("created_at"),"job":job,"clip":publish_clip,"publication":publication}));
    }
    Ok(json!({"jobs":items,"total":total}))
}
