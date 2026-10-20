use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::session_groups::{self, MergeError};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

fn error(error: MergeError) -> Response {
    match error {
        MergeError::Invalid(message) => (StatusCode::BAD_REQUEST, message).into_response(),
        MergeError::NotFound => (StatusCode::NOT_FOUND, "候选不存在或已处理").into_response(),
        MergeError::Db(error) => {
            (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
        }
    }
}

pub async fn list_candidates(State(pool): State<ConnectionPool>) -> Response {
    match session_groups::candidates(&pool).await {
        Ok(items) => Json(serde_json::json!({"items":items})).into_response(),
        Err(e) => error(e.into()),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateGroup {
    pub source_session_ids: Vec<i64>,
}

pub async fn create_group(
    State(pool): State<ConnectionPool>,
    Json(body): Json<CreateGroup>,
) -> Response {
    match session_groups::create(&pool, &body.source_session_ids).await {
        Ok(id) => (StatusCode::CREATED, Json(serde_json::json!({"id":id}))).into_response(),
        Err(e) => error(e),
    }
}

pub async fn confirm_candidate(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
) -> Response {
    match session_groups::confirm(&pool, id).await {
        Ok(id) => Json(serde_json::json!({"id":id})).into_response(),
        Err(e) => error(e),
    }
}

pub async fn dismiss_candidate(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
) -> Response {
    match session_groups::dismiss(&pool, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => error(MergeError::NotFound),
        Err(e) => error(e.into()),
    }
}
