use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use error_stack::Report;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::log::error;

/// 应用程序错误类型枚举
#[derive(Error, Debug)]
pub enum AppError {
    /// 未知错误
    #[error("Unknown Error")]
    Unknown,

    /// 自定义错误消息
    #[error("{0}")]
    Custom(String),
}

/// 将错误报告转换为HTTP响应
pub fn report_to_response(report: impl Into<Report<AppError>>) -> Response {
    let report = report.into();
    let (status, error_message) = match report.downcast_ref::<AppError>() {
        Some(AppError::Unknown) => (StatusCode::INTERNAL_SERVER_ERROR, report.to_string()),
        Some(AppError::Custom(msg)) => (StatusCode::INTERNAL_SERVER_ERROR, msg.into()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, report.to_string()),
    };
    tracing::error!(error = ?report);
    // 为了保持一致性，将单个错误序列化为类似422验证响应的映射向量格式
    let body = Json(ApiError::new(error_message));

    (status, body).into_response()
}

/// API错误响应结构体
#[derive(Debug, Deserialize, Serialize)]
pub struct ApiError {
    /// 错误详情列表
    pub errors: Vec<HashMap<String, String>>,
    /// 错误消息
    pub message: String,
}

impl ApiError {
    /// 创建新的API错误
    pub fn new(message: String) -> Self {
        let errors: Vec<HashMap<String, String>> = Vec::new();
        // 可以根据需要添加具体的错误字段信息
        // errors.push(HashMap::from([
        //     (String::from("resource"), "Issue".to_string()),
        //     (String::from("field"), "title".to_string()),
        //     (String::from("code"), "missing_field".to_string()),
        //     ...
        // ]));
        Self { errors, message }
    }
}

/// 应用程序结果类型别名
pub type AppResult<T> = Result<T, Report<AppError>>;
