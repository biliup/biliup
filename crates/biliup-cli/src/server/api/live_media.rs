//! 正在录制的直播间封面 / 主播头像接口。
//!
//! - 封面每场直播都可能换：服务端以直播间页面地址作 Referer 去取（页面直连图片 CDN 会被
//!   B 站等按站外 Referer 拒绝），按图片 URL 做一份短暂的内存缓存后转发。
//! - 头像几乎不变：开播时下载一次存到 `data/avatar/`，这里直接读本地文件；
//!   本地还没有（或地址变了）时现场下载一次再返回。
//!
//! 只按主播 id 取 worker 里已经拿到的地址，不接受任意 URL，避免变成开放代理。
//! 路由注册在 `router()` 里，`--auth` 时与 `/v1/streamers` 同一道登录校验。

use crate::server::common::live_image::{
    AvatarStore, FetchedImage, LiveImage, avatar_store, fetch_image, image_client, upstream_url,
};
use crate::server::core::download_manager::DownloadManager;
use crate::server::infrastructure::context::{Worker, WorkerStatus};
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::warn;

/// 封面在内存里的缓存时间
const CACHE_TTL: Duration = Duration::from_secs(300);
/// 缓存条目上限（按图片 URL）。默认 pool1_size = 5，远用不到这个数
const MAX_CACHE_ENTRIES: usize = 64;

#[derive(Debug, Clone)]
struct CachedImage {
    fetched_at: Instant,
    content_type: String,
    body: Bytes,
}

/// 封面的抓取客户端与内存缓存，进程内唯一，放在 `ServiceRegister` 里。
pub struct ImageProxy {
    client: Client,
    cache: Mutex<HashMap<String, CachedImage>>,
}

