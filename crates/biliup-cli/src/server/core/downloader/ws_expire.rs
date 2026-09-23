//! 斗鱼网宿直链追加 `expire=0` 的拉流前探测。
//!
//! stream-gears 在自己的首连里处理 403（见 `stream_gears::connect`）；ffmpeg / mesio /
//! streamlink / 边录边传把直链交给外部进程或库，拿不到首连的状态码，所以在交出去之前
//! 先用同一组请求头探一次：被 403 就改用原直链，其它结果一律保留追加后的直链。

use crate::server::common::construct_headers;
use biliup::downloader::live::strip_ws_expire_override;
use reqwest::StatusCode;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{info, warn};

const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// 返回交给下载器的直链：不是追加过 `expire=0` 的网宿直链时原样返回、不发请求。
/// 探测只等响应头、不读 body；网宿对 403 不消耗 token，探测成功后同一直链可再连。
pub(crate) async fn resolve_ws_expire_override(
    url: String,
    headers: &HashMap<String, String>,
) -> String {
    let Some(original) = strip_ws_expire_override(&url) else {
        return url;
    };
    let client = match construct_headers(headers).map(|headers| {
        reqwest::Client::builder()
            .default_headers(headers)
            .connect_timeout(PROBE_TIMEOUT)
            .timeout(PROBE_TIMEOUT)
            .build()
    }) {
        Ok(Ok(client)) => client,
        _ => return url,
    };
    match client.get(&url).send().await {
        Ok(response) if response.status() == StatusCode::FORBIDDEN => {
            warn!(
                "网宿拒绝了追加 expire=0 的直链（403），本次改用原直链，连接仍会按 expire 定时断开"
            );
            original.to_string()
        }
        Ok(response) => {
            info!(status = %response.status(), "网宿直链 expire=0 探测通过");
            url
        }
        Err(e) => {
            warn!(error = %e, "网宿直链 expire=0 探测失败，保留追加后的直链");
            url
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::extract::RawQuery;
    use axum::http::{HeaderMap, StatusCode as HttpStatus};
    use axum::routing::get;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    struct Cdn {
        base: String,
        hits: Arc<AtomicUsize>,
        user_agents: Arc<Mutex<Vec<String>>>,
    }

    /// `reject_duplicate`：模拟网宿修掉重复参数的解析差异，带重复 expire 的一律 403
    async fn wangsu(reject_duplicate: bool, status_otherwise: HttpStatus) -> Cdn {
        let hits = Arc::new(AtomicUsize::new(0));
        let user_agents = Arc::new(Mutex::new(Vec::new()));
        let (counter, seen) = (hits.clone(), user_agents.clone());
        let app = Router::new().route(
            "/live/a.flv",
            get(move |RawQuery(query): RawQuery, headers: HeaderMap| {
                counter.fetch_add(1, Ordering::SeqCst);
                if let Some(ua) = headers.get("user-agent") {
                    seen.lock().unwrap().push(ua.to_str().unwrap().to_string());
                }
                async move {
                    let expires = query
                        .unwrap_or_default()
                        .split('&')
                        .filter(|pair| pair.starts_with("expire="))
                        .count();
                    if reject_duplicate && expires > 1 {
                        (HttpStatus::FORBIDDEN, "Invalid Request")
                    } else {
                        (status_otherwise, "FLV")
                    }
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        Cdn {
            base: format!("http://{addr}/live/a.flv"),
            hits,
            user_agents,
        }
    }

    fn headers() -> HashMap<String, String> {
        HashMap::from([("User-Agent".to_string(), "biliup-test".to_string())])
    }

    #[tokio::test]
    async fn keeps_override_when_wangsu_accepts_it() {
        let cdn = wangsu(false, HttpStatus::OK).await;
        let overridden = format!("{}?wsAuth=a&token=t&expire=300&fcdn=ws&expire=0", cdn.base);

        let url = resolve_ws_expire_override(overridden.clone(), &headers()).await;

        assert_eq!(url, overridden);
        assert_eq!(cdn.hits.load(Ordering::SeqCst), 1);
        assert_eq!(*cdn.user_agents.lock().unwrap(), ["biliup-test"]);
    }

    #[tokio::test]
    async fn falls_back_to_original_url_on_403() {
        let cdn = wangsu(true, HttpStatus::OK).await;
        let original = format!("{}?wsAuth=a&token=t&expire=300&fcdn=ws", cdn.base);

        let url = resolve_ws_expire_override(format!("{original}&expire=0"), &headers()).await;

        assert_eq!(url, original);
        assert_eq!(cdn.hits.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn keeps_override_on_other_errors() {
        let cdn = wangsu(false, HttpStatus::NOT_FOUND).await;
        let overridden = format!("{}?wsAuth=a&expire=300&fcdn=ws&expire=0", cdn.base);

        assert_eq!(
            resolve_ws_expire_override(overridden.clone(), &headers()).await,
            overridden
        );

        let unreachable = "http://127.0.0.1:1/live/a.flv?wsAuth=a&expire=300&fcdn=ws&expire=0";
        assert_eq!(
            resolve_ws_expire_override(unreachable.to_string(), &headers()).await,
            unreachable
        );
    }

    #[tokio::test]
    async fn sends_nothing_for_urls_without_override() {
        let cdn = wangsu(true, HttpStatus::OK).await;
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
        assert_eq!(cdn.hits.load(Ordering::SeqCst), 0);
    }
}
