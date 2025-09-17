use std::borrow::Cow;
use std::{collections::HashMap, fmt::Debug};

use axum::response::Response;
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

use thiserror::Error;
use tracing::log::error;

pub type AppResult<T> = Result<T, AppError>;

pub type ConduitErrorMap = HashMap<Cow<'static, str>, Vec<Cow<'static, str>>>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("authentication is required to access this resource")]
    Unauthorized,
    #[error("username or password is incorrect")]
    InvalidLoginAttempt,
    #[error("user does not have privilege to access this resource")]
    Forbidden,
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    ApplicationStartup(String),
    #[error("{0}")]
    BadRequest(String),
    #[error("unexpected error has occurred")]
    InternalServerError,
    #[error("{0}")]
    InternalServerErrorWithContext(String),
    #[error("{0}")]
    ObjectConflict(String),
    #[error("unprocessable request has occurred")]
    UnprocessableEntity { errors: ConduitErrorMap },
    #[error(transparent)]
    AxumJsonRejection(#[from] axum::extract::rejection::JsonRejection),
    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
    #[error(transparent)]
    DownloadError(#[from] biliup::downloader::error::Error),
    #[error(transparent)]
    UploadError(#[from] biliup::error::Kind),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    OrmliteError(#[from] ormlite::Error),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        error!("AppError{}", AppError = self);
        let (status, error_message) = match self {
            Self::InternalServerErrorWithContext(err) => (StatusCode::INTERNAL_SERVER_ERROR, err),
            Self::NotFound(err) => (StatusCode::NOT_FOUND, err),
            Self::ObjectConflict(err) => (StatusCode::CONFLICT, err),
            Self::InvalidLoginAttempt => (
                StatusCode::BAD_REQUEST,
                Self::InvalidLoginAttempt.to_string(),
            ),
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, Self::Unauthorized.to_string()),
            Self::AnyhowError(err) => 'e: {
                for cause in err.chain() {
                    if let Some(error) = cause.downcast_ref::<Box<dyn sqlx::error::DatabaseError>>()
                    {
                        println!("121212cause: {}", error.message());
                        break 'e (StatusCode::BAD_REQUEST, error.to_string());
                    }
                    println!("{}", cause);
                }
                (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            }
            AppError::Forbidden
            | AppError::ApplicationStartup(_)
            | AppError::BadRequest(_)
            | AppError::InternalServerError
            | AppError::UnprocessableEntity { .. }
            | AppError::AxumJsonRejection(_)
            | AppError::DownloadError(_)
            | AppError::UploadError(_)
            | AppError::SerdeJsonError(_)
            | AppError::ReqwestError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("unexpected error occurred"),
            ),
            AppError::OrmliteError(ormlite) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                String::from("OrmliteError occurred"),
            ),
        };

        // I'm not a fan of the error specification, so for the sake of consistency,
        // serialize singular errors as a map of vectors similar to the 422 validation responses
        let body = Json(ApiError::new(error_message));

        (status, body).into_response()
    }
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
