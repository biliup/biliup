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

impl Kind {
    /// 等网络恢复后重试有意义的故障：连不上（含 DNS 解析失败）、超时、请求没发完或响应没收完、
    /// HTTP 5xx。限流（601）、风控、其他 4xx、本地 IO、解析错误都不算，重试只会无用或更糟。
    pub fn is_transient(&self) -> bool {
        match self {
            Kind::Reqwest(e) => transient_reqwest(e),
            Kind::ReqwestMiddleware(e) => transient_middleware(e),
            _ => false,
        }
    }
}

fn transient_reqwest(e: &reqwest::Error) -> bool {
    match e.status() {
        Some(status) => status.is_server_error(),
        None => e.is_connect() || e.is_timeout() || e.is_request() || e.is_body(),
    }
}

/// `reqwest-retry` 重试用尽后把最后一次的错误包在 `Middleware(RetryError)` 里
fn transient_middleware(e: &reqwest_middleware::Error) -> bool {
    match e {
        reqwest_middleware::Error::Reqwest(e) => transient_reqwest(e),
        reqwest_middleware::Error::Middleware(e) => {
            match e.downcast_ref::<reqwest_retry::RetryError>() {
                Some(
                    reqwest_retry::RetryError::WithRetries { err, .. }
                    | reqwest_retry::RetryError::Error(err),
                ) => transient_middleware(err),
                None => false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Kind;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// 本机没人监听的端口：`send()` 立刻得到连接被拒，与断网时的 DNS 失败同属连接错误
    const REFUSED: &str = "http://127.0.0.1:1/";

    async fn send_error(url: &str) -> reqwest::Error {
        reqwest::Client::new().get(url).send().await.unwrap_err()
    }

    /// 读完请求头就回 `status` 的本地 HTTP 服务
    async fn status_server(status: u16) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = vec![0u8; 4096];
                let _ = socket.read(&mut buf).await;
                let reply = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
                let _ = socket.write_all(reply.as_bytes()).await;
            }
        });
        url
    }

    async fn status_error(status: u16) -> Kind {
        let url = status_server(status).await;
        let response = reqwest::get(&url).await.unwrap();
        Kind::from(response.error_for_status().unwrap_err())
    }

    #[tokio::test]
    async fn network_failures_and_server_errors_are_transient() {
        assert!(Kind::from(send_error(REFUSED).await).is_transient());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let held = tokio::spawn(async move {
            let mut sockets = Vec::new();
            while let Ok((socket, _)) = listener.accept().await {
                sockets.push(socket);
            }
        });
        let timeout = reqwest::Client::new()
            .get(&url)
            .timeout(Duration::from_millis(200))
            .send()
            .await
            .unwrap_err();
        assert!(timeout.is_timeout());
        assert!(Kind::from(timeout).is_transient());
        held.abort();

        assert!(status_error(503).await.is_transient());
    }

    #[tokio::test]
    async fn rejections_and_local_errors_are_not_transient() {
        assert!(!status_error(403).await.is_transient());
        assert!(!status_error(404).await.is_transient());
        assert!(
            !Kind::RateLimit {
                code: 601,
                message: "上传过快".into()
            }
            .is_transient()
        );
        assert!(!Kind::Custom("Failed to pre_upload from {}".into()).is_transient());
        assert!(!Kind::from(std::io::Error::from(std::io::ErrorKind::NotFound)).is_transient());
    }

    /// `client_with_middleware` 重试用尽后的错误形态，与 `StatelessClient` 用的中间件相同
    #[tokio::test]
    async fn exhausted_middleware_retries_keep_the_underlying_cause() {
        use reqwest_retry::RetryTransientMiddleware;
        use reqwest_retry::policies::ExponentialBackoff;
        let policy = ExponentialBackoff::builder()
            .retry_bounds(Duration::from_millis(1), Duration::from_millis(2))
            .build_with_max_retries(1);
        let client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
            .with(RetryTransientMiddleware::new_with_policy(policy))
            .build();
        let error = client.get(REFUSED).send().await.unwrap_err();
        assert!(matches!(error, reqwest_middleware::Error::Middleware(_)));
        assert!(Kind::from(error).is_transient());

        let other = reqwest_middleware::Error::middleware(std::io::Error::other("bad middleware"));
        assert!(!Kind::from(other).is_transient());
    }

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
