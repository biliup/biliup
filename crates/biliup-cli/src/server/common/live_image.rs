//! 直播间封面 / 主播头像的抓取与落盘。
//!
//! 页面直接 `<img src="https://i0.hdslb.com/...">` 会带上 biliup 自己的 Referer，
//! B 站等图片 CDN 对站外 Referer 返回 403，所以都由服务端以直播间页面地址作 Referer 去取。
//! 封面每场直播都可能换，只在内存里短暂缓存（见 `api::live_media`）；
//! 头像几乎不变，开播时下载一次存到 `data/avatar/`，地址变了才重新下载。

use axum::http::{HeaderValue, header};
use bytes::Bytes;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{info, warn};

/// 单张图片上限；封面 / 头像通常几十到几百 KB，超出的不缓存、不落盘、不转发
pub const MAX_IMAGE_BYTES: usize = 4 * 1024 * 1024;
const FETCH_TIMEOUT: Duration = Duration::from_secs(15);
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36";
/// 头像落盘目录，与 `use_live_cover` 的 `data/cover` 并列
const AVATAR_DIR: &str = "data/avatar";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveImage {
    Cover,
    Avatar,
}

impl LiveImage {
    pub fn label(self) -> &'static str {
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
pub fn upstream_url(url: &str, kind: LiveImage) -> String {
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

/// 图片抓取用的 HTTP 客户端：浏览器 UA、15 秒总超时
pub fn image_client() -> Client {
    Client::builder()
        .user_agent(USER_AGENT)
        .timeout(FETCH_TIMEOUT)
        .build()
        .expect("reqwest client for live images")
}

#[derive(Debug, Clone)]
pub struct FetchedImage {
    pub content_type: String,
    pub body: Bytes,
}

/// 带 Referer 拉一张图。非 2xx、超过大小上限、上游不是图片都算失败。
pub async fn fetch_image(
    client: &Client,
    url: &str,
    referer: &str,
) -> Result<FetchedImage, String> {
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
    Ok(FetchedImage { content_type, body })
}

/// 上游标了 `image/*` 就用上游的；否则按文件头识别常见格式，都认不出就拒绝。
pub fn image_content_type(upstream: Option<&str>, body: &[u8]) -> Option<String> {
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

/// 文件扩展名按 Content-Type 定，认不出的存成 `.img`（响应时仍按记录的 Content-Type 返回）
fn extension_for(content_type: &str) -> &'static str {
    match content_type {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/avif" => "avif",
        _ => "img",
    }
}

/// 已落盘的头像：图片文件旁有一份同名 `.json` 记录来源地址与类型，
/// 服务重启后据此判断要不要重新下载。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredAvatar {
    /// 平台给的原始地址（不含 B 站缩放后缀），变了才重新下载
    pub url: String,
    pub content_type: String,
    /// 图片文件名（相对头像目录）
    pub file: String,
}

/// 主播头像的本地存储：`<dir>/<主播 id>.<ext>` + `<dir>/<主播 id>.json`。
pub struct AvatarStore {
    dir: PathBuf,
    client: Client,
    /// 同一时刻只让一个下载在写盘，避免开播时的预下载和页面首次请求撞车
    write_lock: Mutex<()>,
}

/// 进程内共用的头像存储，录制流程与 Web 接口都用它。
pub fn avatar_store() -> &'static AvatarStore {
    static STORE: LazyLock<AvatarStore> = LazyLock::new(|| AvatarStore::new(AVATAR_DIR));
    &STORE
}

impl AvatarStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            client: image_client(),
            write_lock: Mutex::new(()),
        }
    }

    fn meta_path(&self, id: i64) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    /// 已落盘且文件确实存在的头像。
    pub async fn load(&self, id: i64) -> Option<StoredAvatar> {
        let text = tokio::fs::read_to_string(self.meta_path(id)).await.ok()?;
        let stored: StoredAvatar = serde_json::from_str(&text).ok()?;
        tokio::fs::metadata(self.dir.join(&stored.file))
            .await
            .ok()
            .filter(|meta| meta.is_file())?;
        Some(stored)
    }

    pub fn image_path(&self, stored: &StoredAvatar) -> PathBuf {
        self.dir.join(&stored.file)
    }

    /// 确保本地有 `url` 对应的头像：已存且地址相同就直接用；否则带 Referer 下载后写盘。
    /// 下载失败时返回错误，磁盘上的旧文件原样保留。
    pub async fn ensure(&self, id: i64, url: &str, referer: &str) -> Result<StoredAvatar, String> {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err("头像地址不是 http(s)".to_string());
        }
        if let Some(stored) = self.load(id).await
            && stored.url == url
        {
            return Ok(stored);
        }
        let _guard = self.write_lock.lock().await;
        // 拿到锁后再看一次，前一个持锁者可能刚刚下完同一张
        if let Some(stored) = self.load(id).await
            && stored.url == url
        {
            return Ok(stored);
        }
        let image =
            fetch_image(&self.client, &upstream_url(url, LiveImage::Avatar), referer).await?;
        let stored = StoredAvatar {
            url: url.to_string(),
            content_type: image.content_type.clone(),
            file: format!("{id}.{}", extension_for(&image.content_type)),
        };
        let previous = self.load(id).await;
        self.write(id, &stored, &image.body)
            .await
            .map_err(|e| format!("写入头像失败: {e}"))?;
        if let Some(previous) = previous
            && previous.file != stored.file
        {
            let _ = tokio::fs::remove_file(self.dir.join(previous.file)).await;
        }
        info!(id, url, file = %self.image_path(&stored).display(), "主播头像已保存到本地");
        Ok(stored)
    }

    /// 先写临时文件再改名，避免页面读到半张图或半份 json
    async fn write(&self, id: i64, stored: &StoredAvatar, body: &[u8]) -> std::io::Result<()> {
        tokio::fs::create_dir_all(&self.dir).await?;
        write_atomically(&self.image_path(stored), body).await?;
        write_atomically(
            &self.meta_path(id),
            serde_json::to_string_pretty(stored)
                .map_err(std::io::Error::other)?
                .as_bytes(),
        )
        .await
    }
}

async fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
    ));
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(&tmp, path).await
}

/// 开播 / 换流时在后台把头像存到本地。失败只记日志，不影响录制；页面请求时还会再试一次。
pub fn spawn_avatar_download(id: i64, url: Option<String>, referer: String) {
    let Some(url) = url else { return };
    tokio::spawn(async move {
        if let Err(reason) = avatar_store().ensure(id, &url, &referer).await {
            warn!(id, url, reason, "下载主播头像失败");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;
    use axum::{Router, http::HeaderMap};
    use std::sync::{Arc, Mutex as StdMutex};

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

    /// (请求路径, Referer) 记录
    type Seen = Arc<StdMutex<Vec<(String, Option<String>)>>>;

    /// 一个记下每次请求 Referer 的本地图片服务；`/broken` 返回 403
    async fn image_server() -> (String, Seen) {
        let seen: Seen = Arc::default();
        let record = {
            let seen = seen.clone();
            move |path: &str, headers: &HeaderMap| {
                seen.lock().unwrap().push((
                    path.to_string(),
                    headers
                        .get(header::REFERER)
                        .and_then(|v| v.to_str().ok())
                        .map(str::to_string),
                ));
            }
        };
        let jpeg = Bytes::from_static(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46]);
        let png = Bytes::from_static(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0]);
        let app = Router::new()
            .route(
                "/a.jpg",
                get({
                    let record = record.clone();
                    let jpeg = jpeg.clone();
                    move |headers: HeaderMap| {
                        let (record, jpeg) = (record.clone(), jpeg.clone());
                        async move {
                            record("/a.jpg", &headers);
                            jpeg
                        }
                    }
                }),
            )
            .route(
                "/b.png",
                get({
                    let record = record.clone();
                    move |headers: HeaderMap| {
                        let (record, png) = (record.clone(), png.clone());
                        async move {
                            record("/b.png", &headers);
                            ([("content-type", "image/png")], png)
                        }
                    }
                }),
            )
            .route(
                "/broken",
                get(move |headers: HeaderMap| {
                    let record = record.clone();
                    async move {
                        record("/broken", &headers);
                        (axum::http::StatusCode::FORBIDDEN, "denied")
                    }
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (format!("http://{addr}"), seen)
    }

    #[tokio::test]
    async fn avatar_is_downloaded_once_with_referer_and_survives_a_restart() {
        let (base, seen) = image_server().await;
        let dir = tempfile::tempdir().unwrap();
        let store = AvatarStore::new(dir.path());
        let referer = "https://live.bilibili.com/1";

        let stored = store
            .ensure(7, &format!("{base}/a.jpg"), referer)
            .await
            .unwrap();
        assert_eq!(stored.file, "7.jpg");
        assert_eq!(stored.content_type, "image/jpeg");
        assert!(dir.path().join("7.jpg").is_file());
        assert!(dir.path().join("7.json").is_file());
        assert_eq!(
            std::fs::read(dir.path().join("7.jpg")).unwrap()[..3],
            [0xFF, 0xD8, 0xFF]
        );

        // 同一地址再来一次：不请求上游
        let again = store
            .ensure(7, &format!("{base}/a.jpg"), referer)
            .await
            .unwrap();
        assert_eq!(again, stored);
        assert_eq!(
            *seen.lock().unwrap(),
            vec![("/a.jpg".to_string(), Some(referer.to_string()))]
        );

        // 「重启」：新的 store 实例读同一目录
        let reopened = AvatarStore::new(dir.path());
        assert_eq!(reopened.load(7).await, Some(stored.clone()));
        assert_eq!(reopened.load(8).await, None);

        // 地址变了才重新下载；类型变了旧文件被清掉
        let switched = reopened
            .ensure(7, &format!("{base}/b.png"), referer)
            .await
            .unwrap();
        assert_eq!(switched.file, "7.png");
        assert!(dir.path().join("7.png").is_file());
        assert!(!dir.path().join("7.jpg").exists(), "旧扩展名的文件应被删除");
        assert_eq!(seen.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn a_failed_download_keeps_the_previous_avatar() {
        let (base, _seen) = image_server().await;
        let dir = tempfile::tempdir().unwrap();
        let store = AvatarStore::new(dir.path());
        let ok = store
            .ensure(3, &format!("{base}/a.jpg"), "https://live.bilibili.com/1")
            .await
            .unwrap();

        let err = store
            .ensure(3, &format!("{base}/broken"), "https://live.bilibili.com/1")
            .await
            .unwrap_err();
        assert!(err.contains("403"), "{err}");
        assert_eq!(store.load(3).await, Some(ok), "失败时保留旧头像");

        assert!(
            store
                .ensure(3, "data:image/png;base64,AAAA", "x")
                .await
                .is_err()
        );
    }
}
