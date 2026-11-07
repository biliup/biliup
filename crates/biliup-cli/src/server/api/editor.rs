//! Saved editing projects use the same authentication/permissions as the workbench.
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::editor::{self, Document, Error};
use crate::server::workbench::renders::jobs::RenderJobs;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;
pub mod agent;
pub async fn shaders(State(pool): State<ConnectionPool>, Path(project): Path<i64>) -> Response {
    response(editor::shaders::list(&pool, project).await)
}

#[derive(Deserialize)]
pub struct ExportRequest {
    revision: i64,
    options: Value,
}
fn export_response<T: serde::Serialize>(result: Result<T, String>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}
pub async fn generators(State(jobs): State<Arc<RenderJobs>>) -> Response {
    export_response(jobs.editor_generate.catalog().await)
}
pub async fn graphic_write(
    State(pool): State<ConnectionPool>,
    Path(project): Path<i64>,
    Json(input): Json<editor::graphics::Write>,
) -> Response {
    response(editor::graphics::write(&pool, project, input).await)
}
pub async fn graphic_read(
    State(pool): State<ConnectionPool>,
    Path((project, name)): Path<(i64, String)>,
    Query(query): Query<GraphicQuery>,
) -> Response {
    let result = async {
        let mut graphic =
            editor::graphics::read(&pool, project, &format!("graphics/{name}")).await?;
        if !query.raw {
            graphic["html"] = json!(
                editor::graphics::resolve(graphic["html"].as_str().unwrap())
                    .map_err(Error::Invalid)?
            )
        }
        Ok::<_, Error>(graphic)
    }
    .await;
    response(result)
}
#[derive(Deserialize)]
pub struct GraphicQuery {
    #[serde(default)]
    raw: bool,
}
pub async fn render_assets(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path(project): Path<i64>,
) -> Response {
    response(editor::fonts::resources(&pool, jobs.root(), project).await)
}
pub async fn project_fonts(
    State(pool): State<ConnectionPool>,
    Path(project): Path<i64>,
) -> Response {
    response(editor::project_fonts::list(&pool, project).await)
}
pub async fn proxies(State(jobs): State<Arc<RenderJobs>>, Path(project): Path<i64>) -> Response {
    response(jobs.editor_proxies.list(project).await)
}
pub async fn recent_media(
    State(pool): State<ConnectionPool>,
    Path(project): Path<i64>,
    Query(query): Query<editor::recent_media::Query>,
) -> Response {
    response(editor::recent_media::search(&pool, project, query).await)
}
pub async fn recent_import(
    State(pool): State<ConnectionPool>,
    Path((project, id)): Path<(i64, String)>,
) -> Response {
    response(editor::recent_media::import(&pool, project, &id).await)
}
pub async fn recent_thumb(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path((project, id)): Path<(i64, String)>,
    request: axum::extract::Request,
) -> Response {
    let _lifecycle = crate::server::workbench::retention::MEDIA_LIFECYCLE
        .read()
        .await;
    match editor::recent_media::thumbnail_source(&pool, project, &id).await {
        Ok((path, source)) => serve_media(&jobs, path, source, "thumb", request).await,
        Err(error) => response::<()>(Err(error)),
    }
}
pub async fn proxies_sync(
    State(jobs): State<Arc<RenderJobs>>,
    Path(project): Path<i64>,
) -> Response {
    response(jobs.editor_proxies.sync(project).await)
}
pub async fn proxy_cancel(
    State(jobs): State<Arc<RenderJobs>>,
    Path((project, id)): Path<(i64, String)>,
) -> Response {
    response(jobs.editor_proxies.cancel(project, &id).await)
}
pub async fn proxy_retry(
    State(jobs): State<Arc<RenderJobs>>,
    Path((project, id)): Path<(i64, String)>,
) -> Response {
    response(jobs.editor_proxies.retry(project, &id).await)
}
pub async fn project_font_upload(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path(project): Path<i64>,
    Query(input): Query<editor::project_fonts::Upload>,
    bytes: axum::body::Bytes,
) -> Response {
    response(editor::project_fonts::upload(&pool, jobs.root(), project, input, &bytes).await)
}
#[derive(Deserialize)]
pub struct GraphicMediaQuery {
    path: String,
    #[serde(default)]
    t: f64,
    w: Option<u32>,
    h: Option<u32>,
    p: Option<String>,
}
pub async fn graphic_media(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path((project, id)): Path<(i64, String)>,
    Query(query): Query<GraphicMediaQuery>,
) -> Response {
    let lifecycle = crate::server::workbench::retention::MEDIA_LIFECYCLE
        .read()
        .await;
    let result = async {
        let ask = editor::graphic_media::Request {
            id,
            t: query.t,
            w: query.w,
            h: query.h,
        };
        ask.validate().map_err(Error::Invalid)?;
        let (entry, source) =
            editor::media::registered(&pool, project, &ask.id, Some(&query.path)).await?;
        let kind = entry["kind"].as_str().ok_or(Error::NotFound)?;
        let path = editor::media::source_path(&pool, &source).await?;
        let cancel = tokio_util::sync::CancellationToken::new();
        let _cancel = cancel.clone().drop_guard();
        let proxy = if query.p.as_deref() == Some("1") && kind == "video" {
            jobs.editor_proxies
                .cached(project, &path, &source, None)
                .await
                .ok()
        } else {
            None
        };
        if let Some(proxy) = proxy {
            let full = editor::graphic_media::PREVIEW
                .display_size(&path, &ask, &cancel)
                .await
                .map_err(Error::Invalid)?;
            // Proxy cache files are disposable. Keep the source lifecycle guard until any
            // fallback completes, but do not retain it in a decoder for a derived cache file.
            if let Ok(picture) = editor::graphic_media::PREVIEW
                .picture(&proxy, kind, &ask, &cancel, None)
                .await
            {
                return Ok((picture, Some(full)));
            }
        }
        let picture = editor::graphic_media::PREVIEW
            .picture(&path, kind, &ask, &cancel, Some(lifecycle))
            .await
            .map_err(Error::Invalid)?;
        Ok::<_, Error>((picture, None))
    }
    .await;
    match result {
        Ok((picture, full)) => {
            let mut response = (
                [
                    (axum::http::header::CONTENT_TYPE, picture.mime),
                    (axum::http::header::CACHE_CONTROL, "no-store"),
                    (axum::http::header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
                    (
                        axum::http::header::CONTENT_SECURITY_POLICY,
                        "sandbox; default-src 'none'; style-src 'unsafe-inline'; img-src data:",
                    ),
                ],
                (*picture.bytes).clone(),
            )
                .into_response();
            if let Some(frame) = picture.frame
                && let Ok(value) = frame.to_string().parse()
            {
                response.headers_mut().insert("x-editor-frame", value);
            }
            if let Some((width, height)) = full
                && let Ok(value) = format!("{width}x{height}").parse()
            {
                response.headers_mut().insert("x-biliup-full", value);
            }
            response
        }
        Err(error) => response::<()>(Err(error)),
    }
}
pub async fn model_settings(State(pool): State<ConnectionPool>) -> Response {
    export_response(editor::text_model::get(&pool).await)
}
pub async fn save_model_settings(
    State(pool): State<ConnectionPool>,
    Json(settings): Json<editor::text_model::Settings>,
) -> Response {
    export_response(editor::text_model::save(&pool, settings).await)
}
pub async fn generation_jobs(State(jobs): State<Arc<RenderJobs>>, Path(id): Path<i64>) -> Response {
    export_response(jobs.editor_generate.list(id).await)
}
pub async fn generation_start(
    State(jobs): State<Arc<RenderJobs>>,
    Path(id): Path<i64>,
    Json(req): Json<editor::generate::Request>,
) -> Response {
    export_response(
        jobs.editor_generate
            .start(id, req)
            .await
            .map(|id| json!({"jobId":id})),
    )
}
pub async fn generation_cancel(
    State(jobs): State<Arc<RenderJobs>>,
    Path((project, job)): Path<(i64, String)>,
) -> Response {
    export_response(jobs.editor_generate.cancel(project, &job).await)
}
pub async fn generation_result(
    State(jobs): State<Arc<RenderJobs>>,
    Path((project, job)): Path<(i64, String)>,
) -> Response {
    export_response(jobs.editor_generate.result(project, &job).await)
}
pub async fn exports(State(jobs): State<Arc<RenderJobs>>, Path(id): Path<i64>) -> Response {
    export_response(jobs.editor_exports.list(id).await)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoudnessRequest {
    revision: i64,
    target_lufs: f64,
}
pub async fn export_loudness(
    State(jobs): State<Arc<RenderJobs>>,
    Path(id): Path<i64>,
    Json(input): Json<LoudnessRequest>,
) -> Response {
    let stream = jobs
        .editor_exports
        .estimate_stream(id, input.revision, input.target_lufs);
    let mut response = axum::body::Body::from_stream(stream).into_response();
    response.headers_mut().insert(
        "content-type",
        axum::http::HeaderValue::from_static("application/x-ndjson; charset=utf-8"),
    );
    response.headers_mut().insert(
        "cache-control",
        axum::http::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        "x-accel-buffering",
        axum::http::HeaderValue::from_static("no"),
    );
    response
}
pub async fn export_create(
    State(jobs): State<Arc<RenderJobs>>,
    Path(id): Path<i64>,
    Json(input): Json<ExportRequest>,
) -> Response {
    export_response(
        jobs.editor_exports
            .enqueue(id, input.revision, input.options)
            .await
            .map(|id| json!({"jobId":id})),
    )
}
pub async fn export_cancel(
    State(jobs): State<Arc<RenderJobs>>,
    Path((id, job)): Path<(i64, i64)>,
) -> Response {
    export_response(jobs.editor_exports.cancel(id, job).await)
}
pub async fn export_retry(
    State(jobs): State<Arc<RenderJobs>>,
    Path((project, job)): Path<(i64, i64)>,
) -> Response {
    export_response(
        jobs.editor_exports
            .retry(project, job)
            .await
            .map(|job_id| json!({"jobId":job_id})),
    )
}
#[derive(Deserialize)]
pub struct ExportDownloadQuery {
    #[serde(default)]
    attachment: bool,
    #[serde(default)]
    subtitles: bool,
}
pub async fn export_download(
    State(jobs): State<Arc<RenderJobs>>,
    Path((id, job)): Path<(i64, i64)>,
    Query(query): Query<ExportDownloadQuery>,
    request: axum::extract::Request,
) -> Response {
    let result = if query.subtitles {
        jobs.editor_exports.download_subtitles(id, job).await
    } else {
        jobs.editor_exports.download(id, job).await
    };
    let (path, name) = match result {
        Ok(path) => path,
        Err(error) => return (StatusCode::NOT_FOUND, error).into_response(),
    };
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("mp4")
        .to_string();
    match tower_http::services::ServeFile::new(path)
        .oneshot(request)
        .await
    {
        Ok(response) => {
            let mut response = response.into_response();
            if query.attachment {
                response.headers_mut().insert(
                    axum::http::header::CONTENT_DISPOSITION,
                    format!(
                        "attachment; filename=\"export-{job}.{extension}\"; filename*=UTF-8''{}",
                        urlencoding::encode(&name)
                    )
                    .parse()
                    .unwrap(),
                );
            }
            response
        }
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}

fn response<T: serde::Serialize>(result: Result<T, Error>) -> Response {
    match result {
        Ok(value) => Json(value).into_response(),
        Err(Error::Invalid(message)) => (StatusCode::BAD_REQUEST, message).into_response(),
        Err(error @ Error::NotFound) => (StatusCode::NOT_FOUND, error.to_string()).into_response(),
        Err(error @ Error::Conflict) => (StatusCode::CONFLICT, error.to_string()).into_response(),
        Err(error) => {
            tracing::warn!(%error, "剪辑工程接口失败");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "读取或保存工程失败，请查看服务日志",
            )
                .into_response()
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalysisRequest {
    media_id: String,
    version: String,
    model: Option<String>,
    analyzers: Option<Vec<String>>,
    restore_job: Option<String>,
}
#[derive(Deserialize)]
pub struct AnalysisQuery {
    analyzer: String,
    omit: Option<String>,
    version: String,
    model: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRequest {
    id: String,
    scope: String,
    value: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceDelete {
    id: String,
    scope: String,
}
#[derive(Deserialize)]
pub struct BrandFileQuery {
    name: String,
}
pub async fn brand_files(State(pool): State<ConnectionPool>, Path(project): Path<i64>) -> Response {
    response(editor::resources::files(&pool, project).await)
}
pub async fn brand_state(State(pool): State<ConnectionPool>, Path(project): Path<i64>) -> Response {
    response(
        editor::resources::brand::snapshot(&pool, project)
            .await
            .map(|snapshot| snapshot.state),
    )
}
#[derive(Deserialize)]
pub struct BrandImportRequest {
    url: String,
}
pub async fn brand_import(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path((project, scope, kit)): Path<(i64, String, String)>,
    Json(input): Json<BrandImportRequest>,
) -> Response {
    response(
        editor::resources::importer::download(
            &pool,
            jobs.root(),
            project,
            &scope,
            &kit,
            &input.url,
        )
        .await
        .map(|name| json!({"name":name})),
    )
}
pub async fn brand_upload(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path((project, scope, kit)): Path<(i64, String, String)>,
    Query(input): Query<BrandFileQuery>,
    bytes: axum::body::Bytes,
) -> Response {
    response(
        editor::resources::add_file(
            &pool,
            jobs.root(),
            project,
            &scope,
            &kit,
            &input.name,
            &bytes,
        )
        .await
        .map(|name| json!({"name":name})),
    )
}
pub async fn brand_file(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path((project, scope, kit, name)): Path<(i64, String, String, String)>,
    request: axum::extract::Request,
) -> Response {
    let path = match editor::resources::file(&pool, jobs.root(), project, &scope, &kit, &name).await
    {
        Ok(path) => path,
        Err(error) => return response::<()>(Err(error)),
    };
    match tower_http::services::ServeFile::new(path)
        .oneshot(request)
        .await
    {
        Ok(response) => {
            let mut response = response.into_response();
            response.headers_mut().insert(
                axum::http::header::CONTENT_SECURITY_POLICY,
                "default-src 'none'; sandbox".parse().unwrap(),
            );
            response.headers_mut().insert(
                axum::http::header::X_CONTENT_TYPE_OPTIONS,
                "nosniff".parse().unwrap(),
            );
            response
        }
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}
pub async fn resource_list(
    State(pool): State<ConnectionPool>,
    Path((project, kind)): Path<(i64, String)>,
) -> Response {
    response(editor::resources::list(&pool, project, &kind).await)
}
pub async fn sound_folders(
    State(pool): State<ConnectionPool>,
    Path(project): Path<i64>,
) -> Response {
    response(editor::sound_library::list(&pool, project).await)
}
pub async fn provider_keys(State(pool): State<ConnectionPool>) -> Response {
    export_response(editor::keys::list(&pool).await)
}
pub async fn online_library_state(State(jobs): State<Arc<RenderJobs>>) -> Response {
    export_response(jobs.editor_library.state().await)
}
pub async fn online_library_search(
    State(jobs): State<Arc<RenderJobs>>,
    Json(query): Json<editor::online_library::Query>,
) -> Response {
    export_response(jobs.editor_library.search(query).await)
}
pub async fn online_library_add(
    State(jobs): State<Arc<RenderJobs>>,
    Path((project, source, id)): Path<(i64, String, String)>,
) -> Response {
    export_response(jobs.editor_library.add(project, &source, &id).await)
}
pub async fn library_add(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path((project, id)): Path<(i64, String)>,
) -> Response {
    let cancel = tokio_util::sync::CancellationToken::new();
    let _cancel = cancel.clone().drop_guard();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(300),
        editor::library::add(
            editor::Storage {
                pool: &pool,
                root: jobs.root(),
            },
            &jobs.editor_library,
            project,
            &id,
            200 * 1024 * 1024,
            &cancel,
            |_, _| {},
        ),
    )
    .await;
    response(match result {
        Ok(value) => value,
        Err(_) => Err(editor::Error::Invalid("音源导入超时".into())),
    })
}
pub async fn online_library_preview(
    State(jobs): State<Arc<RenderJobs>>,
    Path((source, id)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
) -> Response {
    let remote = match jobs
        .editor_library
        .preview(
            &source,
            &id,
            headers.get("range").and_then(|h| h.to_str().ok()),
        )
        .await
    {
        Ok(value) => value,
        Err(error) => return export_response::<()>(Err(error)),
    };
    if !remote.status().is_success() {
        return StatusCode::from_u16(remote.status().as_u16())
            .unwrap_or(StatusCode::BAD_GATEWAY)
            .into_response();
    }
    let mut response = Response::builder()
        .status(remote.status().as_u16())
        .header("content-security-policy", "default-src 'none'; sandbox")
        .header("x-content-type-options", "nosniff");
    let mime = remote
        .headers()
        .get("content-type")
        .and_then(|h| h.to_str().ok())
        .filter(|mime| mime.starts_with("audio/") || mime.starts_with("application/ogg"))
        .unwrap_or("application/octet-stream");
    response = response.header("content-type", mime);
    for name in ["content-range", "accept-ranges"] {
        if let Some(value) = remote.headers().get(name) {
            response = response.header(name, value)
        }
    }
    response
        .body(axum::body::Body::from_stream(editor::public_http::stream(
            remote,
            50 * 1024 * 1024,
        )))
        .unwrap()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyRequest {
    key: String,
}
pub async fn save_provider_key(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path(provider): Path<String>,
    Json(input): Json<KeyRequest>,
) -> Response {
    export_response(editor::keys::set(&pool, jobs.root(), &provider, &input.key).await)
}
pub async fn remove_provider_key(
    State(pool): State<ConnectionPool>,
    Path(provider): Path<String>,
) -> Response {
    export_response(editor::keys::remove(&pool, &provider).await)
}
pub async fn save_sound_folder(
    State(pool): State<ConnectionPool>,
    Path(project): Path<i64>,
    Json(folder): Json<editor::sound_library::Folder>,
) -> Response {
    response(editor::sound_library::save(&pool, project, folder).await)
}
pub async fn delete_sound_folder(
    State(pool): State<ConnectionPool>,
    Path((project, id)): Path<(i64, String)>,
) -> Response {
    response(editor::sound_library::delete(&pool, project, &id).await)
}
pub async fn library_sound(
    State(pool): State<ConnectionPool>,
    Path((project, id, kind)): Path<(i64, String, String)>,
    request: axum::extract::Request,
) -> Response {
    let (value, path) = match editor::sound_library::sound(&pool, project, &id).await {
        Ok(value) => value,
        Err(error) => return response::<()>(Err(error)),
    };
    if kind == "metadata" {
        return Json(value).into_response();
    }
    if kind != "file" {
        return StatusCode::NOT_FOUND.into_response();
    }
    match tower_http::services::ServeFile::new(path)
        .oneshot(request)
        .await
    {
        Ok(response) => response.into_response(),
        Err(error) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response(),
    }
}
pub async fn resource_save(
    State(pool): State<ConnectionPool>,
    Path((project, kind)): Path<(i64, String)>,
    Json(input): Json<ResourceRequest>,
) -> Response {
    response(
        editor::resources::save(&pool, project, &kind, &input.id, &input.scope, input.value).await,
    )
}
pub async fn resource_delete(
    State(pool): State<ConnectionPool>,
    Path((project, kind)): Path<(i64, String)>,
    Json(input): Json<ResourceDelete>,
) -> Response {
    response(editor::resources::delete(&pool, project, &kind, &input.id, &input.scope).await)
}
pub async fn transcription_state(State(jobs): State<Arc<RenderJobs>>) -> Response {
    Json(jobs.editor_analysis.models.state().await).into_response()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRequest {
    model: Option<String>,
}
pub async fn transcription_download(
    State(jobs): State<Arc<RenderJobs>>,
    Json(input): Json<ModelRequest>,
) -> Response {
    let models = &jobs.editor_analysis.models;
    let id = match input.model {
        Some(id) => id,
        None => models.selected().await,
    };
    export_response(models.download(id).await)
}
pub async fn transcription_cancel(
    State(jobs): State<Arc<RenderJobs>>,
    Json(input): Json<ModelRequest>,
) -> Response {
    let models = &jobs.editor_analysis.models;
    let id = match input.model {
        Some(id) => id,
        None => models.selected().await,
    };
    export_response(models.cancel(&id))
}
pub async fn transcription_select(
    State(jobs): State<Arc<RenderJobs>>,
    Json(input): Json<ModelRequest>,
) -> Response {
    match input.model {
        Some(id) => export_response(jobs.editor_analysis.models.select(&id).await),
        None => (StatusCode::BAD_REQUEST, "请选择语音模型").into_response(),
    }
}
pub async fn setup_checks(State(pool): State<ConnectionPool>) -> Response {
    let model = match editor::text_model::get(&pool).await {
        Ok(model) => model,
        Err(error) => return export_response::<()>(Err(error)),
    };
    let keys = match editor::keys::list(&pool).await {
        Ok(keys) => keys,
        Err(error) => return export_response::<()>(Err(error)),
    };
    let model_ready = keys
        .iter()
        .any(|k| k["provider"] == model.provider && k["set"] == true);
    let browser = editor::export::available();
    let whisper = editor::analysis::speech_models::binary();
    let ffmpeg = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        crate::tools::ffmpeg_command()
            .arg("-version")
            .kill_on_drop(true)
            .output(),
    )
    .await;
    let ready = ffmpeg.is_ok_and(|result| result.is_ok_and(|output| output.status.success()));
    Json(json!([
        {"id":"model","name":"文本与图形模型","status":if model_ready{"ready"}else{"missing"},"optional":true,"detail":if model_ready{format!("{} · {}",model.provider,model.model)}else{"尚未设置 API 密钥".into()}},
        {"id":"renderer","name":"Chromium","status":if browser.is_ok(){"ready"}else{"missing"},"optional":false,"detail":browser.err().unwrap_or("图形渲染器已就绪".into())},
        {"id":"ffmpeg","name":"FFmpeg","status":if ready{"ready"}else{"missing"},"optional":false,"detail":if ready{"已就绪"}else{"服务器未找到可用的 FFmpeg"},"fix":{"text":"在服务器安装 FFmpeg 并加入 PATH，或配置 ffmpeg_path"}},
        {"id":"whisper","name":"whisper.cpp","status":if whisper.is_some(){"ready"}else{"missing"},"optional":true,"detail":if whisper.is_some(){"本地语音转写已就绪"}else{"服务器尚未安装 whisper-cli"},"fix":{"text":"安装 whisper.cpp 并加入 PATH，或设置 BILIUP_WHISPER_BIN"}}
    ])).into_response()
}
pub async fn analysis_jobs(
    State(jobs): State<Arc<RenderJobs>>,
    Path(project): Path<i64>,
) -> Response {
    response(jobs.editor_analysis.list(project).await)
}
pub async fn analysis_start(
    State(jobs): State<Arc<RenderJobs>>,
    Path(project): Path<i64>,
    Json(input): Json<AnalysisRequest>,
) -> Response {
    if let Some(job) = input.restore_job {
        if input.analyzers.is_some() || input.model.is_some() {
            return response::<Vec<Value>>(Err(editor::Error::Invalid(
                "恢复分析不接受新的分析器参数".into(),
            )));
        }
        return response(
            jobs.editor_analysis
                .retry_restore(project, &input.media_id, &input.version, &job)
                .await,
        );
    }
    let transcript = input
        .analyzers
        .as_ref()
        .is_some_and(|items| items.iter().any(|item| item == "transcript"));
    if transcript != input.model.is_some() {
        return response::<Vec<Value>>(Err(editor::Error::Invalid(
            "转写请求须指定当前语音模型，其他分析不接受语音模型".into(),
        )));
    }
    response(
        jobs.editor_analysis
            .start_configured(
                project,
                input.media_id,
                input.analyzers,
                "auto",
                input.model.as_deref(),
                Some(&input.version),
            )
            .await,
    )
}
pub async fn analysis_cancel(
    State(jobs): State<Arc<RenderJobs>>,
    Path((project, job)): Path<(i64, String)>,
) -> Response {
    response(jobs.editor_analysis.cancel(project, &job).await)
}
pub async fn analysis_cached(
    State(jobs): State<Arc<RenderJobs>>,
    Path((project, media)): Path<(i64, String)>,
    Query(input): Query<AnalysisQuery>,
) -> Response {
    if (input.analyzer == "transcript") != input.model.is_some() {
        return response::<Value>(Err(editor::Error::Invalid(
            "转写读取须指定当前语音模型，其他分析不接受语音模型".into(),
        )));
    }
    let omit = input
        .omit
        .unwrap_or_default()
        .split(',')
        .take(20)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    response(
        jobs.editor_analysis
            .cached_version(
                project,
                &media,
                editor::analysis::CacheQuery {
                    analyzer: &input.analyzer,
                    requested_language: "auto",
                    version: Some(&input.version),
                    selected_model: input.model.as_deref(),
                },
                &omit,
            )
            .await,
    )
}

#[derive(Deserialize)]
pub struct ListQuery {
    session_id: Option<i64>,
    before: Option<i64>,
    q: Option<String>,
    limit: Option<i64>,
}
pub async fn list(State(pool): State<ConnectionPool>, Query(query): Query<ListQuery>) -> Response {
    response(
        editor::list_filtered(
            &pool,
            query.session_id,
            query.before,
            query.q.as_deref().unwrap_or(""),
            query.limit.unwrap_or(100),
        )
        .await,
    )
}
pub async fn get(State(pool): State<ConnectionPool>, Path(id): Path<i64>) -> Response {
    response(editor::get(&pool, id).await)
}
pub async fn create(State(pool): State<ConnectionPool>, Json(doc): Json<Document>) -> Response {
    let mut result = response(editor::save(&pool, None, doc).await);
    if result.status() == StatusCode::OK {
        *result.status_mut() = StatusCode::CREATED;
    }
    result
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveRequest {
    revision: i64,
    document: Document,
    #[serde(default)]
    generated: Vec<String>,
    history: Option<editor::history::UserNote>,
    suggestion: Option<editor::suggestions::Acceptance>,
}
pub async fn update(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
    Json(req): Json<SaveRequest>,
) -> Response {
    response(
        editor::save_user_results(
            &pool,
            (id, req.revision),
            req.document,
            &req.generated,
            req.history.as_ref(),
            req.suggestion.as_ref(),
        )
        .await,
    )
}
#[derive(Deserialize)]
pub struct HistoryQuery {
    before: Option<i64>,
}
pub async fn history(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
    Query(query): Query<HistoryQuery>,
) -> Response {
    response(editor::history::page(&pool, id, query.before).await)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryStep {
    revision: i64,
    action: editor::history::cursor::Action,
    target: Option<i64>,
}
pub async fn history_step(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
    Json(request): Json<HistoryStep>,
) -> Response {
    response(
        editor::history::cursor::step(&pool, id, request.revision, request.action, request.target)
            .await,
    )
}
#[derive(Deserialize)]
pub struct RevisionQuery {
    revision: i64,
}
pub async fn delete(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path(id): Path<i64>,
    Query(req): Query<RevisionQuery>,
) -> Response {
    match editor::delete(&pool, id, req.revision).await {
        Ok(()) => {
            jobs.editor_agent.forget(id);
            jobs.editor_proxies.forget(id).await;
            StatusCode::NO_CONTENT.into_response()
        }
        Err(error) => response::<()>(Err(error)),
    }
}
pub async fn revision(
    State(pool): State<ConnectionPool>,
    Path((id, rev)): Path<(i64, i64)>,
) -> Response {
    response(editor::revision(&pool, id, rev).await)
}

pub async fn revisions(State(pool): State<ConnectionPool>, Path(id): Path<i64>) -> Response {
    response(editor::revisions(&pool, id).await)
}

#[derive(Deserialize)]
pub struct MediaQuery {
    v: Option<String>,
    path: Option<String>,
}
pub async fn media(
    State(pool): State<ConnectionPool>,
    State(jobs): State<std::sync::Arc<crate::server::workbench::renders::jobs::RenderJobs>>,
    Path((id, media, kind)): Path<(i64, String, String)>,
    Query(query): Query<MediaQuery>,
    request: axum::extract::Request,
) -> Response {
    let _lifecycle = crate::server::workbench::retention::MEDIA_LIFECYCLE
        .read()
        .await;
    if kind == "proxy" {
        return match jobs
            .editor_proxies
            .file(id, &media, query.v.as_deref())
            .await
        {
            Ok(path) => match tower_http::services::ServeFile::new(path)
                .oneshot(request)
                .await
            {
                Ok(value) => {
                    let mut response = value.into_response();
                    response.headers_mut().insert(
                        axum::http::header::CACHE_CONTROL,
                        axum::http::HeaderValue::from_static("private, no-cache"),
                    );
                    response.headers_mut().insert(
                        axum::http::header::X_CONTENT_TYPE_OPTIONS,
                        axum::http::HeaderValue::from_static("nosniff"),
                    );
                    response
                }
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            },
            Err(error) => response::<()>(Err(error)),
        };
    }
    let (path, source) =
        match editor::media::version(&pool, id, &media, query.path.as_deref()).await {
            Ok(value) => value,
            Err(error) => return response::<()>(Err(error)),
        };
    serve_media(&jobs, path, source, &kind, request).await
}

pub async fn session_media(
    State(pool): State<ConnectionPool>,
    State(jobs): State<std::sync::Arc<crate::server::workbench::renders::jobs::RenderJobs>>,
    Path((id, media, kind)): Path<(i64, String, String)>,
    request: axum::extract::Request,
) -> Response {
    let _lifecycle = crate::server::workbench::retention::MEDIA_LIFECYCLE
        .read()
        .await;
    match editor::media::session_source(&pool, id, &media).await {
        Ok((path, source)) if kind == "probe" => {
            let source = editor::MediaSource::Recording(source);
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(90),
                editor::media::probe(&pool, &path, &source, 30.),
            )
            .await;
            match result {
                Ok(Ok(probe)) => Json(probe).into_response(),
                Ok(Err(error)) => {
                    tracing::warn!(%error, session=id, media, "读取导入录像画布失败");
                    response::<()>(Err(Error::Invalid(
                        "无法读取录像信息，请检查文件与 FFmpeg 后重试导入".into(),
                    )))
                }
                Err(_) => response::<()>(Err(Error::Invalid(
                    "录像信息读取超时，请稍后重试导入".into(),
                ))),
            }
        }
        Ok((path, source)) => {
            serve_media(
                &jobs,
                path,
                editor::MediaSource::Recording(source),
                &kind,
                request,
            )
            .await
        }
        Err(error) => response::<()>(Err(error)),
    }
}

async fn serve_media(
    jobs: &crate::server::workbench::renders::jobs::RenderJobs,
    path: std::path::PathBuf,
    source: editor::MediaSource,
    kind: &str,
    request: axum::extract::Request,
) -> Response {
    use tower::ServiceExt;
    let mut layout = None;
    let path = match kind {
        "file" => path,
        "waveform" => {
            return match editor::waveform::generate(jobs.root(), &path, &source).await {
                Ok(Some(value)) => Json(value).into_response(),
                Ok(None) => StatusCode::NO_CONTENT.into_response(),
                Err(error) => {
                    tracing::warn!(%error,"生成音频波形失败");
                    (StatusCode::UNPROCESSABLE_ENTITY, "无法生成音频波形").into_response()
                }
            };
        }
        "thumb" | "filmstrip" => {
            match editor::media::picture(jobs.root(), &path, &source, kind == "filmstrip").await {
                Ok((file, info)) => {
                    if kind == "filmstrip" {
                        layout = Some(info);
                    }
                    file
                }
                Err(error) => {
                    tracing::warn!(%error, "剪辑素材缩略图失败");
                    return (StatusCode::UNPROCESSABLE_ENTITY, "无法生成素材缩略图")
                        .into_response();
                }
            }
        }
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    match tower_http::services::ServeFile::new(path)
        .oneshot(request)
        .await
    {
        Ok(value) => {
            let mut value = value.into_response();
            value.headers_mut().insert(axum::http::header::CONTENT_SECURITY_POLICY,axum::http::HeaderValue::from_static("sandbox; default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:"));
            value.headers_mut().insert(
                axum::http::header::X_CONTENT_TYPE_OPTIONS,
                axum::http::HeaderValue::from_static("nosniff"),
            );
            value.headers_mut().insert(
                axum::http::header::CACHE_CONTROL,
                axum::http::HeaderValue::from_static("private, max-age=60"),
            );
            if let Some(layout) = layout
                && let Ok(header) = layout.to_string().parse()
            {
                value
                    .headers_mut()
                    .insert("x-biliup-filmstrip-layout", header);
            }
            value
        }
        Err(error) => {
            tracing::warn!(%error, "读取剪辑素材失败");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct ImportQuery {
    name: String,
    capture: Option<String>,
    agent_capture: Option<String>,
    capture_source: Option<String>,
}
pub async fn import_media(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    Path(id): Path<i64>,
    Query(query): Query<ImportQuery>,
    caller: crate::server::api::access::Caller,
    body: axum::body::Body,
) -> Response {
    if let Some(capture) = query.agent_capture {
        return response(
            jobs.editor_agent
                .upload_screen(
                    id,
                    &capture,
                    &query.name,
                    query.capture_source.as_deref(),
                    &caller,
                    body,
                )
                .await,
        );
    }
    export_response(
        editor::imports::upload(
            &pool,
            jobs.root(),
            id,
            &query.name,
            query.capture.as_deref(),
            body,
        )
        .await,
    )
}
