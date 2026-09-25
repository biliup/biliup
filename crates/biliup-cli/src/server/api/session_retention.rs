//! `PATCH /v1/sessions/{id}`：「保留这场」，改场次的 `retain_until`。

use crate::server::errors::ApiError;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::workbench::retention;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Deserialize)]
pub struct SessionPatch {
    /// Unix 毫秒；`null` 取消保留。不带这个键是请求错误（不能和「取消保留」混为一谈）。
    #[serde(default, deserialize_with = "present")]
    retain_until: Option<Option<i64>>,
}

fn present<'de, D>(deserializer: D) -> Result<Option<Option<i64>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<i64>::deserialize(deserializer).map(Some)
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct SessionRetention {
    pub id: i64,
    pub retain_until: Option<i64>,
}

/// 在 `retain_until` 之前，这一场的分段在后处理 `rm`、边录边传投稿后删除等删除点一律推迟删除，
/// 磁盘水位兜底时也排在没被保留的分段之后；过期或取消后由清理任务在一分钟内删掉已到期的分段。
pub async fn patch_session(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
    Json(patch): Json<SessionPatch>,
) -> Result<Json<SessionRetention>, Rejection> {
    let Some(retain_until) = patch.retain_until else {
        return Err(reject(
            StatusCode::BAD_REQUEST,
            "缺少 retain_until（Unix 毫秒，null 为取消保留）",
        ));
    };
    if retain_until.is_some_and(|t| t < 0) {
        return Err(reject(StatusCode::BAD_REQUEST, "retain_until 不能是负数"));
    }
    let found = retention::set_session_retention(&pool, id, retain_until)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, session = id, "更新场次保留期失败");
            reject(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string())
        })?;
    if !found {
        return Err(reject(StatusCode::NOT_FOUND, "场次不存在"));
    }
    Ok(Json(SessionRetention { id, retain_until }))
}

type Rejection = (StatusCode, Json<ApiError>);

fn reject(status: StatusCode, message: &str) -> Rejection {
    (status, Json(ApiError::new(message.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::infrastructure::connection_pool::ConnectionManager;

    async fn call(pool: &ConnectionPool, id: i64, body: &str) -> Result<SessionRetention, u16> {
        let patch: SessionPatch = serde_json::from_str(body).unwrap();
        patch_session(State(pool.clone()), Path(id), Json(patch))
            .await
            .map(|Json(r)| r)
            .map_err(|(status, _)| status.as_u16())
    }

    #[tokio::test]
    async fn sets_and_clears_retain_until() {
        let dir = tempfile::tempdir().unwrap();
        let pool = ConnectionManager::new_pool(dir.path().join("d.sqlite3").to_str().unwrap())
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO stream_sessions (id, name, url, title, date, live_cover_path)
             VALUES (7, 'a', 'https://a', 't', '2026-09-24T00:00:00Z', '')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let stored = || async {
            sqlx::query_scalar::<_, Option<i64>>("SELECT retain_until FROM stream_sessions")
                .fetch_one(&pool)
                .await
                .unwrap()
        };

        assert_eq!(
            call(&pool, 7, r#"{"retain_until": 1790000000000}"#).await,
            Ok(SessionRetention {
                id: 7,
                retain_until: Some(1_790_000_000_000)
            })
        );
        assert_eq!(stored().await, Some(1_790_000_000_000));
        assert_eq!(
            call(&pool, 7, r#"{"retain_until": null}"#)
                .await
                .unwrap()
                .retain_until,
            None
        );
        assert_eq!(stored().await, None);
        assert_eq!(call(&pool, 7, "{}").await, Err(400));
        assert_eq!(call(&pool, 7, r#"{"retain_until": -1}"#).await, Err(400));
        assert_eq!(call(&pool, 8, r#"{"retain_until": null}"#).await, Err(404));
    }
}
