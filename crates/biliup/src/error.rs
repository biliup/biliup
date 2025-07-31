use reqwest::header::{InvalidHeaderName, InvalidHeaderValue};

use thiserror::Error;

pub type Result<T> = core::result::Result<T, Kind>;

#[derive(Error, Debug)]
pub enum Kind {
    #[error("{0}")]
    Custom(String),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),

    #[error(transparent)]
    ReqwestMiddleware(#[from] reqwest_middleware::Error),

    #[error(transparent)]
    InvalidHeaderValue(#[from] InvalidHeaderValue),

    #[error(transparent)]
    InvalidHeaderName(#[from] InvalidHeaderName),

    #[error(transparent)]
    SerdeYaml(#[from] serde_yaml::Error),

    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    SerdeUrl(#[from] serde_urlencoded::ser::Error),
    // source and Display delegate to anyhow::Error
    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),

    #[error("need recaptcha")]
    NeedRecaptcha(String),
}

impl From<&str> for Kind {
    fn from(s: &str) -> Self {
        Self::Custom(s.into())
    }
}

impl From<String> for Kind {
    fn from(s: String) -> Self {
        Self::Custom(s)
    }
}
