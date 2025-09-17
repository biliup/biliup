use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use error_stack::Report;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use thiserror::Error;
use tracing::log::error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Unknown Error")]
    Unknown,

    #[error("{0}")]
    Custom(String),
}

pub fn report_to_response(report: impl Into<Report<AppError>>) -> Response {
    let mut report = report.into();
    let (status, error_message) = match report.downcast_ref::<AppError>() {
        Some(AppError::Unknown) => (StatusCode::INTERNAL_SERVER_ERROR, report.to_string()),
        Some(AppError::Custom(msg)) => (StatusCode::INTERNAL_SERVER_ERROR, msg.into()),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, report.to_string()),
    };
    tracing::error!(error = ?report);
    // I'm not a fan of the error specification, so for the sake of consistency,
    // serialize singular errors as a map of vectors similar to the 422 validation responses
    let body = Json(ApiError::new(error_message));

    (status, body).into_response()
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiError {
    pub errors: Vec<HashMap<String, String>>,
    pub message: String,
}

impl ApiError {
    pub fn new(message: String) -> Self {
        let errors: Vec<HashMap<String, String>> = Vec::new();
        // errors.push(HashMap::from([
        //     (String::from("resource"), "Issue".to_string()),
        //     (String::from("field"), "title".to_string()),
        //     (String::from("code"), "missing_field".to_string()),
        //     (String::from("code"), "unprocessable".to_string()),
        //     (String::from("code"), "already_exists".to_string()),
        //     (String::from("code"), "invalid".to_string()),
        //     (String::from("code"), "missing".to_string()),
        // ]));
        Self { errors, message }
    }
}

pub(crate) type AppResult<T> = Result<T, Report<AppError>>;
