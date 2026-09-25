//! 切片接口：`/v1/sessions/{id}/clips[/{cid}]` 增删改查，`/v1/clips/{cid}` 查一个。
//!
//! 看列表要 `file.view`，增删改要 `clip.edit`，都由策略层按路由表判断。
//! 只按场次 id、切片 id 寻址，产物路径不出现在请求和响应里（响应只带文件名）。

use crate::server::api::access::Caller;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::clips::{
    self, Clip, ClipChanges, MAX_CLIP_MS, MAX_CLIPS_PER_SESSION, MAX_TITLE_CHARS, Mode, NewClip,
    State as ClipState, UpdateOutcome,
};
use crate::server::workbench::{recorder, store};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tracing::warn;

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
    pub created_by: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

fn view(clip: Clip) -> ClipView {
    ClipView {
        file_name: clip.file_name().map(str::to_string),
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
pub async fn list_clips(State(pool): State<ConnectionPool>, Path(id): Path<i64>) -> Response {
    match store::session(&pool, id).await {
        Ok(Some(_)) => {}
        Ok(None) => return session_not_found(),
        Err(e) => return internal(e),
    }
    match clips::list(&pool, id).await {
        Ok(list) => Json(ClipList {
            clips: list.into_iter().map(view).collect(),
        })
        .into_response(),
        Err(e) => internal(e),
    }
}

/// `GET /v1/clips/{cid}`
pub async fn get_clip(State(pool): State<ConnectionPool>, Path(cid): Path<i64>) -> Response {
    match clips::get(&pool, cid).await {
        Ok(Some(clip)) => Json(view(clip)).into_response(),
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

/// `POST /v1/sessions/{id}/clips` 的请求体：`in_ms` / `out_ms` 是场次时间。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateClip {
    pub in_ms: Option<i64>,
    pub out_ms: Option<i64>,
    #[serde(default)]
    pub title: String,
    pub marker_id: Option<i64>,
}

/// `POST /v1/sessions/{id}/clips`
pub async fn create_clip(
    caller: Caller,
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
    Json(body): Json<CreateClip>,
) -> Response {
    let title = match check_title(&body.title) {
        Ok(title) => title,
        Err(message) => return bad_request(message),
    };
    match store::session(&pool, id).await {
        Ok(Some(_)) => {}
        Ok(None) => return session_not_found(),
        Err(e) => return internal(e),
    }
    let now = recorder::now_ms();
    let (Some(in_ms), Some(out_ms)) = (body.in_ms, body.out_ms) else {
        return bad_request("要给 in_ms 和 out_ms");
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
    match clips::insert(&pool, id, &new).await {
        Ok(clip) => (StatusCode::CREATED, Json(view(clip))).into_response(),
        Err(e) => internal(e),
    }
}

/// `PATCH /v1/sessions/{id}/clips/{cid}` 的请求体：只改给出的字段。
/// 改了范围的话之前导出的文件作废，要重新导出。
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateClip {
    pub in_ms: Option<i64>,
    pub out_ms: Option<i64>,
    pub title: Option<String>,
}

/// `PATCH /v1/sessions/{id}/clips/{cid}`
pub async fn update_clip(
    State(pool): State<ConnectionPool>,
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
    let changes = ClipChanges {
        in_ms: body.in_ms,
        out_ms: body.out_ms,
        title,
    };
    match clips::update(&pool, id, cid, &changes, recorder::now_ms()).await {
        Ok(UpdateOutcome::Updated(clip)) => Json(view(*clip)).into_response(),
        Ok(UpdateOutcome::NotFound) => clip_not_found(),
        Ok(UpdateOutcome::Busy) => conflict("正在导出，等导出结束后再改范围"),
        Ok(UpdateOutcome::BadRange) => bad_request(format!(
            "出点要在入点之后，且一个切片最长 {} 小时",
            MAX_CLIP_MS / 3_600_000
        )),
        Err(e) => internal(e),
    }
}

/// `DELETE /v1/sessions/{id}/clips/{cid}`
pub async fn delete_clip(
    State(pool): State<ConnectionPool>,
    Path((id, cid)): Path<(i64, i64)>,
) -> Response {
    match clips::delete(&pool, id, cid).await {
        Ok(Some(_)) => StatusCode::NO_CONTENT.into_response(),
        Ok(None) => clip_not_found(),
        Err(e) => internal(e),
    }
}

#[cfg(test)]
mod tests;
