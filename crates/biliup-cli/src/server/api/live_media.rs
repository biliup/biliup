//! 正在录制的直播间封面 / 主播头像的同源图片代理。
//!
//! 页面直接 `<img src="https://i0.hdslb.com/...">` 会带上 biliup 自己的 Referer，
//! B 站等图片 CDN 对站外 Referer 返回 403。这里由服务端以直播间页面地址作 Referer 去取，
//! 再把字节转发给页面，并按图片 URL 做一份短暂的内存缓存。
//!
//! 只按主播 id 取 worker 里已经拿到的地址，不接受任意 URL，避免变成开放代理。
//! 路由注册在 `router()` 里，`--auth` 时与 `/v1/streamers` 同一道登录校验。

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

/// 图片在内存里的缓存时间。封面每场直播才变一次，头像更少变
const CACHE_TTL: Duration = Duration::from_secs(300);
/// 缓存条目上限（按图片 URL）。默认 pool1_size = 5，两张图一路，远用不到这个数
const MAX_CACHE_ENTRIES: usize = 64;
/// 单张图片上限；封面 / 头像通常几十到几百 KB，超出的不缓存也不转发
const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
const FETCH_TIMEOUT: Duration = Duration::from_secs(15);
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveImage {
    Cover,
    Avatar,
}

impl LiveImage {
    fn label(self) -> &'static str {
        match self {
            LiveImage::Cover => "封面",
            LiveImage::Avatar => "头像",
        }
    }

    /// B 站图片 CDN 的缩放后缀（`@宽w_高h_1c` = 等比裁切填满）。
    /// 卡片上的封面按 16:9 缩略图显示，头像只有 22px，用不到原图。
    fn bfs_size_suffix(self) -> &'static str {
        match self {
            LiveImage::Cover => "@640w_360h_1c.jpg",
            LiveImage::Avatar => "@128w_128h_1c.jpg",
        }
    }
}

/// 向上游请求时实际使用的地址。
///
/// B 站的 `*.hdslb.com` 原图可能有上千万像素、几 MB（实测 11520×8640、4 MB），
/// 直接转发给页面既慢又占缓存，借 CDN 的缩放后缀取一张缩略图；已经带后缀的地址不再处理。
/// 其它平台原样使用。
fn upstream_url(url: &str, kind: LiveImage) -> String {
    let is_bfs = url::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(|host| host.ends_with(".hdslb.com")))
        .unwrap_or(false);
    if is_bfs && !url.contains('@') && !url.contains('?') {
        format!("{url}{}", kind.bfs_size_suffix())
    } else {
        url.to_string()
    }
}

#[derive(Debug, Clone)]
struct CachedImage {
    fetched_at: Instant,
    content_type: String,
    body: Bytes,
}

/// 封面 / 头像的抓取客户端与内存缓存，进程内唯一，放在 `ServiceRegister` 里。
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
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .timeout(FETCH_TIMEOUT)
            .build()
            .expect("reqwest client for image proxy");
        Self {
            client,
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
        let image = fetch_image(&self.client, &url, referer).await?;
        self.store(url, image.clone(), Instant::now());
        Ok(image)
    }
}

/// `GET /v1/streamers/{id}/cover`
pub async fn get_live_cover(
    State(managers): State<Arc<DownloadManager>>,
    State(proxy): State<Arc<ImageProxy>>,
    Path(id): Path<i64>,
) -> Response {
    serve(managers.get_room_by_id(id).await, &proxy, LiveImage::Cover).await
}

/// `GET /v1/streamers/{id}/avatar`
pub async fn get_live_avatar(
    State(managers): State<Arc<DownloadManager>>,
    State(proxy): State<Arc<ImageProxy>>,
    Path(id): Path<i64>,
) -> Response {
    serve(managers.get_room_by_id(id).await, &proxy, LiveImage::Avatar).await
}

