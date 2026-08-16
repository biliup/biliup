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
    Reqwest(reqwest::Error),

    #[error(transparent)]
    ReqwestMiddleware(reqwest_middleware::Error),

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
    #[error("need recaptcha")]
    NeedRecaptcha(String),

    #[error("upload rate limit (code: {code}): {message}")]
    RateLimit { code: i64, message: String },
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

impl From<reqwest::Error> for Kind {
    fn from(error: reqwest::Error) -> Self {
        // Request URLs can contain access keys, CSRF values, upload IDs and
        // signed query strings. Keep the error category/status, but never let
        // the URL reach CLI output or persistent Web logs.
        Self::Reqwest(error.without_url())
    }
}

impl From<reqwest_middleware::Error> for Kind {
    fn from(error: reqwest_middleware::Error) -> Self {
        Self::ReqwestMiddleware(error.without_url())
    }
}

#[cfg(test)]
mod tests {
    use super::Kind;

    fn request_error_with_secret_url() -> reqwest::Error {
        reqwest::Client::new()
            .get("://invalid")
            .build()
            .unwrap_err()
            .with_url(
                reqwest::Url::parse("https://example.invalid/?access_key=url-secret-marker")
                    .unwrap(),
            )
    }

    #[test]
    fn uploader_http_errors_strip_sensitive_request_urls() {
        let direct = Kind::from(request_error_with_secret_url());
        assert!(!format!("{direct:?}").contains("url-secret-marker"));

        let middleware = Kind::from(reqwest_middleware::Error::from(
            request_error_with_secret_url(),
        ));
        assert!(!format!("{middleware:?}").contains("url-secret-marker"));
    }
}
