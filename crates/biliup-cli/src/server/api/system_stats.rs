//! 控制台首页的系统状态：`GET /v1/system-stats?since=<Unix 毫秒>`。
//!
//! 采样与口径见 [`crate::server::common::system_stats`]。不带 `since` 返回服务端保留的全部历史
//! （最近 5 分钟），带上则只返回更新的采样，前端每次轮询只拿增量。
//! 与 `/v1/status`、`/v1/tools` 一样，登录即可访问。

use crate::server::common::system_stats::SystemMonitor;
use axum::Json;
use axum::extract::{Query, State};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct SystemStatsQuery {
    /// 只要比这个时刻（Unix 毫秒）新的采样
    since: Option<i64>,
}

/// `GET /v1/system-stats`
pub async fn get_system_stats(
    State(monitor): State<Arc<SystemMonitor>>,
    Query(query): Query<SystemStatsQuery>,
) -> Response {
    let mut response = Json(monitor.snapshot(query.since)).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use serde_json::Value;
    use tower::ServiceExt;

    async fn fetch(app: &Router, uri: &str) -> (StatusCode, Option<HeaderValue>, Value) {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let cache = response.headers().get(header::CACHE_CONTROL).cloned();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json = serde_json::from_slice(&body).unwrap_or(Value::Null);
        (status, cache, json)
    }

    /// 走真实的路由与查询参数解析：首次拉全部历史，之后按 `since` 只拿增量，禁止缓存
    #[tokio::test]
    async fn the_endpoint_serves_history_then_increments() {
        let monitor = SystemMonitor::spawn();
        let app = Router::new()
            .route("/v1/system-stats", get(get_system_stats))
            .with_state(monitor.clone());

        let (status, cache, first) = fetch(&app, "/v1/system-stats").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(cache.unwrap(), "no-store");
        for key in [
            "ts",
            "interval_ms",
            "history_ms",
            "cpu",
            "memory",
            "disk",
            "interfaces",
            "samples",
        ] {
            assert!(first.get(key).is_some(), "响应里应有 {key}");
        }
        assert_eq!(first["interval_ms"], 2_000);

        // 未来的时刻之后不会有采样
        let future = first["ts"].as_i64().unwrap() + 60_000;
        let (_, _, later) = fetch(&app, &format!("/v1/system-stats?since={future}")).await;
        assert_eq!(later["samples"].as_array().unwrap().len(), 0);

        let (status, _, _) = fetch(&app, "/v1/system-stats?since=abc").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}