impl Default for ImageProxy {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageProxy {
    pub fn new() -> Self {
        Self {
            client: image_client(),
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn cached(&self, url: &str, now: Instant) -> Option<CachedImage> {
        let cache = self.cache.lock().unwrap();
        cache
            .get(url)
            .filter(|image| now.duration_since(image.fetched_at) < CACHE_TTL)
            .cloned()
    }

    fn store(&self, url: String, image: CachedImage, now: Instant) {
        let mut cache = self.cache.lock().unwrap();
        cache.retain(|_, image| now.duration_since(image.fetched_at) < CACHE_TTL);
        if cache.len() >= MAX_CACHE_ENTRIES
            && let Some(oldest) = cache
                .iter()
                .min_by_key(|(_, image)| image.fetched_at)
                .map(|(url, _)| url.clone())
        {
            cache.remove(&oldest);
        }
        cache.insert(url, image);
    }

    /// 取图：命中缓存直接返回，否则带 Referer 去上游拉一次并缓存。
    async fn fetch(
        &self,
        url: &str,
        referer: &str,
        kind: LiveImage,
    ) -> Result<CachedImage, String> {
        let url = upstream_url(url, kind);
        let now = Instant::now();
        if let Some(image) = self.cached(&url, now) {
            return Ok(image);
        }
        let FetchedImage { content_type, body } = fetch_image(&self.client, &url, referer).await?;
        let image = CachedImage {
            fetched_at: Instant::now(),
            content_type,
            body,
        };
        self.store(url, image.clone(), image.fetched_at);
        Ok(image)
    }
}

/// `GET /v1/streamers/{id}/cover`：正在录制的直播间封面，服务端转发并短暂缓存。
pub async fn get_live_cover(
    State(managers): State<Arc<DownloadManager>>,
    State(proxy): State<Arc<ImageProxy>>,
    Path(id): Path<i64>,
) -> Response {
    let worker = managers.get_room_by_id(id).await;
    let (url, referer) = match resolve_target(worker.as_deref(), LiveImage::Cover) {
        Ok(target) => target,
        Err(rejection) => return rejection.into_response(),
    };
    match proxy.fetch(&url, &referer, LiveImage::Cover).await {
        Ok(image) => image_response(&image.content_type, image.body, "private, max-age=300"),
        Err(reason) => {
            warn!(url, referer, reason, "拉取直播间封面失败");
            (StatusCode::BAD_GATEWAY, format!("拉取封面失败: {reason}")).into_response()
        }
    }
}

/// `GET /v1/streamers/{id}/avatar`：主播头像，读 `data/avatar/` 里的本地文件。
///
/// 正在录制且本地还没有这个地址的头像时，现场下载一次落盘；下载失败退回旧文件。
/// 不在录制时只读本地，没有就 404。
pub async fn get_live_avatar(
    State(managers): State<Arc<DownloadManager>>,
    Path(id): Path<i64>,
) -> Response {
    let Some(worker) = managers.get_room_by_id(id).await else {
        return (StatusCode::NOT_FOUND, "直播间不存在").into_response();
    };
    serve_avatar(&worker, avatar_store()).await
}

async fn serve_avatar(worker: &Worker, store: &AvatarStore) -> Response {
    let id = worker.live_streamer.id;
    let stored = match resolve_target(Some(worker), LiveImage::Avatar) {
        Ok((url, referer)) => match store.ensure(id, &url, &referer).await {
            Ok(stored) => Some(stored),
            Err(reason) => {
                warn!(id, url, reason, "下载主播头像失败，尝试用本地旧文件");
                store.load(id).await
            }
        },
        Err(_) => store.load(id).await,
    };
    let Some(stored) = stored else {
        return (StatusCode::NOT_FOUND, "该直播间没有可用的头像").into_response();
    };
    match tokio::fs::read(store.image_path(&stored)).await {
        Ok(bytes) => image_response(
            &stored.content_type,
            Bytes::from(bytes),
            "private, max-age=3600",
        ),
        Err(e) => {
            warn!(id, error = %e, "读取本地头像失败");
            (StatusCode::INTERNAL_SERVER_ERROR, "读取本地头像失败").into_response()
        }
    }
}

/// 从正在录制的 worker 里取出图片地址，以及请求它时要带的 Referer（直播间页面地址）。
fn resolve_target(
    worker: Option<&Worker>,
    kind: LiveImage,
) -> Result<(String, String), (StatusCode, String)> {
    let Some(worker) = worker else {
        return Err((StatusCode::NOT_FOUND, "直播间不存在".to_string()));
    };
    let media = match &*worker.downloader_status.read().unwrap() {
        WorkerStatus::Working(task) => task.live_media(),
        _ => return Err((StatusCode::NOT_FOUND, "直播间未在录制".to_string())),
    };
    let url = match kind {
        LiveImage::Cover => media.cover_url,
        LiveImage::Avatar => media.avatar_url,
    };
    let Some(url) = url.filter(|url| url.starts_with("http://") || url.starts_with("https://"))
    else {
        return Err((
            StatusCode::NOT_FOUND,
            format!("该直播间没有可用的{}", kind.label()),
        ));
    };
    Ok((url, worker.live_streamer.url.clone()))
}

fn image_response(content_type: &str, body: Bytes, cache_control: &'static str) -> Response {
    let mut response = body.into_response();
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(content_type) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(cache_control),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::common::download::DownloadTask;
    use crate::server::config::Config;
    use crate::server::core::downloader::{DownloaderRuntime, DownloaderType};
    use crate::server::infrastructure::context::Stage;
    use crate::server::infrastructure::models::live_streamer::LiveStreamer;
    use axum::body::to_bytes;
    use axum::routing::get;
    use axum::{Router, http::HeaderMap};
    use biliup::downloader::live::{DownloaderHint, LiveStream};
    use std::sync::RwLock;

    fn streamer(url: &str) -> LiveStreamer {
        LiveStreamer {
            id: 7,
            url: url.to_string(),
            remark: "x".to_string(),
            filename_prefix: None,
            time_range: None,
            upload_streamers_id: None,
            format: None,
            override_cfg: None,
            preprocessor: None,
            segment_processor: None,
            downloaded_processor: None,
            postprocessor: None,
            opt_args: None,
            excluded_keywords: None,
        }
    }

    fn live_stream(cover: &str, avatar: Option<&str>) -> LiveStream {
        LiveStream {
            name: "n".into(),
            url: "https://live.bilibili.com/1".into(),
            title: "t".into(),
            date: chrono::Utc::now(),
            live_cover_url: cover.into(),
            avatar_url: avatar.map(str::to_string),
            raw_stream_url: "https://cdn.example/live.flv".into(),
            platform: "bilibili".into(),
            stream_headers: HashMap::new(),
            suffix: "flv".into(),
            danmaku: None,
            downloader_hint: DownloaderHint::StreamGears,
            runtime_options: None,
        }
    }

    fn idle_worker() -> Worker {
        Worker::new(
            streamer("https://live.bilibili.com/1"),
            None,
            Arc::new(RwLock::new(Config::default())),
            Default::default(),
        )
    }

    async fn working_worker(cover: &str, avatar: Option<&str>) -> Worker {
        let worker = idle_worker();
        let stream = live_stream(cover, avatar);
        let task = DownloadTask::new(
            DownloaderRuntime::from_type(DownloaderType::StreamGears),
            &stream,
        );
        worker
            .change_status(Stage::Download, WorkerStatus::Working(Arc::new(task)))
            .await;
        worker
    }

    #[tokio::test]
    async fn only_a_recording_room_with_a_url_resolves_to_a_target() {
        assert_eq!(
            resolve_target(None, LiveImage::Cover)
                .err()
                .map(|(status, _)| status),
            Some(StatusCode::NOT_FOUND)
        );
        assert_eq!(
            resolve_target(Some(&idle_worker()), LiveImage::Cover)
                .err()
                .map(|(status, _)| status),
            Some(StatusCode::NOT_FOUND)
        );

        let working = working_worker("https://i0.hdslb.com/cover.jpg", None).await;
        assert_eq!(
            resolve_target(Some(&working), LiveImage::Cover).unwrap(),
            (
                "https://i0.hdslb.com/cover.jpg".to_string(),
                "https://live.bilibili.com/1".to_string()
            )
        );
        // 没有头像 -> 404，而不是去请求一个空地址
        assert_eq!(
            resolve_target(Some(&working), LiveImage::Avatar)
                .err()
                .map(|(status, _)| status),
            Some(StatusCode::NOT_FOUND)
        );
        // 非 http(s) 地址不代理
        let odd = working_worker("data:image/png;base64,AAAA", None).await;
        assert_eq!(
            resolve_target(Some(&odd), LiveImage::Cover)
                .err()
                .map(|(status, _)| status),
            Some(StatusCode::NOT_FOUND)
        );
    }

    #[tokio::test]
    async fn cover_proxy_fetches_with_the_room_page_as_referer_and_caches_by_url() {
        let seen: Arc<Mutex<Vec<Option<String>>>> = Arc::default();
        let app = Router::new().route(
            "/cover.jpg",
            get({
                let seen = seen.clone();
                move |headers: HeaderMap| {
                    let seen = seen.clone();
                    async move {
                        seen.lock().unwrap().push(
                            headers
                                .get(header::REFERER)
                                .and_then(|v| v.to_str().ok())
                                .map(str::to_string),
                        );
                        // 上游没标 content-type，靠文件头识别
                        Bytes::from_static(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10])
                    }
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let proxy = ImageProxy::new();
        let url = format!("http://{addr}/cover.jpg");
        let referer = "https://live.bilibili.com/1";
        let first = proxy.fetch(&url, referer, LiveImage::Cover).await.unwrap();
        assert_eq!(first.content_type, "image/jpeg");
        assert_eq!(first.body.len(), 6);
        let second = proxy.fetch(&url, referer, LiveImage::Cover).await.unwrap();
        assert_eq!(second.body, first.body);
        assert_eq!(
            *seen.lock().unwrap(),
            vec![Some(referer.to_string())],
            "第二次应命中缓存，上游只被请求一次，且带直播间页面作 Referer"
        );

        let response = image_response(
            &first.content_type,
            first.body.clone(),
            "private, max-age=300",
        );
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/jpeg"
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "private, max-age=300"
        );
        assert_eq!(
            to_bytes(response.into_body(), usize::MAX).await.unwrap(),
            first.body
        );
    }

    #[tokio::test]
    async fn non_image_upstream_responses_are_not_forwarded() {
        let app = Router::new()
            .route("/html", get(|| async { "<html>403</html>" }))
            .route(
                "/missing",
                get(|| async { (StatusCode::FORBIDDEN, "denied") }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let proxy = ImageProxy::new();
        let err = proxy
            .fetch(
                &format!("http://{addr}/html"),
                "https://live.bilibili.com/1",
                LiveImage::Cover,
            )
            .await
            .unwrap_err();
        assert!(err.contains("不是图片"), "{err}");
        let err = proxy
            .fetch(
                &format!("http://{addr}/missing"),
                "https://live.bilibili.com/1",
                LiveImage::Cover,
            )
            .await
            .unwrap_err();
        assert!(err.contains("403"), "{err}");
    }

    #[test]
    fn cache_expires_and_evicts_the_oldest_entry_when_full() {
        let proxy = ImageProxy::new();
        let t0 = Instant::now();
        let image = |at: Instant| CachedImage {
            fetched_at: at,
            content_type: "image/png".into(),
            body: Bytes::from_static(b"x"),
        };
        proxy.store("a".into(), image(t0), t0);
        assert!(proxy.cached("a", t0 + Duration::from_secs(10)).is_some());
        assert!(proxy.cached("a", t0 + CACHE_TTL).is_none(), "到期后失效");

        for i in 0..MAX_CACHE_ENTRIES {
            let at = t0 + Duration::from_secs(i as u64);
            proxy.store(format!("u{i}"), image(at), at);
        }
        let now = t0 + Duration::from_secs(MAX_CACHE_ENTRIES as u64);
        proxy.store("newest".into(), image(now), now);
        let cache = proxy.cache.lock().unwrap();
        assert!(cache.len() <= MAX_CACHE_ENTRIES);
        assert!(!cache.contains_key("u0"), "最旧的条目被淘汰");
        assert!(cache.contains_key("newest"));
    }

    /// 头像接口：录制中首次请求会现场下载落盘；之后（包括不在录制时）直接读本地文件
    #[tokio::test]
    async fn avatar_endpoint_downloads_once_then_serves_the_local_file() {
        let hits = Arc::new(Mutex::new(0usize));
        let app = Router::new().route(
            "/face.png",
            get({
                let hits = hits.clone();
                move || {
                    let hits = hits.clone();
                    async move {
                        *hits.lock().unwrap() += 1;
                        (
                            [("content-type", "image/png")],
                            Bytes::from_static(&[
                                0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1,
                            ]),
                        )
                    }
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let dir = tempfile::tempdir().unwrap();
        let store = AvatarStore::new(dir.path());
        let url = format!("http://{addr}/face.png");
        let working = working_worker("", Some(&url)).await;

        let response = serve_avatar(&working, &store).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/png"
        );
        assert_eq!(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .len(),
            9
        );
        assert_eq!(*hits.lock().unwrap(), 1);

        // 第二次：命中本地文件，上游不再被请求；即使房间已不在录制也能读到
        let again = serve_avatar(&working, &store).await;
        let offline = serve_avatar(&idle_worker(), &store).await;
        assert_eq!(again.status(), StatusCode::OK);
        assert_eq!(
            offline.status(),
            StatusCode::OK,
            "本地已有文件时不在录制也能读到"
        );
        assert_eq!(*hits.lock().unwrap(), 1, "第二次不该再请求上游");
        assert!(dir.path().join("7.png").is_file());
        assert!(dir.path().join("7.json").is_file());
    }
}