async fn serve(worker: Option<Arc<Worker>>, proxy: &ImageProxy, kind: LiveImage) -> Response {
    let (url, referer) = match resolve_target(worker.as_deref(), kind) {
        Ok(target) => target,
        Err(rejection) => return rejection.into_response(),
    };
    match proxy.fetch(&url, &referer, kind).await {
        Ok(image) => image_response(&image),
        Err(reason) => {
            warn!(url, referer, reason, "拉取直播间{}失败", kind.label());
            (
                StatusCode::BAD_GATEWAY,
                format!("拉取{}失败: {reason}", kind.label()),
            )
                .into_response()
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

async fn fetch_image(client: &Client, url: &str, referer: &str) -> Result<CachedImage, String> {
    let mut request = client.get(url);
    if let Ok(referer) = HeaderValue::from_str(referer) {
        request = request.header(header::REFERER, referer);
    }
    let response = request.send().await.map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("上游返回 {status}"));
    }
    if response
        .content_length()
        .is_some_and(|len| len > MAX_IMAGE_BYTES as u64)
    {
        return Err(format!("图片超过 {MAX_IMAGE_BYTES} 字节"));
    }
    let upstream_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let body = response.bytes().await.map_err(|e| e.to_string())?;
    if body.len() > MAX_IMAGE_BYTES {
        return Err(format!("图片超过 {MAX_IMAGE_BYTES} 字节"));
    }
    let content_type = image_content_type(upstream_type.as_deref(), &body)
        .ok_or_else(|| "上游返回的不是图片".to_string())?;
    Ok(CachedImage {
        fetched_at: Instant::now(),
        content_type,
        body,
    })
}

/// 上游标了 `image/*` 就用上游的；否则按文件头识别常见格式，都认不出就拒绝转发。
fn image_content_type(upstream: Option<&str>, body: &[u8]) -> Option<String> {
    if let Some(upstream) = upstream {
        let essence = upstream.split(';').next().unwrap_or_default().trim();
        if essence.starts_with("image/") {
            return Some(essence.to_string());
        }
    }
    sniff_image_type(body).map(str::to_string)
}

fn sniff_image_type(body: &[u8]) -> Option<&'static str> {
    if body.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if body.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("image/png")
    } else if body.starts_with(b"GIF87a") || body.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if body.len() >= 12 && &body[0..4] == b"RIFF" && &body[8..12] == b"WEBP" {
        Some("image/webp")
    } else if body.len() >= 12 && &body[4..8] == b"ftyp" && &body[8..12] == b"avif" {
        Some("image/avif")
    } else {
        None
    }
}

fn image_response(image: &CachedImage) -> Response {
    let mut response = image.body.clone().into_response();
    let headers = response.headers_mut();
    if let Ok(value) = HeaderValue::from_str(&image.content_type) {
        headers.insert(header::CONTENT_TYPE, value);
    }
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=300"),
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

    async fn working_worker(cover: &str, avatar: Option<&str>) -> Worker {
        let worker = Worker::new(
            streamer("https://live.bilibili.com/1"),
            None,
            Arc::new(RwLock::new(Config::default())),
            Default::default(),
        );
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

        let idle = Worker::new(
            streamer("https://live.bilibili.com/1"),
            None,
            Arc::new(RwLock::new(Config::default())),
            Default::default(),
        );
        assert_eq!(
            resolve_target(Some(&idle), LiveImage::Cover)
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
    async fn proxy_fetches_with_the_room_page_as_referer_and_caches_by_url() {
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

        let response = image_response(&first);
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
                LiveImage::Avatar,
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

    #[test]
    fn bilibili_bfs_urls_get_a_thumbnail_suffix_and_others_pass_through() {
        assert_eq!(
            upstream_url(
                "https://i0.hdslb.com/bfs/live/user_cover/abc.jpg",
                LiveImage::Cover
            ),
            "https://i0.hdslb.com/bfs/live/user_cover/abc.jpg@640w_360h_1c.jpg"
        );
        assert_eq!(
            upstream_url("https://i1.hdslb.com/bfs/face/def.jpg", LiveImage::Avatar),
            "https://i1.hdslb.com/bfs/face/def.jpg@128w_128h_1c.jpg"
        );
        // 已带缩放参数的不重复追加
        assert_eq!(
            upstream_url(
                "https://i0.hdslb.com/bfs/live/abc.jpg@100w_100h.jpg",
                LiveImage::Cover
            ),
            "https://i0.hdslb.com/bfs/live/abc.jpg@100w_100h.jpg"
        );
        // 其它平台（含签名 query 的抖音、虎牙截图）原样使用
        for url in [
            "https://p3-webcast.douyinpic.com/img/x~tplv-obj.image",
            "https://tx-live-cover.msstatic.com/huyalive/a/20260922.jpg?sign=abc",
            "https://rpic.douyucdn.cn/asrpic/260922/1_src.avif/dy4",
        ] {
            assert_eq!(upstream_url(url, LiveImage::Cover), url);
        }
    }

    #[test]
    fn content_type_prefers_upstream_image_type_then_sniffs() {
        assert_eq!(
            image_content_type(Some("image/webp; charset=binary"), b"junk").as_deref(),
            Some("image/webp")
        );
        assert_eq!(
            image_content_type(
                Some("application/octet-stream"),
                &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
            )
            .as_deref(),
            Some("image/png")
        );
        let avif = [
            0u8, 0, 0, 0x1c, b'f', b't', b'y', b'p', b'a', b'v', b'i', b'f',
        ];
        assert_eq!(
            image_content_type(None, &avif).as_deref(),
            Some("image/avif")
        );
        assert_eq!(image_content_type(Some("text/html"), b"<html>"), None);
    }
}
