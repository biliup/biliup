//! Persistent service activity, newest first across days and service restarts.
use crate::server::infrastructure::connection_pool::ConnectionPool;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ListQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ActivityEvent {
    pub id: i64,
    pub kind: String,
    /// Unix milliseconds; clients display both date and time.
    pub ts: i64,
    pub text: String,
    pub sub: Option<String>,
    pub session_id: Option<i64>,
    pub entity_id: Option<i64>,
}

async fn recent(pool: &ConnectionPool, limit: i64) -> sqlx::Result<Vec<ActivityEvent>> {
    sqlx::query("SELECT id,kind,ts,text,sub,session_id,entity_id FROM activity_events ORDER BY ts DESC,id DESC LIMIT ?")
        .bind(limit).fetch_all(pool).await?
        .iter().map(|row| Ok(ActivityEvent {
            id: row.try_get("id")?, kind: row.try_get("kind")?, ts: row.try_get("ts")?,
            text: row.try_get("text")?, sub: row.try_get("sub")?,
            session_id: row.try_get("session_id")?, entity_id: row.try_get("entity_id")?,
        })).collect()
}

pub async fn list(State(pool): State<ConnectionPool>, Query(query): Query<ListQuery>) -> Response {
    let limit = query.limit.unwrap_or(12);
    if !(1..=100).contains(&limit) {
        return (StatusCode::BAD_REQUEST, "事件数量须在 1–100 之间").into_response();
    }
    match recent(&pool, limit).await {
        Ok(events) => Json(serde_json::json!({ "events": events })).into_response(),
        Err(error) => {
            tracing::warn!(%error, "读取持久活动事件失败");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "读取最近事件失败，请查看服务日志",
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests;
