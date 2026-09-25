//! 切片接口：`/v1/sessions/{id}/clips[/{cid}]` 增删改查，`/v1/clips/{cid}` 查一个、
//! `/v1/clips/{cid}/export` 导出、`/v1/clips/{cid}/download` 下载。
//!
//! 看列表、下载要 `file.view`，增删改（含发布设置）、导出要 `clip.edit`，都由策略层按路由表判断。
//! 只按场次 id、切片 id 寻址，产物路径不出现在请求和响应里（响应只带文件名）。

use crate::server::api::access::Caller;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::clips::export::{ClipExports, DownloadError, Progress};
use crate::server::workbench::clips::publish::queue::{ClipPublisher, JobState};
use crate::server::workbench::clips::publish::{StudioOverride, cover_file};
use crate::server::workbench::clips::{
    self, Clip, ClipChanges, MAX_CLIP_MS, MAX_CLIPS_PER_SESSION, MAX_TITLE_CHARS, Mode, NewClip,
    State as ClipState, UpdateOutcome,
};
use crate::server::workbench::markers::{self, Timing};
use crate::server::workbench::{live, recorder, store};
use axum::Json;
use axum::body::Body;
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::Arc;
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::{debug, warn};

/// 「剪下刚才 N 秒」最长多少（毫秒）。
pub const MAX_LAST_MS: i64 = 600_000;

fn internal(error: impl std::fmt::Display) -> Response {
    warn!(%error, "切片接口出错");
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
}

fn bad_request(message: impl Into<String>) -> Response {
    (StatusCode::BAD_REQUEST, message.into()).into_response()
}

fn conflict(message: impl Into<String>) -> Response {
    (StatusCode::CONFLICT, message.into()).into_response()
}

fn session_not_found() -> Response {
    (StatusCode::NOT_FOUND, "场次不存在").into_response()
}

fn clip_not_found() -> Response {
    (StatusCode::NOT_FOUND, "切片不存在，可能已经被删掉了").into_response()
}

#[derive(Debug, Serialize)]
pub struct ClipView {
    pub id: i64,
    pub session_id: i64,
    pub marker_id: Option<i64>,
    pub in_ms: i64,
    pub out_ms: i64,
    /// 最近一次导出实际切在哪里（场次时间）；快速剪在关键帧上。
    pub cut_in_ms: Option<i64>,
    pub cut_out_ms: Option<i64>,
    pub mode: Option<Mode>,
    pub title: String,
    pub state: ClipState,
    /// 产物文件名（不含目录）。
    pub file_name: Option<String>,
    pub output_bytes: Option<i64>,
    /// 产物的媒体时长（毫秒）。
    pub duration_ms: Option<i64>,
    /// 导出失败的原因。
    pub error: Option<String>,
    /// 发布用的上传模板；`None` = 用主播绑定的模板。
    pub template_id: Option<i64>,
    /// 发布设置里覆盖模板的部分。
    pub studio_override: StudioOverride,
    /// 发布后的稿件号。
    pub archive_bvid: Option<String>,
    pub published_at: Option<i64>,
    pub created_by: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    /// 正在导出时的进度。
    pub progress: Option<Progress>,
}

fn view(clip: Clip, exports: &ClipExports) -> ClipView {
    let progress = (clip.state == ClipState::Exporting)
        .then(|| exports.progress(clip.id))
        .flatten();
    ClipView {
        file_name: clip.file_name().map(str::to_string),
        progress,
        id: clip.id,
        session_id: clip.session_id,
        marker_id: clip.marker_id,
        in_ms: clip.in_ms,
        out_ms: clip.out_ms,
        cut_in_ms: clip.cut_in_ms,
        cut_out_ms: clip.cut_out_ms,
        mode: clip.mode,
        title: clip.title,
        state: clip.state,
        output_bytes: clip.output_bytes,
        duration_ms: clip.duration_ms,
        error: clip.error,
        studio_override: StudioOverride::parse(clip.studio_override.as_deref()),
        template_id: clip.template_id,
        archive_bvid: clip.archive_bvid,
        published_at: clip.published_at,
        created_by: clip.created_by,
        created_at: clip.created_at,
        updated_at: clip.updated_at,
    }
}

#[derive(Debug, Serialize)]
pub struct ClipList {
    pub clips: Vec<ClipView>,
}

