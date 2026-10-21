//! Offline room-cover lookup without resolving or opening media streams.

use super::live_image::{StoredAvatar, cover_store, image_client};
use crate::server::core::download_manager::DownloadManager;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::WorkerStatus;
use biliup::downloader::live::media_ext_from_url;
use futures::{StreamExt, stream};
use regex::Regex;
use reqwest::Client;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;
use tracing::debug;
use url::Url;

const METADATA_TTL: Duration = Duration::from_secs(1800);
const LIVE_TTL: Duration = Duration::from_secs(300);
const MAX_METADATA_BYTES: usize = 2 * 1024 * 1024;

struct RefreshState {
    room_url: String,
    live_cover: Option<String>,
    attempted: Instant,
    running: bool,
    retry_after: Duration,
}

static ATTEMPTS: LazyLock<Mutex<HashMap<i64, RefreshState>>> = LazyLock::new(Default::default);
static CLIENT: LazyLock<Client> = LazyLock::new(image_client);
// HTTP requests and background refreshes share the same upstream budget.
static REFRESH_SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(3);

struct RefreshGuard(i64);
impl Drop for RefreshGuard {
    fn drop(&mut self) {
        if let Some(state) = ATTEMPTS.lock().unwrap().get_mut(&self.0) {
            state.running = false;
        }
    }
}

/// Returns the latest valid file, retaining it when metadata or a CDN request fails.
/// Each room has one refresh in flight; repeated page requests do not multiply upstream calls.
pub async fn refresh_room_cover(
    pool: &ConnectionPool,
    id: i64,
    room_url: &str,
    live_cover: Option<&str>,
) -> Option<StoredAvatar> {
    let live_cover = live_cover.filter(|url| valid_image_url(url));
    let should_refresh = {
        let mut states = ATTEMPTS.lock().unwrap();
        match states.get(&id) {
            Some(state) if state.running => false,
            Some(state)
                if state.room_url == room_url
                    && state.live_cover.as_deref() == live_cover
                    && state.attempted.elapsed() < state.retry_after =>
            {
                false
            }
            _ => {
                states.insert(
                    id,
                    RefreshState {
                        room_url: room_url.to_string(),
                        live_cover: live_cover.map(str::to_string),
                        attempted: Instant::now(),
                        running: true,
                        retry_after: LIVE_TTL,
                    },
                );
                true
            }
        }
    };
    if !should_refresh {
        return cover_store().load(id).await;
    }
    let _guard = RefreshGuard(id);
    let _slot = REFRESH_SLOTS.acquire().await.ok()?;
    let store = cover_store();
    if let Some(url) = live_cover {
        if let Ok(stored) = store.ensure(id, url, room_url).await {
            return Some(stored);
        }
    }
    // Bootstrap an old room from recording history before requesting fresh public metadata.
    if store.load(id).await.is_none() {
        let history: Vec<String> = sqlx::query_scalar(
            "SELECT live_cover_path FROM stream_sessions WHERE url = ? AND live_cover_path <> '' ORDER BY id DESC LIMIT 3"
        ).bind(room_url).fetch_all(pool).await.unwrap_or_default();
        for url in history {
            if valid_image_url(&url) && store.ensure(id, &url, room_url).await.is_ok() {
                break;
            }
        }
    }
    let refreshed = match fetch_metadata_cover(&CLIENT, room_url).await {
        Ok(Some(url)) => {
            if let Err(reason) = store.ensure(id, &url, room_url).await {
                debug!(id, reason, "房间封面更新失败，保留旧图片");
                false
            } else {
                true
            }
        }
        Ok(None) => false,
        Err(reason) => {
            debug!(id, reason, "未取得房间封面元数据");
            false
        }
    };
    if refreshed && live_cover.is_none() {
        if let Some(state) = ATTEMPTS.lock().unwrap().get_mut(&id) {
            state.retry_after = METADATA_TTL;
        }
    }
    store.load(id).await
}

