use crate::server::infrastructure::service_register::ServiceRegister;
use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post};
use biliup::downloader::live::Douyu;
use serde::{Deserialize, Serialize};

pub fn router() -> Router<ServiceRegister> {
    Router::new().route("/v1/douyu/validate-cookie", post(validate_cookie))
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
