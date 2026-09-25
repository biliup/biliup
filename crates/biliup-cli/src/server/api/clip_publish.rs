//! 切片发布接口。
//!
//! - `POST /v1/clips/{cid}/publish`：需要时先导出，再上传、投稿（抢发一条龙）；
//! - `POST /v1/publish-jobs`：勾选的几个切片集中发布，每个切片一个稿件，或合成一个多 P 稿件；
//! - `POST /v1/publish-jobs/preview`：按发布设置算出最终的标题、简介、标签等（不排队）；
//! - `GET /v1/publish-jobs`、`POST /v1/publish-jobs/resume`、`POST /v1/publish-jobs/{jid}/retry`、
//!   `DELETE /v1/publish-jobs/{jid}`：看队列、601 暂停后继续、重试、移出队列；
//! - `GET /v1/sessions/{id}/thumb?t=`：取一帧 JPEG；`PUT / GET / DELETE /v1/clips/{cid}/cover`：切片封面
//!   （取帧、直播间封面或上传的图片）。
//!
//! 发布、重试、继续要 `upload.submit`，看队列、取帧、看封面要 `file.view`，改封面要 `clip.edit`，
//! 都由策略层按路由表判断。

use crate::server::api::access::Caller;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::clips::export::ClipExports;
use crate::server::workbench::clips::publish::queue::{
    self, ActionError, ClipPublisher, EnqueueError, JobView, QueueView, Settings,
};
use crate::server::workbench::clips::publish::{
    Cover, MAX_PARTS, Rendered, StudioOverride, cover_file,
};
use crate::server::workbench::clips::thumb::{self, ThumbError};
use crate::server::workbench::clips::{self, Clip, Mode, State as ClipState};
use crate::server::workbench::recorder::now_ms;
use crate::server::workbench::store;
use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use biliup::client::StatelessClient;
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::Arc;
use tracing::warn;

/// 一次最多发布多少个切片。
pub const MAX_BATCH: usize = 50;

fn internal(error: impl std::fmt::Display) -> Response {
    warn!(%error, "切片发布接口出错");
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
}

fn bad_request(message: impl Into<String>) -> Response {
    (StatusCode::BAD_REQUEST, message.into()).into_response()
}

fn conflict(message: impl Into<String>) -> Response {
    (StatusCode::CONFLICT, message.into()).into_response()
}

/// 辅助函数里的拒绝：状态码和一句给人看的话。
type Rejection = (StatusCode, String);

fn failed(error: impl std::fmt::Display) -> Rejection {
    warn!(%error, "切片发布接口出错");
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn clip_not_found() -> Response {
    (StatusCode::NOT_FOUND, "切片不存在，可能已经被删掉了").into_response()
}

/// 区分「没给」和「给了 null」。
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

/// 发布设置：给了就用给的（`template_id: null` = 用主播绑定的模板），没给就用切片存着的。
#[derive(Debug, Default)]
pub struct PublishSettings {
    pub template_id: Option<Option<i64>>,
    pub studio_override: Option<StudioOverride>,
}

impl PublishSettings {
    fn validate(&self) -> Result<(), String> {
        match &self.studio_override {
            Some(over) => over.validate(),
            None => Ok(()),
        }
    }

    /// 与切片存着的设置合起来。
    fn for_clip(&self, clip: &Clip) -> Settings {
        Settings {
            template_id: self.template_id.unwrap_or(clip.template_id),
            over: self
                .studio_override
                .clone()
                .unwrap_or_else(|| StudioOverride::parse(clip.studio_override.as_deref())),
        }
    }
}

/// `POST /v1/clips/{cid}/publish` 的请求体。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishClip {
    #[serde(default, deserialize_with = "present")]
    pub template_id: Option<Option<i64>>,
    #[serde(default)]
    pub studio_override: Option<StudioOverride>,
    /// 还没导出时怎么导出；默认快速剪。
    #[serde(default)]
    pub mode: Option<Mode>,
}

impl PublishClip {
    fn settings(&self) -> PublishSettings {
        PublishSettings {
            template_id: self.template_id,
            studio_override: self.studio_override.clone(),
        }
    }
}

/// `POST /v1/publish-jobs`、`/v1/publish-jobs/preview` 的请求体。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishBatch {
    pub clip_ids: Vec<i64>,
    /// 合成一个多 P 稿件；默认每个切片一个稿件。
    #[serde(default)]
    pub combine: bool,
    #[serde(default, deserialize_with = "present")]
    pub template_id: Option<Option<i64>>,
    #[serde(default)]
    pub studio_override: Option<StudioOverride>,
    #[serde(default)]
    pub mode: Option<Mode>,
}

