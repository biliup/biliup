//! 自动切片候选：`GET /v1/sessions/{id}/suggestions` 列表（`file.view`），
//! `POST …/suggestions/{sid}/accept` 接受（可带改过的入出点和标题，建切片草稿）、
//! `POST …/suggestions/{sid}/dismiss` 丢弃（都是 `clip.edit`，由策略层按路由表判断）。

use crate::server::api::access::Caller;
use crate::server::api::clips::{ClipView, check_range, check_title, view};
use crate::server::auto_clip::suggestions::{
    self, AcceptOutcome, Acceptance, DismissOutcome, Suggestion, SuggestionState,
};
use crate::server::errors::ApiError;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::clips::MAX_CLIPS_PER_SESSION;
use crate::server::workbench::clips::export::ClipExports;
use crate::server::workbench::recorder::now_ms;
use crate::server::workbench::store;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

type Rejection = (StatusCode, Json<ApiError>);

fn reject(status: StatusCode, message: impl Into<String>) -> Rejection {
    (status, Json(ApiError::new(message.into())))
}

fn internal(error: sqlx::Error) -> Rejection {
    tracing::error!(%error, "读写候选失败");
    reject(StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn not_found() -> Rejection {
    reject(StatusCode::NOT_FOUND, "候选不存在")
}

fn not_pending(suggestion: &Suggestion) -> Rejection {
    let what = match suggestion.state {
        SuggestionState::Accepted => "已经接受过了",
        SuggestionState::Dismissed => "已经丢弃了",
        SuggestionState::Expired => "已经过期了（72 小时没处理），重新生成候选后再选",
        SuggestionState::Pending => "还没处理",
    };
    reject(StatusCode::CONFLICT, format!("这个候选{what}"))
}

async fn require_session(pool: &ConnectionPool, id: i64) -> Result<(), Rejection> {
    match store::session(pool, id).await.map_err(internal)? {
        Some(_) => Ok(()),
        None => Err(reject(StatusCode::NOT_FOUND, "场次不存在")),
    }
}

#[derive(Debug, Serialize)]
pub struct SuggestionList {
    pub suggestions: Vec<Suggestion>,
}

/// `GET /v1/sessions/{id}/suggestions`
pub async fn list_suggestions(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
) -> Result<Json<SuggestionList>, Rejection> {
    require_session(&pool, id).await?;
    let suggestions = suggestions::list(&pool, id).await.map_err(internal)?;
    Ok(Json(SuggestionList { suggestions }))
}

/// `POST …/accept` 的请求体：都可以不给，不给就用候选自己的。
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AcceptSuggestion {
    pub in_ms: Option<i64>,
    pub out_ms: Option<i64>,
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Accepted {
    pub suggestion: Suggestion,
    pub clip: ClipView,
}

/// `POST /v1/sessions/{id}/suggestions/{sid}/accept`
pub async fn accept_suggestion(
    caller: Caller,
    State(pool): State<ConnectionPool>,
    State(exports): State<Arc<ClipExports>>,
    Path((id, sid)): Path<(i64, i64)>,
    body: Option<Json<AcceptSuggestion>>,
) -> Result<(StatusCode, Json<Accepted>), Rejection> {
    let body = body.map(|Json(body)| body).unwrap_or_default();
    require_session(&pool, id).await?;
    let current = suggestions::get(&pool, sid)
        .await
        .map_err(internal)?
        .filter(|s| s.session_id == id)
        .ok_or_else(not_found)?;
    let title = body
        .title
        .as_deref()
        .map(check_title)
        .transpose()
        .map_err(|message| reject(StatusCode::BAD_REQUEST, message))?;
    let in_ms = body.in_ms.unwrap_or(current.in_ms);
    let out_ms = body.out_ms.unwrap_or(current.out_ms);
    check_range(in_ms, out_ms).map_err(|message| reject(StatusCode::BAD_REQUEST, message))?;
    let acceptance = Acceptance::builder()
        .in_ms(in_ms)
        .out_ms(out_ms)
        .maybe_title(title)
        .maybe_created_by(caller.subject.user_id)
        .build();
    match suggestions::accept(&pool, id, sid, &acceptance, now_ms())
        .await
        .map_err(internal)?
    {
        AcceptOutcome::Accepted { suggestion, clip } => {
            info!(
                session = id,
                suggestion = sid,
                clip = clip.id,
                "自动切片：接受候选，建切片草稿"
            );
            Ok((
                StatusCode::CREATED,
                Json(Accepted {
                    suggestion: *suggestion,
                    clip: view(*clip, &exports),
                }),
            ))
        }
        AcceptOutcome::NotFound => Err(not_found()),
        AcceptOutcome::NotPending(suggestion) => Err(not_pending(&suggestion)),
        AcceptOutcome::TooManyClips => Err(reject(
            StatusCode::CONFLICT,
            format!("这一场的切片已达上限（{MAX_CLIPS_PER_SESSION} 个），先删掉一些再接受"),
        )),
    }
}

/// `POST /v1/sessions/{id}/suggestions/{sid}/dismiss`
pub async fn dismiss_suggestion(
    State(pool): State<ConnectionPool>,
    Path((id, sid)): Path<(i64, i64)>,
) -> Result<Json<Suggestion>, Rejection> {
    require_session(&pool, id).await?;
    match suggestions::dismiss(&pool, id, sid, now_ms())
        .await
        .map_err(internal)?
    {
        DismissOutcome::Dismissed(suggestion) => Ok(Json(*suggestion)),
        DismissOutcome::NotFound => Err(not_found()),
        DismissOutcome::NotPending(suggestion) => Err(not_pending(&suggestion)),
    }
}
