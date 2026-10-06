//! 斗鱼网宿直链追加 `expire=0` 的拉流前探测。
//!
//! stream-gears 在自己的首连里处理 403（见 `stream_gears::connect`）；ffmpeg / mesio /
//! streamlink / 边录边传把直链交给外部进程或库，拿不到首连的状态码，所以在交出去之前
//! 先用同一组请求头探一次，确认网宿接受追加后的直链才用它，否则退回原直链。

use crate::server::common::construct_headers;
use biliup::downloader::live::strip_ws_expire_override;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};

const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// 返回交给下载器的直链：不是追加过 `expire=0` 的网宿直链时原样返回、不发请求。
///
/// 探测必须用 HEAD：网宿的 token 一经 GET 成功就作废，同一直链再 GET 只回一个 GOP
/// 就断开；HEAD 照样校验签名（不通过即 403），却不消耗 token。只有 HEAD 返回 2xx
/// 才保留追加后的直链，403、其它状态码与网络错误一律退回原直链。
pub(crate) async fn resolve_ws_expire_override(
    url: String,
    headers: &HashMap<String, String>,
) -> String {
    let Some(original) = strip_ws_expire_override(&url) else {
        return url;
    };
    let original = original.to_string();
    let client = match construct_headers(headers).map(|headers| {
        reqwest::Client::builder()
            .default_headers(headers)
            .connect_timeout(PROBE_TIMEOUT)
            .timeout(PROBE_TIMEOUT)
            .build()
    }) {
        Ok(Ok(client)) => client,
        _ => return original,
    };
    match client.head(&url).send().await {
        Ok(response) if response.status().is_success() => {
            info!(status = %response.status(), "网宿接受追加 expire=0 的直链");
            url
        }
        Ok(response) => {
            warn!(
                status = %response.status(),
                "网宿拒绝了追加 expire=0 的直链，本次改用原直链，连接仍会按 expire 定时断开"
            );
            original
        }
        Err(e) => {
            warn!(error = %e, "探测追加 expire=0 的直链失败，本次改用原直链");
            original
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::extract::RawQuery;
    use axum::http::{HeaderMap, Method, StatusCode};
    use axum::routing::any;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, PartialEq)]
    struct Hit {
        method: Method,
        user_agent: Option<String>,
    }

    struct Cdn {
        base: String,
        hits: Arc<Mutex<Vec<Hit>>>,
    }

    /// `reject_duplicate`：模拟网宿修掉重复参数的解析差异，带重复 expire 的一律 403
    async fn wangsu(reject_duplicate: bool, status_otherwise: StatusCode) -> Cdn {
        let hits = Arc::new(Mutex::new(Vec::new()));
        let seen = hits.clone();
        let app = Router::new().route(
            "/live/a.flv",
            any(
                move |method: Method, RawQuery(query): RawQuery, headers: HeaderMap| {
                    seen.lock().unwrap().push(Hit {
                        method,
                        user_agent: headers
                            .get("user-agent")
                            .map(|ua| ua.to_str().unwrap().to_string()),
                    });
                    async move {
                        let expires = query
                            .unwrap_or_default()
                            .split('&')
                            .filter(|pair| pair.starts_with("expire="))
                            .count();
                        if reject_duplicate && expires > 1 {
                            StatusCode::FORBIDDEN
                        } else {
                            status_otherwise
                        }
                    }
                },
            ),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Cdn {
            base: format!("http://{addr}/live/a.flv"),
            hits,
        }
    }

    fn headers() -> HashMap<String, String> {
        HashMap::from([("User-Agent".to_string(), "biliup-test".to_string())])
    }

    #[tokio::test]
    async fn keeps_override_when_wangsu_accepts_it() {
        let cdn = wangsu(false, StatusCode::OK).await;
        let overridden = format!("{}?wsAuth=a&token=t&expire=300&fcdn=ws&expire=0", cdn.base);

        let url = resolve_ws_expire_override(overridden.clone(), &headers()).await;

        assert_eq!(url, overridden);
        assert_eq!(
            *cdn.hits.lock().unwrap(),
            [Hit {
                method: Method::HEAD,
                user_agent: Some("biliup-test".to_string()),
            }]
        );
    }

    #[tokio::test]
    async fn falls_back_to_original_url_on_403() {
        let cdn = wangsu(true, StatusCode::OK).await;
        let original = format!("{}?wsAuth=a&token=t&expire=300&fcdn=ws", cdn.base);

        let url = resolve_ws_expire_override(format!("{original}&expire=0"), &headers()).await;

        assert_eq!(url, original);
        assert_eq!(cdn.hits.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn falls_back_to_original_url_unless_probe_succeeds() {
        let cdn = wangsu(false, StatusCode::METHOD_NOT_ALLOWED).await;
        let original = format!("{}?wsAuth=a&expire=300&fcdn=ws", cdn.base);
        assert_eq!(
            resolve_ws_expire_override(format!("{original}&expire=0"), &headers()).await,
            original
        );

        let unreachable = "http://127.0.0.1:1/live/a.flv?wsAuth=a&expire=300&fcdn=ws";
        assert_eq!(
            resolve_ws_expire_override(format!("{unreachable}&expire=0"), &headers()).await,
            unreachable
        );
    }

    #[tokio::test]
    async fn sends_nothing_for_urls_without_override() {
        let cdn = wangsu(true, StatusCode::OK).await;
        for url in [
            format!("{}?wsAuth=a&expire=300&fcdn=ws", cdn.base),
            format!("{}?wsAuth=a&expire=300&fcdn=hw&expire=0", cdn.base),
            "https://example.com/live.m3u8".to_string(),
        ] {
            assert_eq!(
                resolve_ws_expire_override(url.clone(), &headers()).await,
                url
            );
        }
        assert!(cdn.hits.lock().unwrap().is_empty());
    }
}
