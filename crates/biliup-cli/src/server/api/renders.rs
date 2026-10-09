//! Post-recording composition APIs. Browser input contains IDs and recipes,
//! never server-side paths. Reads use `file.view`, edits use `clip.edit`.

use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::clips::publish::queue::ClipPublisher;
use crate::server::workbench::renders::{RenderJobs, RenderRecipe, assets, engine, store};
use crate::server::workbench::{recorder, store as recordings};
use axum::Json;
use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, FromRef, Path, Query, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{MethodRouter, get};
use serde::{Deserialize, Serialize};
use std::path::{Path as FilePath, PathBuf};
use std::sync::{Arc, LazyLock};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;
use tower_http::services::ServeFile;

// Debounced browser edits can still overlap during slow XML conversion. Bound
// preview processes independently from the video encoding budget.
static PREVIEW_SLOTS: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(2));

fn internal(error: impl std::fmt::Display) -> Response {
    tracing::warn!(%error, "录后合成接口失败");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "读取或保存合成数据失败，请查看服务日志",
    )
        .into_response()
}

fn bad_request(error: impl Into<String>) -> Response {
    (StatusCode::BAD_REQUEST, error.into()).into_response()
}

fn conflict(error: impl Into<String>) -> Response {
    (StatusCode::CONFLICT, error.into()).into_response()
}

fn not_found(message: &'static str) -> Response {
    (StatusCode::NOT_FOUND, message).into_response()
}

async fn require_session(pool: &ConnectionPool, session_id: i64) -> Result<(), Response> {
    match recordings::session(pool, session_id).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(not_found("场次不存在")),
        Err(error) => Err(internal(error)),
    }
}

/// `GET /v1/sessions/{id}/render-recipe` (also `/render-settings`).
pub async fn get_render_recipe(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(response) = require_session(&pool, id).await {
        return response;
    }
    match store::recipe(&pool, id).await {
        Ok(recipe) => Json(recipe).into_response(),
        Err(error) => internal(error),
    }
}

/// Saves a recipe only after validating all asset IDs belong to this session.
pub async fn put_render_recipe(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path(id): Path<i64>,
    Json(recipe): Json<RenderRecipe>,
) -> Response {
    if let Err(response) = require_session(&pool, id).await {
        return response;
    }
    if let Err(error) = jobs.validate_assets(id, &recipe).await {
        return bad_request(error);
    }
    match store::save_recipe(&pool, id, &recipe, recorder::now_ms()).await {
        Ok(()) => Json(recipe).into_response(),
        Err(error) => internal(error),
    }
}

pub async fn list_render_assets(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
) -> Response {
    if let Err(response) = require_session(&pool, id).await {
        return response;
    }
    match assets::list(&pool, id).await {
        Ok(assets) => Json(serde_json::json!({ "assets": assets })).into_response(),
        Err(error) => internal(error),
    }
}

/// Raw PNG/JPEG request body, capped before buffering. No multipart filenames
/// or filesystem paths enter the asset store.
pub fn render_assets_route<S>() -> MethodRouter<S>
where
    S: Clone + Send + Sync + 'static,
    ConnectionPool: FromRef<S>,
    Arc<RenderJobs>: FromRef<S>,
{
    get(list_render_assets)
        .post(upload_render_asset)
        .layer(DefaultBodyLimit::max(assets::MAX_ASSET_BYTES))
}

pub async fn upload_render_asset(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = require_session(&pool, id).await {
        return response;
    }
    let mime = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !matches!(
        mime.split(';').next().unwrap_or("").trim(),
        "image/png" | "image/jpeg"
    ) {
        return (
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "请直接上传 PNG/JPEG 图片，Content-Type 必须为 image/png 或 image/jpeg",
        )
            .into_response();
    }
    match assets::upload(&pool, jobs.root(), id, mime, &body, recorder::now_ms()).await {
        Ok(asset) => (StatusCode::CREATED, Json(asset)).into_response(),
        Err(error) => bad_request(error),
    }
}

/// Canonicalize managed files so a substituted symlink cannot expose unrelated
/// files even when it has a valid asset/job ID.
async fn owned_file(root: &FilePath, path: &FilePath) -> Result<PathBuf, Response> {
    let root = tokio::fs::canonicalize(root)
        .await
        .map_err(|_| conflict("合成资源目录已丢失"))?;
    let path = tokio::fs::canonicalize(path)
        .await
        .map_err(|_| conflict("合成资源文件已丢失"))?;
    if !path.starts_with(&root) || !tokio::fs::metadata(&path).await.is_ok_and(|m| m.is_file()) {
        return Err(conflict("合成资源路径无效"));
    }
    Ok(path)
}