/// `GET /v1/sessions/{id}/clips`
pub async fn list_clips(
    State(pool): State<ConnectionPool>,
    State(exports): State<Arc<ClipExports>>,
    Path(id): Path<i64>,
) -> Response {
    match store::session(&pool, id).await {
        Ok(Some(_)) => {}
        Ok(None) => return session_not_found(),
        Err(e) => return internal(e),
    }
    match clips::list(&pool, id).await {
        Ok(list) => Json(ClipList {
            clips: list.into_iter().map(|c| view(c, &exports)).collect(),
        })
        .into_response(),
        Err(e) => internal(e),
    }
}

/// `GET /v1/clips/{cid}`
pub async fn get_clip(
    State(pool): State<ConnectionPool>,
    State(exports): State<Arc<ClipExports>>,
    Path(cid): Path<i64>,
) -> Response {
    match clips::get(&pool, cid).await {
        Ok(Some(clip)) => Json(view(clip, &exports)).into_response(),
        Ok(None) => clip_not_found(),
        Err(e) => internal(e),
    }
}

fn check_title(title: &str) -> Result<String, String> {
    let title = title.trim();
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(format!("切片标题最多 {MAX_TITLE_CHARS} 个字"));
    }
    if title.chars().any(char::is_control) {
        return Err("切片标题里不能有换行或控制字符".into());
    }
    Ok(title.to_string())
}

fn check_range(in_ms: i64, out_ms: i64) -> Result<(), String> {
    if in_ms < 0 {
        return Err("入点不能是负数".into());
    }
    if out_ms <= in_ms {
        return Err("出点要在入点之后".into());
    }
    if out_ms - in_ms > MAX_CLIP_MS {
        return Err(format!(
            "一个切片最长 {} 小时，把范围缩短一些",
            MAX_CLIP_MS / 3_600_000
        ));
    }
    Ok(())
}

/// `POST /v1/sessions/{id}/clips` 的请求体。
///
/// 按时间选段给 `in_ms` / `out_ms`（场次时间）；看直播时「剪下刚才 N 秒」给 `last_ms`，出点按
/// 「现在屏幕上的画面」换算（`client_now` / `pressed_at` / `latency_ms` 与打标记相同），要求这一场
/// 正在录。`export` 给了就建好之后立即导出。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateClip {
    pub in_ms: Option<i64>,
    pub out_ms: Option<i64>,
    pub last_ms: Option<i64>,
    pub client_now: Option<i64>,
    pub pressed_at: Option<i64>,
    pub latency_ms: Option<i64>,
    #[serde(default)]
    pub title: String,
    pub marker_id: Option<i64>,
    pub export: Option<Mode>,
}

/// `POST /v1/sessions/{id}/clips`
pub async fn create_clip(
    caller: Caller,
    State(pool): State<ConnectionPool>,
    State(exports): State<Arc<ClipExports>>,
    Path(id): Path<i64>,
    Json(body): Json<CreateClip>,
) -> Response {
    let title = match check_title(&body.title) {
        Ok(title) => title,
        Err(message) => return bad_request(message),
    };
    let session = match store::session(&pool, id).await {
        Ok(Some(session)) => session,
        Ok(None) => return session_not_found(),
        Err(e) => return internal(e),
    };
    let now = recorder::now_ms();
    let (in_ms, out_ms) = match (body.in_ms, body.out_ms, body.last_ms) {
        (Some(in_ms), Some(out_ms), None) => (in_ms, out_ms),
        (None, None, Some(last_ms)) => {
            if !(1_000..=MAX_LAST_MS).contains(&last_ms) {
                return bad_request(format!(
                    "last_ms 要在 1000 到 {MAX_LAST_MS} 毫秒（10 分钟）之间"
                ));
            }
            let timing = Timing {
                client_now: body.client_now,
                pressed_at: body.pressed_at,
                latency_ms: body.latency_ms,
            };
            match markers::watched_at_ms(id, now, timing) {
                Some(out_ms) => {
                    debug!(
                        session = id,
                        out_ms,
                        last_ms,
                        ?timing,
                        "按当前画面换算切片出点"
                    );
                    ((out_ms - last_ms).max(0), out_ms.max(1))
                }
                None if live::is_recording(id) || session.ended_at.is_none() => {
                    return conflict("这次录制还没开出分段，等画面开始写盘后再剪");
                }
                None => {
                    return conflict("这一场没有在录，不能按当前画面剪；请在剪辑台里选段");
                }
            }
        }
        _ => return bad_request("要么给 in_ms 和 out_ms，要么只给 last_ms"),
    };
    if let Err(message) = check_range(in_ms, out_ms) {
        return bad_request(message);
    }
    if let Some(marker_id) = body.marker_id {
        match sqlx::query_scalar::<_, i64>("SELECT session_id FROM markers WHERE id = ?")
            .bind(marker_id)
            .fetch_optional(&pool)
            .await
        {
            Ok(Some(session_id)) if session_id == id => {}
            Ok(_) => return bad_request("marker_id 不是这一场的标记"),
            Err(e) => return internal(e),
        }
    }
    match clips::count(&pool, id).await {
        Ok(n) if n >= MAX_CLIPS_PER_SESSION => {
            return conflict(format!(
                "这一场的切片已达上限（{MAX_CLIPS_PER_SESSION} 个），先删掉一些再剪"
            ));
        }
        Ok(_) => {}
        Err(e) => return internal(e),
    }
    let new = NewClip {
        marker_id: body.marker_id,
        in_ms,
        out_ms,
        title,
        created_by: caller.subject.user_id,
        created_at: now,
    };
    let mut clip = match clips::insert(&pool, id, &new).await {
        Ok(clip) => clip,
        Err(e) => return internal(e),
    };
    if let Some(mode) = body.export {
        match clips::begin_export(&pool, clip.id, mode, now).await {
            Ok(Some(exporting)) => {
                exports.start(exporting.clone());
                clip = exporting;
            }
            Ok(None) => {}
            Err(e) => return internal(e),
        }
    }
    (StatusCode::CREATED, Json(view(clip, &exports))).into_response()
}