impl PublishBatch {
    fn settings(&self) -> PublishSettings {
        PublishSettings {
            template_id: self.template_id,
            studio_override: self.studio_override.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Jobs {
    pub jobs: Vec<JobView>,
}

/// 读出切片并检查：都在、属于同一场、没发布过。按时间排序。
async fn load_clips(pool: &ConnectionPool, ids: &[i64]) -> Result<Vec<Clip>, Rejection> {
    if ids.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "没有选切片".into()));
    }
    let mut list: Vec<Clip> = Vec::with_capacity(ids.len());
    for &id in ids {
        if list.iter().any(|c| c.id == id) {
            continue;
        }
        match clips::get(pool, id).await {
            Ok(Some(clip)) => list.push(clip),
            Ok(None) => {
                return Err((StatusCode::NOT_FOUND, format!("切片 #{id} 不存在")));
            }
            Err(e) => return Err(failed(e)),
        }
    }
    list.sort_by_key(|c| (c.session_id, c.in_ms, c.id));
    Ok(list)
}

/// 每个稿件一组（每个切片一个稿件时一个切片一组）。
fn groups(clips: Vec<Clip>, combine: bool) -> Vec<Vec<Clip>> {
    if combine {
        vec![clips]
    } else {
        clips.into_iter().map(|c| vec![c]).collect()
    }
}