pub async fn get_render_asset(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path(aid): Path<i64>,
    request: Request,
) -> Response {
    let asset = match assets::get(&pool, aid).await {
        Ok(Some(asset)) => asset,
        Ok(None) => return not_found("图片不存在"),
        Err(error) => return internal(error),
    };
    let path = match owned_file(jobs.root(), &asset.render.path).await {
        Ok(path) => path,
        Err(response) => return response,
    };
    let mut response = match ServeFile::new(path).oneshot(request).await {
        Ok(response) => response.map(Body::new),
        Err(error) => return internal(error),
    };
    if let Ok(value) = HeaderValue::from_str(&asset.mime) {
        response.headers_mut().insert(header::CONTENT_TYPE, value);
    }
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=31536000, immutable"),
    );
    response
}

pub async fn delete_render_asset(
    State(pool): State<ConnectionPool>,
    Path(aid): Path<i64>,
) -> Response {
    match assets::delete(&pool, aid).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => not_found("图片不存在"),
        Err(error) => conflict(error),
    }
}

pub async fn list_renders(State(pool): State<ConnectionPool>, Path(id): Path<i64>) -> Response {
    if let Err(response) = require_session(&pool, id).await {
        return response;
    }
    match store::list(&pool, id).await {
        Ok(jobs) => Json(serde_json::json!({ "jobs": jobs })).into_response(),
        Err(error) => internal(error),
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRender {
    pub clip_id: Option<i64>,
    /// Absent = snapshot the session's saved recipe.
    pub recipe: Option<RenderRecipe>,
}

pub async fn create_render(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    State(publisher): State<Arc<ClipPublisher>>,
    Path(id): Path<i64>,
    Json(body): Json<CreateRender>,
) -> Response {
    if let Err(response) = require_session(&pool, id).await {
        return response;
    }
    if body.clip_id.is_some_and(|id| id <= 0) {
        return bad_request("切片 ID 无效");
    }
    if body
        .clip_id
        .is_some_and(|cid| publisher.state_of(cid).is_some())
    {
        return conflict("切片正在发布队列中，请先移出队列再合成");
    }
    let recipe = match body.recipe {
        Some(recipe) => recipe,
        None => match store::recipe(&pool, id).await {
            Ok(recipe) => recipe,
            Err(error) => return internal(error),
        },
    };
    let spec = match jobs.make_spec(id, body.clip_id, None, recipe).await {
        Ok(spec) => spec,
        Err(error) => return bad_request(error),
    };
    // enqueue owns the guarded transition to exporting and retains the clip's
    // previous product. Beginning a normal export here would discard it.
    match jobs.enqueue(spec).await {
        Ok(job) => (StatusCode::ACCEPTED, Json(job)).into_response(),
        Err(error) => conflict(error),
    }
}

pub async fn get_render(State(pool): State<ConnectionPool>, Path(jid): Path<i64>) -> Response {
    match store::get(&pool, jid).await {
        Ok(Some(job)) => Json(job.view).into_response(),
        Ok(None) => not_found("合成任务不存在"),
        Err(error) => internal(error),
    }
}

pub async fn cancel_render(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path(jid): Path<i64>,
) -> Response {
    match store::get(&pool, jid).await {
        Ok(Some(job)) if matches!(job.view.state.as_str(), "queued" | "running") => {}
        Ok(Some(job)) if job.view.state == "cancelled" => {
            return StatusCode::ACCEPTED.into_response();
        }
        Ok(Some(_)) => return conflict("只有排队或运行中的合成任务可以取消"),
        Ok(None) => return not_found("合成任务不存在"),
        Err(error) => return internal(error),
    }
    match jobs.cancel(jid).await {
        Ok(true) => StatusCode::ACCEPTED.into_response(),
        Ok(false) => {
            // The worker can acknowledge the cancelled token before cancel()
            // updates the row. A completed cancellation is still successful.
            match store::get(&pool, jid).await {
                Ok(Some(job)) if job.view.state == "cancelled" => {
                    StatusCode::ACCEPTED.into_response()
                }
                Ok(_) => conflict("任务已经结束，请刷新任务列表"),
                Err(error) => internal(error),
            }
        }
        Err(error) => conflict(error),
    }
}

pub async fn retry_render(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    State(publisher): State<Arc<ClipPublisher>>,
    Path(jid): Path<i64>,
) -> Response {
    match store::get(&pool, jid).await {
        Ok(Some(record)) => {
            if record
                .view
                .clip_id
                .is_some_and(|cid| publisher.state_of(cid).is_some())
            {
                return conflict("切片正在发布队列中，请先移出队列再合成");
            }
        }
        Ok(None) => return not_found("合成任务不存在"),
        Err(error) => return internal(error),
    }
    match jobs.retry(jid).await {
        Ok(job) => (StatusCode::ACCEPTED, Json(job)).into_response(),
        Err(error) => conflict(error),
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DownloadQuery {
    pub attachment: bool,
}

/// Inline by default for playback; `?attachment=true` downloads. ServeFile
/// handles Range and HEAD without loading the MP4 into memory.
pub async fn download_render(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path(jid): Path<i64>,
    Query(query): Query<DownloadQuery>,
    request: Request,
) -> Response {
    let record = match store::get(&pool, jid).await {
        Ok(Some(record)) => record,
        Ok(None) => return not_found("合成任务不存在"),
        Err(error) => return internal(error),
    };
    if record.view.state != "ready" {
        return conflict("合成尚未完成");
    }
    let Some(path) = record.output_path else {
        return conflict("合成产物不存在");
    };
    let path = match owned_file(jobs.root(), &path).await {
        Ok(path) => path,
        Err(response) => return response,
    };
    let mut response = match ServeFile::new(path).oneshot(request).await {
        Ok(response) => response.map(Body::new),
        Err(error) => return internal(error),
    };
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("video/mp4"));
    let disposition = format!(
        "{}; filename=\"render-{jid}.mp4\"",
        if query.attachment {
            "attachment"
        } else {
            "inline"
        }
    );
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, value);
    }
    response
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewRequest {
    pub recipe: RenderRecipe,
    pub from_ms: i64,
    pub to_ms: i64,
    #[serde(default)]
    pub canvas_segment_id: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PreviewView {
    pub ass: String,
    pub font_urls: Vec<String>,
    pub width: u32,
    pub height: u32,
    /// Subtract from the scene clock to obtain the ASS clock.
    pub origin_ms: i64,
    pub estimated_timing: bool,
    pub comment_count: usize,
}

pub async fn preview_render(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path(id): Path<i64>,
    Json(body): Json<PreviewRequest>,
) -> Response {
    if let Err(response) = require_session(&pool, id).await {
        return response;
    }
    if body.from_ms < 0 || body.to_ms <= body.from_ms {
        return bad_request("预览范围无效");
    }
    let spec = match jobs
        .make_spec(id, None, Some((body.from_ms, body.to_ms)), body.recipe)
        .await
    {
        Ok(spec) => spec,
        Err(error) => return bad_request(error),
    };
    if spec.sources.len() != 1 {
        return bad_request("弹幕预览请限定在一个录像分段内；切换分段时重新请求预览");
    }
    let source = &spec.sources[0];
    let _preview_permit = match PREVIEW_SLOTS.acquire().await {
        Ok(permit) => permit,
        Err(error) => return internal(error),
    };
    let canvas_path: Option<String> = if let Some(canvas_id) = body.canvas_segment_id {
        match sqlx::query_scalar("SELECT path FROM segments WHERE id=? AND session_id=? AND state IN ('finished','pending_delete')")
            .bind(canvas_id).bind(id).fetch_optional(&pool).await {
            Ok(Some(path)) => Some(path),
            Ok(None) => return bad_request("预览画布分段不存在或不属于当前场次"),
            Err(error) => return internal(error),
        }
    } else {
        None
    };
    match engine::preview_on_canvas(
        source,
        &spec.recipe,
        &CancellationToken::new(),
        canvas_path.as_deref().map(FilePath::new),
    )
    .await
    {
        Ok(preview) => Json(PreviewView {
            ass: preview.ass,
            font_urls: font_urls(),
            width: preview.width,
            height: preview.height,
            origin_ms: source.start_ms + preview.time_origin_ms,
            estimated_timing: preview.estimated_timing,
            comment_count: preview.comment_count,
        })
        .into_response(),
        Err(error) => bad_request(format!("分段 {}：{error}", source.segment_id)),
    }
}

fn font_urls() -> Vec<String> {
    crate::tools::render_font()
        .map(|_| vec!["/v1/render-fonts/0".into()])
        .unwrap_or_default()
}

pub async fn render_tools() -> Response {
    Json(serde_json::json!({ "tools": engine::capabilities().await, "font_urls": font_urls() }))
        .into_response()
}

pub async fn get_render_font(Path(id): Path<usize>, request: Request) -> Response {
    if id != 0 {
        return not_found("字体不存在");
    }
    let path = match crate::tools::render_font() {
        Some(path) => path,
        None => return not_found("字体不存在"),
    };
    // Files come exclusively from the configured/bundled font inventory.
    let mut response = match ServeFile::new(path).oneshot(request).await {
        Ok(response) => response.map(Body::new),
        Err(error) => return internal(error),
    };
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=3600"),
    );
    response
}

#[cfg(test)]
mod tests;