/// 区分「没给」和「给了 null」。
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

/// `PATCH /v1/sessions/{id}/clips/{cid}` 的请求体：只改给出的字段。
/// 改了范围的话之前导出的文件作废，要重新导出。`template_id: null` = 改回主播绑定的模板，
/// `studio_override: null` = 清掉覆盖（切片封面文件保留，`cover` 不再指向它）。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateClip {
    pub in_ms: Option<i64>,
    pub out_ms: Option<i64>,
    pub title: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub template_id: Option<Option<i64>>,
    #[serde(default, deserialize_with = "present")]
    pub studio_override: Option<Option<StudioOverride>>,
}

/// `PATCH /v1/sessions/{id}/clips/{cid}`
pub async fn update_clip(
    State(pool): State<ConnectionPool>,
    State(exports): State<Arc<ClipExports>>,
    State(publisher): State<Arc<ClipPublisher>>,
    Path((id, cid)): Path<(i64, i64)>,
    Json(body): Json<UpdateClip>,
) -> Response {
    let title = match body.title.as_deref().map(check_title).transpose() {
        Ok(title) => title,
        Err(message) => return bad_request(message),
    };
    if body.in_ms.is_some_and(|v| v < 0) {
        return bad_request("入点不能是负数");
    }
    if let Some(Some(over)) = &body.studio_override
        && let Err(message) = over.validate()
    {
        return bad_request(message);
    }
    let changes = ClipChanges {
        in_ms: body.in_ms,
        out_ms: body.out_ms,
        title,
    };
    let before = match clips::get(&pool, cid).await {
        Ok(Some(clip)) if clip.session_id == id => clip,
        Ok(_) => return clip_not_found(),
        Err(e) => return internal(e),
    };
    let range_changes = body.in_ms.is_some_and(|v| v != before.in_ms)
        || body.out_ms.is_some_and(|v| v != before.out_ms);
    if range_changes
        && matches!(
            publisher.state_of(cid),
            Some(JobState::Queued | JobState::Running | JobState::Paused)
        )
    {
        return conflict("这个切片在发布队列里，等它发完或先移出队列再改范围");
    }
    let now = recorder::now_ms();
    match clips::update(&pool, id, cid, &changes, now).await {
        Ok(UpdateOutcome::Updated(mut clip)) => {
            if before.output_path.is_some() && clip.output_path.is_none() {
                exports.remove_outputs(id, cid).await;
            }
            if (clip.in_ms, clip.out_ms) != (before.in_ms, before.out_ms) {
                publisher.forget_upload(cid);
            }
            if body.template_id.is_some() || body.studio_override.is_some() {
                let template_id = body.template_id.unwrap_or(clip.template_id);
                let over = match &body.studio_override {
                    Some(over) => over.clone().unwrap_or_default(),
                    None => StudioOverride::parse(clip.studio_override.as_deref()),
                };
                match clips::set_publish_settings(
                    &pool,
                    cid,
                    template_id,
                    over.to_json().as_deref(),
                    now,
                )
                .await
                {
                    Ok(Some(saved)) => *clip = saved,
                    Ok(None) => return clip_not_found(),
                    Err(e) => return internal(e),
                }
            }
            Json(view(*clip, &exports)).into_response()
        }
        Ok(UpdateOutcome::NotFound) => clip_not_found(),
        Ok(UpdateOutcome::Busy) => conflict("正在导出，等导出结束后再改范围"),
        Ok(UpdateOutcome::BadRange) => bad_request(format!(
            "出点要在入点之后，且一个切片最长 {} 小时",
            MAX_CLIP_MS / 3_600_000
        )),
        Err(e) => internal(e),
    }
}