/// Keep recording covers fresh. Offline rooms refresh only when their image is requested.
pub fn spawn_room_cover_refresh(
    managers: &Arc<DownloadManager>,
    pool: ConnectionPool,
) -> JoinHandle<()> {
    let managers = Arc::downgrade(managers);
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(30));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let Some(manager) = managers.upgrade() else {
                break;
            };
            let rooms: Vec<(i64, String)> = sqlx::query_as("SELECT id,url FROM livestreamers")
                .fetch_all(&pool)
                .await
                .unwrap_or_default();
            let workers = manager.get_rooms().await;
            let active: HashMap<i64, String> = workers
                .iter()
                .filter_map(|worker| match &*worker.downloader_status.read().unwrap() {
                    WorkerStatus::Working(task) => {
                        task.live_media().cover_url.map(|url| (worker.id(), url))
                    }
                    _ => None,
                })
                .collect();
            drop(manager);
            let ids: HashSet<i64> = rooms.iter().map(|(id, _)| *id).collect();
            ATTEMPTS
                .lock()
                .unwrap()
                .retain(|id, state| state.running || ids.contains(id));
            stream::iter(rooms.into_iter().filter(|(id, _)| active.contains_key(id)))
                .for_each_concurrent(3, |(id, room_url)| {
                    let pool = pool.clone();
                    let cover = active.get(&id).cloned();
                    async move {
                        refresh_room_cover(&pool, id, &room_url, cover.as_deref()).await;
                    }
                })
                .await;
        }
    })
}

fn valid_image_url(value: &str) -> bool {
    Url::parse(value)
        .ok()
        .is_some_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
}

