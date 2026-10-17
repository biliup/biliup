use crate::server::infrastructure::service_register::ServiceRegister;
use crate::server::services::douyu_keeper::KeeperError;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use biliup::downloader::live::Douyu;
use serde::{Deserialize, Serialize};

pub fn router() -> Router<ServiceRegister> {
    Router::new()
        .route("/v1/douyu/validate-cookie", post(validate_cookie))
        .route("/v1/douyu/auth/status", get(auth_status))
        .route("/v1/douyu/auth/refresh", post(refresh_auth))
}

#[derive(Deserialize, Default)]
struct AuthScope {
    streamer_id: Option<i64>,
}

fn keeper_error(error: KeeperError) -> Response {
    let status = match error {
        KeeperError::NotFound => StatusCode::NOT_FOUND,
        KeeperError::InvalidCookie => StatusCode::BAD_REQUEST,
        KeeperError::Database(_) => StatusCode::SERVICE_UNAVAILABLE,
    };
    // Display is a fixed safe message; never serialize the database source error.
    (
        status,
        Json(serde_json::json!({"message": error.to_string()})),
    )
        .into_response()
}

async fn auth_status(
    State(services): State<ServiceRegister>,
    Query(scope): Query<AuthScope>,
) -> Response {
    match services.douyu_keeper.status(scope.streamer_id).await {
        Ok(status) => ([(header::CACHE_CONTROL, "no-store")], Json(status)).into_response(),
        Err(error) => keeper_error(error),
    }
}

async fn refresh_auth(
    State(services): State<ServiceRegister>,
    Json(scope): Json<AuthScope>,
) -> Response {
    match services.douyu_keeper.refresh(scope.streamer_id).await {
        Ok(status) => ([(header::CACHE_CONTROL, "no-store")], Json(status)).into_response(),
        Err(error) => keeper_error(error),
    }
}

#[derive(Deserialize)]
struct ValidateCookieRequest {
    cookie: String,
}

#[derive(Serialize)]
struct ValidateCookieResponse {
    valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

/// 验证斗鱼Cookie是否有效
async fn validate_cookie(
    State(service_register): State<ServiceRegister>,
    Json(payload): Json<ValidateCookieRequest>,
) -> impl IntoResponse {
    // 基本验证：检查cookie是否为空
    if payload.cookie.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ValidateCookieResponse {
                valid: false,
                message: Some("Cookie不能为空".to_string()),
            }),
        )
            .into_response();
    }

    // 获取HTTP客户端 - ServiceRegister的client字段是public的
    let client = &service_register.client.client;

    // 调用实际验证逻辑
    match Douyu::validate_cookie(&payload.cookie, client).await {
        Ok(true) => (
            StatusCode::OK,
            Json(ValidateCookieResponse {
                valid: true,
                message: Some("Cookie有效".to_string()),
            }),
        )
            .into_response(),
        Ok(false) => (
            StatusCode::OK,
            Json(ValidateCookieResponse {
                valid: false,
                message: Some("Cookie无效或已过期".to_string()),
            }),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ValidateCookieResponse {
                valid: false,
                message: Some(format!("验证失败: {}", err)),
            }),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::{
        config::Config, core::download_manager::DownloadManager,
        infrastructure::connection_pool::ConnectionManager,
    };
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use std::sync::{Arc, RwLock};
    use tower::ServiceExt;
    use tracing_subscriber::{EnvFilter, reload};

    #[tokio::test]
    async fn auth_endpoints_report_safe_state_without_exposing_saved_source() {
        let dir = tempfile::tempdir().unwrap();
        let pool = ConnectionManager::new_pool(dir.path().join("data.sqlite3").to_str().unwrap())
            .await
            .unwrap();
        let config = Arc::new(RwLock::new(Config {
            douyu_cookie: Some("acf_uid=42; acf_auth=fixture-private".into()),
            douyu_ltp0: Some("fixture-long-private".into()),
            douyu_refresh_device_id: Some("fixture-device-private".into()),
            ..Config::default()
        }));
        let managers = DownloadManager::new(5, 3, pool.clone());
        let (_layer, log_handle) = reload::Layer::new(EnvFilter::new("info"));
        let services = ServiceRegister::new(pool, config, managers, log_handle).await;
        let app = router().with_state(services.clone());
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/douyu/auth/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let body = to_bytes(response.into_body(), 8192).await.unwrap();
        let text = std::str::from_utf8(&body).unwrap();
        for secret in [
            "fixture-private",
            "fixture-long-private",
            "fixture-device-private",
            "source_hash",
            "lease_token",
        ] {
            assert!(!text.contains(secret));
        }
        let status: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(status["has_ltp0"], true);
        assert_eq!(status["login_state"], "unknown");
        // A missing room is rejected before any remote request.
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/douyu/auth/refresh")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"streamer_id":123456}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        services.cleanup().await;
    }
}