/// `DELETE /v1/sessions/{id}/clips/{cid}`：正在导出的先停掉，文件（含切片封面）一起删，撤销对录像的引用。
/// 在发布队列里的要先移出。
pub async fn delete_clip(
    State(pool): State<ConnectionPool>,
    State(exports): State<Arc<ClipExports>>,
    State(publisher): State<Arc<ClipPublisher>>,
    Path((id, cid)): Path<(i64, i64)>,
) -> Response {
    if publisher.state_of(cid).is_some() {
        return conflict("这个切片在发布队列里，先把它移出队列再删");
    }
    match clips::delete(&pool, id, cid).await {
        Ok(Some(_)) => {
            exports.cancel(cid);
            exports.remove_outputs(id, cid).await;
            let _ = tokio::fs::remove_file(cover_file(&exports.dir(id), cid)).await;
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(None) => clip_not_found(),
        Err(e) => internal(e),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportClip {
    pub mode: Mode,
}

/// `POST /v1/clips/{cid}/export`：开始导出（失败后重试也是它），返回 202 和导出中的切片。
pub async fn export_clip(
    State(pool): State<ConnectionPool>,
    State(exports): State<Arc<ClipExports>>,
    Path(cid): Path<i64>,
    Json(body): Json<ExportClip>,
) -> Response {
    match clips::begin_export(&pool, cid, body.mode, recorder::now_ms()).await {
        Ok(Some(clip)) => {
            exports.start(clip.clone());
            (StatusCode::ACCEPTED, Json(view(clip, &exports))).into_response()
        }
        Ok(None) => match clips::get(&pool, cid).await {
            Ok(None) => clip_not_found(),
            Ok(Some(clip)) if clip.state == ClipState::Exporting => {
                conflict("这个切片正在导出，等它结束")
            }
            Ok(Some(_)) => conflict("已发布或已放弃的切片不能再导出"),
            Err(e) => internal(e),
        },
        Err(e) => internal(e),
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadFormat {
    /// 产物本身（快速剪是源容器，精确剪是 MP4）。
    #[default]
    Source,
    /// MP4：产物不是 MP4 时用 ffmpeg 转封装（不转码）。
    Mp4,
}

#[derive(Debug, Default, Deserialize)]
pub struct DownloadQuery {
    #[serde(default)]
    pub format: DownloadFormat,
}

/// 下载时的文件名：标题（去掉文件名里不能用的字符）或 `切片-<id>`，加扩展名。
fn download_name(clip: &Clip, extension: &str) -> String {
    let title: String = clip
        .title
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .collect();
    let title = title.trim().trim_matches('.');
    if title.is_empty() {
        format!("切片-{}.{extension}", clip.id)
    } else {
        format!("{title}.{extension}")
    }
}

/// `GET /v1/clips/{cid}/download?format=source|mp4`
pub async fn download_clip(
    State(pool): State<ConnectionPool>,
    State(exports): State<Arc<ClipExports>>,
    Path(cid): Path<i64>,
    Query(query): Query<DownloadQuery>,
    request: Request<Body>,
) -> Response {
    let clip = match clips::get(&pool, cid).await {
        Ok(Some(clip)) => clip,
        Ok(None) => return clip_not_found(),
        Err(e) => return internal(e),
    };
    let path = match query.format {
        DownloadFormat::Source => match clip.output_path.as_deref() {
            Some(path) if matches!(clip.state, ClipState::Ready | ClipState::Published) => {
                std::path::PathBuf::from(path)
            }
            _ => return conflict(DownloadError::NotReady.to_string()),
        },
        DownloadFormat::Mp4 => match exports.mp4(&clip).await {
            Ok(path) => path,
            Err(e) => return conflict(e.to_string()),
        },
    };
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return conflict(DownloadError::Missing.to_string());
    }
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin")
        .to_ascii_lowercase();
    let name = download_name(&clip, &extension);
    let disposition = format!(
        "attachment; filename=\"clip-{cid}.{extension}\"; filename*=UTF-8''{}",
        urlencoding::encode(&name)
    );
    let mut response = match ServeFile::new(&path).oneshot(request).await {
        Ok(response) => response.map(Body::new),
        Err(e) => return internal(e),
    };
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, value);
    }
    response
}

#[cfg(test)]
mod tests;
