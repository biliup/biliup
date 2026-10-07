use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json,
};
use biliup::downloader::live::Douyu;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::server::infrastructure::service_register::ServiceRegister;

pub fn router() -> Router<ServiceRegister> {
    Router::new()
        .route("/v1/douyu/validate-cookie", post(validate_cookie))
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

    // 检查cookie是否包含必需字段
    let required_fields = ["acf_uid", "acf_auth"];
    let has_required_fields = required_fields.iter().all(|field| {
        payload.cookie.contains(&format!("{}=", field))
    });

    if !has_required_fields {
        return (
            StatusCode::OK,
            Json(ValidateCookieResponse {
                valid: false,
                message: Some("Cookie缺少必需字段（acf_uid, acf_auth）".to_string()),
            }),
        )
            .into_response();
    }

    // 获取HTTP客户端
    let client = service_register.client();

    // 调用实际验证逻辑
    match Douyu::validate_cookie(&payload.cookie, &client).await {
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