async fn get_text(client: &Client, url: &str, referer: &str) -> Result<String, String> {
    let mut response = client
        .get(url)
        .header("referer", referer)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    if response
        .content_length()
        .is_some_and(|len| len > MAX_METADATA_BYTES as u64)
    {
        return Err("房间元数据超过大小限制".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len() + chunk.len() > MAX_METADATA_BYTES {
            return Err("房间元数据超过大小限制".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

async fn fetch_metadata_cover(client: &Client, room_url: &str) -> Result<Option<String>, String> {
    let parsed = Url::parse(room_url).map_err(|e| e.to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Ok(None);
    }
    // 通用插件也可以登记媒体直链。封面任务不能为这些地址额外打开视频或清单连接。
    if media_ext_from_url(room_url).is_some() {
        return Ok(None);
    }
    let room_id = parsed
        .path_segments()
        .and_then(|mut parts| parts.find(|part| !part.is_empty()));
    let host = parsed.host_str().unwrap_or_default();
    let api = match (host, room_id) {
        ("live.bilibili.com", Some(id)) if id.chars().all(|c| c.is_ascii_digit()) => Some(format!(
            "https://api.live.bilibili.com/xlive/web-room/v1/index/getInfoByRoom?room_id={id}"
        )),
        ("www.douyu.com" | "m.douyu.com" | "douyu.com", Some(id))
            if id.chars().all(|c| c.is_ascii_digit()) =>
        {
            Some(format!("https://www.douyu.com/betard/{id}"))
        }
        ("www.huya.com" | "m.huya.com" | "huya.com", Some(id))
            if id.chars().all(|c| c.is_ascii_digit()) =>
        {
            Some(format!(
                "https://mp.huya.com/cache.php?m=Live&do=profileRoom&roomid={id}"
            ))
        }
        _ => None,
    };
    if let Some(api) = api {
        if let Ok(text) = get_text(client, &api, room_url).await {
            if let Ok(value) = serde_json::from_str::<Value>(&text) {
                if let Some(cover) = metadata_json_cover(&value, &parsed) {
                    return Ok(Some(cover));
                }
            }
        }
    }
    let page = get_text(client, room_url, room_url).await?;
    Ok(metadata_page_cover(&page, &parsed))
}

fn normalized_cover(value: &str, room_url: &Url) -> Option<String> {
    if value.trim().is_empty() {
        return None;
    }
    let value = value
        .replace("&amp;", "&")
        .replace("&#38;", "&")
        .replace("&quot;", "\"");
    let url = room_url.join(value.trim()).ok()?;
    valid_image_url(url.as_str()).then(|| url.to_string())
}

fn metadata_json_cover(value: &Value, room_url: &Url) -> Option<String> {
    [
        "/data/room_info/cover",
        "/data/room_info/user_cover",
        "/data/room_info/keyframe",
        "/room/room_pic",
        "/data/liveData/screenshot",
        "/data/liveInfo/screenshot",
    ]
    .iter()
    .filter_map(|path| value.pointer(path).and_then(Value::as_str))
    .find_map(|value| normalized_cover(value, room_url))
}

fn metadata_page_cover(page: &str, room_url: &Url) -> Option<String> {
    static META: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<meta\b[^>]*>").unwrap());
    static ATTR: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?is)([\w:-]+)\s*=\s*["']([^"']*)["']"#).unwrap());
    static JSON_COVER: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?i)"(?:screenshot|room_pic|cover|user_cover)"\s*:\s*("(?:\\.|[^"\\])*")"#)
            .unwrap()
    });
    for meta in META.find_iter(page) {
        let attrs: HashMap<_, _> = ATTR
            .captures_iter(meta.as_str())
            .map(|capture| (capture[1].to_ascii_lowercase(), capture[2].to_string()))
            .collect();
        if attrs
            .get("property")
            .or_else(|| attrs.get("name"))
            .is_some_and(|name| {
                matches!(
                    name.to_ascii_lowercase().as_str(),
                    "og:image" | "og:image:secure_url" | "twitter:image"
                )
            })
        {
            if let Some(cover) = attrs
                .get("content")
                .and_then(|value| normalized_cover(value, room_url))
            {
                return Some(cover);
            }
        }
    }
    JSON_COVER
        .captures_iter(page)
        .filter_map(|capture| serde_json::from_str::<String>(&capture[1]).ok())
        .find_map(|value| normalized_cover(&value, room_url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_is_available_without_live_status_or_stream_urls() {
        let room = Url::parse("https://live.bilibili.com/1").unwrap();
        let offline = serde_json::json!({"data":{"room_info":{"live_status":0,"cover":"", "user_cover":"https://i0.hdslb.com/a.jpg"}}});
        assert_eq!(
            metadata_json_cover(&offline, &room).as_deref(),
            Some("https://i0.hdslb.com/a.jpg")
        );
        let douyu =
            serde_json::json!({"room":{"show_status":2,"room_pic":"//rpic.douyucdn.cn/a.avif"}});
        assert_eq!(
            metadata_json_cover(&douyu, &room).as_deref(),
            Some("https://rpic.douyucdn.cn/a.avif")
        );
        let huya = serde_json::json!({"data":{"liveStatus":"OFF","liveData":{"screenshot":"https://x.example/a.jpg"}}});
        assert!(metadata_json_cover(&huya, &room).is_some());
    }

    #[test]
    fn page_metadata_handles_attribute_order_relative_urls_and_json_escaping() {
        let room = Url::parse("https://www.huya.com/1").unwrap();
        let page = r#"<meta content='/cover.jpg?a=1&amp;b=2' property='og:image'>"#;
        assert_eq!(
            metadata_page_cover(page, &room).as_deref(),
            Some("https://www.huya.com/cover.jpg?a=1&b=2")
        );
        let page = r#"var data = {"screenshot":"https:\/\/x.example\/a.jpg"};"#;
        assert_eq!(
            metadata_page_cover(page, &room).as_deref(),
            Some("https://x.example/a.jpg")
        );
        assert_eq!(
            metadata_page_cover(
                r#"<meta property='og:image' content='javascript:alert(1)'>"#,
                &room
            ),
            None
        );
        assert_eq!(metadata_page_cover("<html>no cover</html>", &room), None);
    }

    #[tokio::test]
    async fn lookup_visits_room_metadata_without_fetching_media_or_forwarding_credentials() {
        use axum::{Router, http::HeaderMap, routing::get};
        let headers_seen = Arc::new(Mutex::new(Vec::new()));
        let router = Router::new().route("/room", get({
            let headers_seen = headers_seen.clone();
            move |headers: HeaderMap| {
                headers_seen.lock().unwrap().push(headers);
                async { r#"<meta name="og:image" content="/cover.jpg"><video src="/media.flv"></video>"# }
            }
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let room = format!("{base}/room");
        assert_eq!(
            fetch_metadata_cover(&image_client(), &room).await.unwrap(),
            Some(format!("{base}/cover.jpg"))
        );
        let seen = headers_seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].get("referer").unwrap(), room.as_str());
        assert!(seen[0].get("cookie").is_none());
        assert!(seen[0].get("authorization").is_none());
        drop(seen);
        assert_eq!(
            fetch_metadata_cover(&image_client(), &format!("{base}/media.flv"))
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            fetch_metadata_cover(&image_client(), &format!("{base}/playlist?format=m3u8"))
                .await
                .unwrap(),
            None
        );
        assert_eq!(
            headers_seen.lock().unwrap().len(),
            1,
            "media URLs are never requested"
        );
        server.abort();
    }
}
