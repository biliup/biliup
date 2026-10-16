//! Unified, read-only upload observations; access is registered as `file.view`.
use crate::server::infrastructure::service_register::ServiceRegister;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

#[derive(Default, Deserialize)]
pub struct ListQuery {
    pub source: Option<String>,
    pub state: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
pub async fn list(
    State(services): State<ServiceRegister>,
    Query(query): Query<ListQuery>,
) -> Response {
    Json(
        services
            .upload_observations
            .list(
                query.source.as_deref(),
                query.state.as_deref(),
                query.limit.unwrap_or(50).clamp(1, 200),
                query.offset.unwrap_or(0),
            )
            .await,
    )
    .into_response()
}
pub async fn detail(State(services): State<ServiceRegister>, Path(id): Path<String>) -> Response {
    match services.upload_observations.detail(&id).await {
        Ok(Some(task)) => Json(task).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "上传任务不存在").into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "读取上传任务历史失败").into_response(),
    }
}