fn check_batch(body: &PublishBatch, clips: &[Clip]) -> Result<(), Rejection> {
    if let Err(message) = body.settings().validate() {
        return Err((StatusCode::BAD_REQUEST, message));
    }
    if clips.len() > MAX_BATCH {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("一次最多发布 {MAX_BATCH} 个切片"),
        ));
    }
    if body.combine {
        if clips.len() > MAX_PARTS {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("一个稿件最多 {MAX_PARTS} 个分 P"),
            ));
        }
        if clips.windows(2).any(|w| w[0].session_id != w[1].session_id) {
            return Err((
                StatusCode::BAD_REQUEST,
                "合成一个稿件的切片要来自同一场直播".into(),
            ));
        }
    } else if clips.len() > 1 && body.settings().studio_override.is_some() {
        return Err((
            StatusCode::BAD_REQUEST,
            "每个切片一个稿件时，标题等按各个切片自己的发布设置；要统一填写请合成一个稿件".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct PreviewArchive {
    pub clip_ids: Vec<i64>,
    pub template_id: Option<i64>,
    /// 算出来的稿件字段；模板找不到时为 `None`，看 `problem`。
    pub rendered: Option<Rendered>,
    /// 发不了的原因（没有模板、没有标签等）。
    pub problem: Option<String>,
    pub cover: Option<Cover>,
}

#[derive(Debug, Serialize)]
pub struct Preview {
    pub archives: Vec<PreviewArchive>,
}

async fn preview_group(
    pool: &ConnectionPool,
    exports: &ClipExports,
    group: &[Clip],
    body: &PublishBatch,
) -> PreviewArchive {
    let settings = body.settings().for_clip(&group[0]);
    let clip_ids = group.iter().map(|c| c.id).collect();
    let session_id = group[0].session_id;
    match queue::archive(pool, exports, session_id, group, &settings).await {
        Ok(archive) => PreviewArchive {
            clip_ids,
            template_id: Some(archive.template.id),
            problem: archive.problem(),
            rendered: Some(archive.render()),
            cover: settings.over.cover.clone(),
        },
        Err(problem) => PreviewArchive {
            clip_ids,
            template_id: settings.template_id,
            rendered: None,
            problem: Some(problem),
            cover: settings.over.cover.clone(),
        },
    }
}

/// `POST /v1/publish-jobs/preview`
pub async fn preview_publish(
    State(pool): State<ConnectionPool>,
    State(exports): State<Arc<ClipExports>>,
    Json(body): Json<PublishBatch>,
) -> Response {
    let clips = match load_clips(&pool, &body.clip_ids).await {
        Ok(clips) => clips,
        Err(rejection) => return rejection.into_response(),
    };
    if let Err(rejection) = check_batch(&body, &clips) {
        return rejection.into_response();
    }
    let mut archives = Vec::new();
    for group in groups(clips, body.combine) {
        archives.push(preview_group(&pool, &exports, &group, &body).await);
    }
    Json(Preview { archives }).into_response()
}

async fn enqueue(
    caller: &Caller,
    pool: &ConnectionPool,
    exports: &ClipExports,
    publisher: &ClipPublisher,
    body: &PublishBatch,
    clips: Vec<Clip>,
) -> Response {
    if let Err(rejection) = check_batch(body, &clips) {
        return rejection.into_response();
    }
    if let Some(clip) = clips.iter().find(|c| c.state == ClipState::Published) {
        return conflict(format!(
            "切片 #{} 已经发布过了（{}）",
            clip.id,
            clip.archive_bvid.as_deref().unwrap_or("稿件号未知")
        ));
    }
    let groups = groups(clips, body.combine);
    // 先全部检查一遍（模板、标签、封面），有一个不行就都不排
    for group in &groups {
        let settings = body.settings().for_clip(&group[0]);
        let problem =
            match queue::archive(pool, exports, group[0].session_id, group, &settings).await {
                Ok(archive) => archive.problem(),
                Err(problem) => Some(problem),
            };
        if let Some(problem) = problem {
            let which = if groups.len() > 1 {
                format!("切片 #{}：", group[0].id)
            } else {
                String::new()
            };
            return conflict(format!("{which}{problem}"));
        }
    }
    let mode = body.mode.unwrap_or(Mode::Quick);
    let mut jobs = Vec::new();
    for group in &groups {
        let settings = body.settings().for_clip(&group[0]);
        match publisher.enqueue(
            group[0].session_id,
            group,
            settings,
            mode,
            caller.subject.user_id,
        ) {
            Ok(job) => jobs.push(job),
            Err(EnqueueError::Invalid(m)) => return bad_request(m),
            Err(EnqueueError::Conflict(m)) if jobs.is_empty() => return conflict(m),
            Err(EnqueueError::Conflict(m)) => {
                warn!(message = %m, "集中发布时有切片没排进队列");
            }
        }
    }
    (StatusCode::ACCEPTED, Json(Jobs { jobs })).into_response()
}

/// `POST /v1/clips/{cid}/publish`：给了发布设置就先存到切片上，再排进发布队列；返回 202。
pub async fn publish_clip(
    caller: Caller,
    State(pool): State<ConnectionPool>,
    State(exports): State<Arc<ClipExports>>,
    State(publisher): State<Arc<ClipPublisher>>,
    Path(cid): Path<i64>,
    Json(body): Json<PublishClip>,
) -> Response {
    if let Err(message) = body.settings().validate() {
        return bad_request(message);
    }
    let mut clip = match clips::get(&pool, cid).await {
        Ok(Some(clip)) => clip,
        Ok(None) => return clip_not_found(),
        Err(e) => return internal(e),
    };
    if body.settings().template_id.is_some() || body.settings().studio_override.is_some() {
        let settings = body.settings().for_clip(&clip);
        match clips::set_publish_settings(
            &pool,
            cid,
            settings.template_id,
            settings.over.to_json().as_deref(),
            now_ms(),
        )
        .await
        {
            Ok(Some(saved)) => clip = saved,
            Ok(None) => return clip_not_found(),
            Err(e) => return internal(e),
        }
    }
    let batch = PublishBatch {
        clip_ids: vec![cid],
        mode: body.mode,
        ..PublishBatch::default()
    };
    enqueue(&caller, &pool, &exports, &publisher, &batch, vec![clip]).await
}

/// `POST /v1/publish-jobs`
pub async fn publish_batch(
    caller: Caller,
    State(pool): State<ConnectionPool>,
    State(exports): State<Arc<ClipExports>>,
    State(publisher): State<Arc<ClipPublisher>>,
    Json(body): Json<PublishBatch>,
) -> Response {
    let clips = match load_clips(&pool, &body.clip_ids).await {
        Ok(clips) => clips,
        Err(rejection) => return rejection.into_response(),
    };
    enqueue(&caller, &pool, &exports, &publisher, &body, clips).await
}

#[derive(Debug, Default, Deserialize)]
pub struct JobsQuery {
    /// 只看这一场的任务。
    pub session: Option<i64>,
}

/// `GET /v1/publish-jobs?session=`
pub async fn list_publish_jobs(
    State(publisher): State<Arc<ClipPublisher>>,
    Query(query): Query<JobsQuery>,
) -> Json<QueueView> {
    Json(publisher.view(query.session))
}

/// `POST /v1/publish-jobs/resume`：601 暂停后继续。
pub async fn resume_publish(State(publisher): State<Arc<ClipPublisher>>) -> Json<QueueView> {
    publisher.resume();
    Json(publisher.view(None))
}

fn action_error(e: ActionError) -> Response {
    match e {
        ActionError::NotFound => (
            StatusCode::NOT_FOUND,
            "发布任务不存在（服务重启后队列会清空）",
        )
            .into_response(),
        ActionError::Conflict(m) => conflict(m),
    }
}

/// `POST /v1/publish-jobs/{jid}/retry`
pub async fn retry_publish(
    State(publisher): State<Arc<ClipPublisher>>,
    Path(jid): Path<u64>,
) -> Response {
    match publisher.retry(jid) {
        Ok(job) => (StatusCode::ACCEPTED, Json(job)).into_response(),
        Err(e) => action_error(e),
    }
}

/// `DELETE /v1/publish-jobs/{jid}`
pub async fn remove_publish(
    State(publisher): State<Arc<ClipPublisher>>,
    Path(jid): Path<u64>,
) -> Response {
    match publisher.remove(jid) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => action_error(e),
    }
}

#[derive(Debug, Deserialize)]
pub struct ThumbQuery {
    pub t: i64,
    #[serde(default)]
    pub w: Option<u32>,
}

fn jpeg(bytes: Vec<u8>) -> Response {
    (
        [
            (header::CONTENT_TYPE, HeaderValue::from_static("image/jpeg")),
            (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
        ],
        bytes,
    )
        .into_response()
}

fn thumb_error(e: ThumbError) -> Response {
    match e {
        ThumbError::NoFfmpeg(m) => (StatusCode::SERVICE_UNAVAILABLE, m).into_response(),
        ThumbError::Unavailable(m) => conflict(m),
        ThumbError::Failed(m) => (StatusCode::UNPROCESSABLE_ENTITY, m).into_response(),
    }
}

/// `GET /v1/sessions/{id}/thumb?t=<场次毫秒>&w=<宽>`：那一帧的 JPEG。
pub async fn get_session_thumb(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
    Query(query): Query<ThumbQuery>,
) -> Response {
    match store::session(&pool, id).await {
        Ok(Some(_)) => {}
        Ok(None) => return (StatusCode::NOT_FOUND, "场次不存在").into_response(),
        Err(e) => return internal(e),
    }
    match thumb::frame(&pool, id, query.t, query.w.unwrap_or(480)).await {
        Ok(bytes) => jpeg(bytes),
        Err(e) => thumb_error(e),
    }
}

/// 图片类型（按文件头认），不是图片时为 `None`。
fn image_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverRequest {
    /// 从录像取这一帧（场次毫秒）。
    #[serde(default)]
    pub t: Option<i64>,
    /// 用开播时记下的直播间封面。
    #[serde(default)]
    pub live: bool,
}

async fn write_cover(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        tokio::fs::create_dir_all(dir).await?;
    }
    let part = path.with_extension("jpg.part");
    tokio::fs::write(&part, bytes).await?;
    tokio::fs::rename(&part, path).await
}

async fn live_cover(
    pool: &ConnectionPool,
    client: &StatelessClient,
    session_id: i64,
) -> Result<Vec<u8>, Rejection> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT live_cover_path, url FROM stream_sessions WHERE id = ?")
            .bind(session_id)
            .fetch_optional(pool)
            .await
            .map_err(failed)?;
    let Some((url, room)) = row.filter(|(url, _)| url.starts_with("http")) else {
        return Err((
            StatusCode::CONFLICT,
            "这一场没有记下直播间封面，改用取帧或上传图片".into(),
        ));
    };
    let fetch = async {
        let response = client
            .client
            .get(&url)
            .header(header::REFERER, room)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await?
            .error_for_status()?;
        response.bytes().await
    };
    match fetch.await {
        Ok(bytes) if bytes.len() <= thumb::MAX_JPEG_BYTES && image_type(&bytes).is_some() => {
            Ok(bytes.to_vec())
        }
        Ok(_) => Err((
            StatusCode::CONFLICT,
            "直播间封面下载下来不是图片或太大，改用取帧或上传图片".into(),
        )),
        Err(e) => Err((
            StatusCode::CONFLICT,
            format!(
                "直播间封面下载失败（{}），改用取帧或上传图片",
                e.without_url()
            ),
        )),
    }
}

/// `PUT /v1/clips/{cid}/cover`：JSON `{"t": 毫秒}` 取帧、`{"live": true}` 用直播间封面，
/// 或者直接上传图片（`Content-Type: image/jpeg|png|webp`，最大 5 MB）。存好后切片的发布设置改用它。
pub async fn put_clip_cover(
    State(pool): State<ConnectionPool>,
    State(exports): State<Arc<ClipExports>>,
    State(client): State<StatelessClient>,
    Path(cid): Path<i64>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let clip = match clips::get(&pool, cid).await {
        Ok(Some(clip)) => clip,
        Ok(None) => return clip_not_found(),
        Err(e) => return internal(e),
    };
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let (bytes, cover) = if content_type.starts_with("application/json") {
        let request: CoverRequest = match serde_json::from_slice(&body) {
            Ok(r) => r,
            Err(e) => return bad_request(format!("请求体不对：{e}")),
        };
        match (request.t, request.live) {
            (Some(t), false) => {
                match thumb::frame(&pool, clip.session_id, t, thumb::MAX_WIDTH).await {
                    Ok(bytes) => (bytes, Cover::Frame { t_ms: t.max(0) }),
                    Err(e) => return thumb_error(e),
                }
            }
            (None, true) => match live_cover(&pool, &client, clip.session_id).await {
                Ok(bytes) => (bytes, Cover::Live),
                Err(rejection) => return rejection.into_response(),
            },
            _ => return bad_request("要么给 t（取帧），要么给 live: true"),
        }
    } else {
        if body.len() > thumb::MAX_JPEG_BYTES {
            return (StatusCode::PAYLOAD_TOO_LARGE, "封面图片最大 5 MB").into_response();
        }
        if image_type(&body).is_none() {
            return (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "只支持 JPEG、PNG、WebP 图片",
            )
                .into_response();
        }
        (body.to_vec(), Cover::Upload)
    };
    let path = cover_file(&exports.dir(clip.session_id), cid);
    if let Err(e) = write_cover(&path, &bytes).await {
        return internal(format!("保存封面失败：{e}"));
    }
    let mut over = StudioOverride::parse(clip.studio_override.as_deref());
    over.cover = Some(cover);
    match clips::set_publish_settings(
        &pool,
        cid,
        clip.template_id,
        over.to_json().as_deref(),
        now_ms(),
    )
    .await
    {
        Ok(Some(_)) => Json(over).into_response(),
        Ok(None) => clip_not_found(),
        Err(e) => internal(e),
    }
}

/// `GET /v1/clips/{cid}/cover`
pub async fn get_clip_cover(
    State(pool): State<ConnectionPool>,
    State(exports): State<Arc<ClipExports>>,
    Path(cid): Path<i64>,
) -> Response {
    let clip = match clips::get(&pool, cid).await {
        Ok(Some(clip)) => clip,
        Ok(None) => return clip_not_found(),
        Err(e) => return internal(e),
    };
    let path = cover_file(&exports.dir(clip.session_id), cid);
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let kind = image_type(&bytes).unwrap_or("application/octet-stream");
            (
                [
                    (header::CONTENT_TYPE, HeaderValue::from_static(kind)),
                    (header::CACHE_CONTROL, HeaderValue::from_static("no-store")),
                ],
                bytes,
            )
                .into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "这个切片没有自己的封面").into_response(),
    }
}

/// `DELETE /v1/clips/{cid}/cover`：删掉切片封面，改回模板的封面。
pub async fn delete_clip_cover(
    State(pool): State<ConnectionPool>,
    State(exports): State<Arc<ClipExports>>,
    Path(cid): Path<i64>,
) -> Response {
    let clip = match clips::get(&pool, cid).await {
        Ok(Some(clip)) => clip,
        Ok(None) => return clip_not_found(),
        Err(e) => return internal(e),
    };
    let _ = tokio::fs::remove_file(cover_file(&exports.dir(clip.session_id), cid)).await;
    let mut over = StudioOverride::parse(clip.studio_override.as_deref());
    over.cover = None;
    match clips::set_publish_settings(
        &pool,
        cid,
        clip.template_id,
        over.to_json().as_deref(),
        now_ms(),
    )
    .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => internal(e),
    }
}

#[cfg(test)]
mod tests;
