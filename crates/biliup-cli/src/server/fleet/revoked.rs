//! 被控制面移除（吊销）的节点：原受管主播转成本地主播并暂停，暂停跨重启保持，直到管理员在本机界面上恢复。
//!
//! 被移除的节点，房间多半已被改派给别的节点，它接着录就会与新节点重复录制、重复投稿。节点发现自己被吊销
//! （连接以 `revoked` 关闭，或重连时 relay 以 `revoked` 拒绝）时，把此刻托管的主播记进
//! `data/fleet-revoked.json` 并暂停。本机的暂停只在内存里，所以每次启动按这份清单重新暂停；
//! 管理员恢复某个主播、删掉它、或点「全部恢复」/「知道了」时从清单里拿掉，清单空了删文件。
//!
//! 只有真的被吊销过才会有这个文件；`biliup node leave`（节点主人主动离开）不经过这里，房间照常录。
//! 协议不变：控制面不知道这份清单，也不需要知道。

use super::reconcile::apply_paused;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::WorkerStatus;
use crate::server::infrastructure::service_register::ServiceRegister;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tracing::{info, warn};

pub const REVOKED_FILE_NAME: &str = "fleet-revoked.json";
const REVOKED_FILE_VERSION: u32 = 1;
/// 保存主播的请求体上限，与 `guard` 一致
const BODY_LIMIT: usize = 2 * 1024 * 1024;

/// `data/fleet-revoked.json`
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RevokedFile {
    pub version: u32,
    /// 界面上显示的控制面名字（同 `Managed::controller`）
    pub controller: String,
    /// 最近一次发现被吊销的时刻（毫秒）
    pub revoked_at: i64,
    /// 本地主播 id → 直播间地址；启动时地址对不上（id 已换了主播）的不再暂停
    #[serde(default)]
    pub streamers: BTreeMap<i64, String>,
}

pub fn revoked_path(node_file: &Path) -> PathBuf {
    node_file.with_file_name(REVOKED_FILE_NAME)
}

fn load(path: &Path) -> Option<RevokedFile> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<RevokedFile>(&text) {
        Ok(file) if file.version == REVOKED_FILE_VERSION => Some(file),
        Ok(file) => {
            warn!(
                version = file.version,
                "{} has an unsupported version, ignoring it",
                path.display()
            );
            None
        }
        Err(e) => {
            warn!(error = %e, "{} is not valid, ignoring it", path.display());
            None
        }
    }
}

fn save(path: &Path, file: &RevokedFile) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).change_context(AppError::Unknown)?;
    }
    let tmp = path.with_extension("json.tmp");
    let body = serde_json::to_vec_pretty(file).change_context(AppError::Unknown)?;
    {
        use std::io::Write;
        let mut out = std::fs::File::create(&tmp)
            .change_context(AppError::Unknown)
            .attach_with(|| format!("could not write {}", tmp.display()))?;
        out.write_all(&body).change_context(AppError::Unknown)?;
        out.sync_all().change_context(AppError::Unknown)?;
    }
    std::fs::rename(&tmp, path)
        .change_context(AppError::Unknown)
        .attach_with(|| format!("could not write {}", path.display()))
}

/// 被移除后等管理员确认恢复的主播；节点代理与 HTTP 层共享
pub struct Revoked {
    path: PathBuf,
    services: ServiceRegister,
    file: RwLock<Option<RevokedFile>>,
}

pub type RevokedHandle = Arc<Revoked>;

impl std::fmt::Debug for Revoked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Revoked")
            .field("path", &self.path)
            .field("file", &self.file)
            .finish_non_exhaustive()
    }
}

impl Revoked {
    /// 读清单（没有文件就是空的，不创建）
    pub fn load(path: PathBuf, services: ServiceRegister) -> Self {
        let file = if path.exists() { load(&path) } else { None };
        Revoked {
            path,
            services,
            file: RwLock::new(file),
        }
    }

    /// 单机与控制面进程：只有清单里还有主播时才要这一层
    pub async fn resume_if_present(
        path: PathBuf,
        services: &ServiceRegister,
    ) -> Option<RevokedHandle> {
        if !path.exists() {
            return None;
        }
        let revoked = Arc::new(Revoked::load(path, services.clone()));
        revoked.apply().await;
        (!revoked.is_empty()).then_some(revoked)
    }

    pub fn is_empty(&self) -> bool {
        self.file
            .read()
            .unwrap()
            .as_ref()
            .is_none_or(|file| file.streamers.is_empty())
    }

    pub fn contains(&self, id: i64) -> bool {
        self.file
            .read()
            .unwrap()
            .as_ref()
            .is_some_and(|file| file.streamers.contains_key(&id))
    }

    pub fn ids(&self) -> Vec<i64> {
        self.file
            .read()
            .unwrap()
            .as_ref()
            .map(|file| file.streamers.keys().copied().collect())
            .unwrap_or_default()
    }

    /// `/v1/me` 的 `fleet_revoked`；清单空时为 `None`
    pub fn view(&self) -> Option<Value> {
        let file = self.file.read().unwrap();
        let file = file.as_ref().filter(|file| !file.streamers.is_empty())?;
        Some(json!({
            "controller": file.controller,
            "revoked_at": file.revoked_at,
            "streamers": file.streamers.keys().collect::<Vec<_>>(),
        }))
    }

    /// 改清单并落盘；空了删文件
    fn update(&self, change: impl FnOnce(&mut Option<RevokedFile>)) {
        let mut file = self.file.write().unwrap();
        let before = file.clone();
        change(&mut file);
        if file.as_ref().is_some_and(|file| file.streamers.is_empty()) {
            *file = None;
        }
        if *file == before {
            return;
        }
        let result = match file.as_ref() {
            Some(file) => save(&self.path, file),
            None => match std::fs::remove_file(&self.path) {
                Ok(()) => {
                    info!(
                        "{} removed, no streamers wait for confirmation any more",
                        self.path.display()
                    );
                    Ok(())
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e) => Err(e).change_context(AppError::Unknown),
            },
        };
        if let Err(e) = result {
            warn!(error = ?e, "could not update {}", self.path.display());
        }
    }

    /// 发现被吊销：把此刻托管的主播记进清单（与已有的合并）并暂停。没有主播时什么也不写。
    pub async fn record(&self, controller: &str, streamers: BTreeMap<i64, String>, now: i64) {
        if streamers.is_empty() {
            return;
        }
        let ids: Vec<i64> = streamers.keys().copied().collect();
        self.update(|file| {
            let file = file.get_or_insert_with(|| RevokedFile {
                version: REVOKED_FILE_VERSION,
                ..RevokedFile::default()
            });
            file.controller = controller.to_string();
            file.revoked_at = now;
            file.streamers.extend(streamers);
        });
        for id in &ids {
            apply_paused(&self.services, *id, true).await;
        }
        info!(streamers = ?ids, "removed from the fleet, formerly managed streamers are paused until an administrator resumes them");
    }

    /// 启动时：清单里还在本机的主播重新暂停；已经不在（或 id 已换了主播）的拿掉
    pub async fn apply(&self) {
        let listed: Vec<(i64, String)> = self
            .file
            .read()
            .unwrap()
            .as_ref()
            .map(|file| file.streamers.clone().into_iter().collect())
            .unwrap_or_default();
        let mut gone = Vec::new();
        for (id, url) in listed {
            let current: Option<String> =
                sqlx::query_scalar("SELECT url FROM livestreamers WHERE id = ?")
                    .bind(id)
                    .fetch_optional(&self.services.pool)
                    .await
                    .ok()
                    .flatten();
            if current.as_deref() == Some(url.as_str()) {
                apply_paused(&self.services, id, true).await;
            } else {
                gone.push(id);
            }
        }
        if !gone.is_empty() {
            info!(streamers = ?gone, "streamers paused after the fleet removal are gone");
            self.remove(&gone);
        }
        let ids = self.ids();
        if !ids.is_empty() {
            warn!(streamers = ?ids, "这台机器曾被控制面移除，原受管主播保持暂停，在「直播管理」确认后手动恢复");
        }
    }

    pub fn remove(&self, ids: &[i64]) {
        self.update(|file| {
            if let Some(file) = file {
                for id in ids {
                    file.streamers.remove(id);
                }
            }
        });
    }

    /// 「全部恢复」：清单里的主播恢复监控，清空清单。返回恢复了的主播
    pub async fn resume_all(&self) -> Vec<i64> {
        let ids = self.ids();
        for id in &ids {
            apply_paused(&self.services, *id, false).await;
        }
        self.remove(&ids);
        if !ids.is_empty() {
            info!(streamers = ?ids, "streamers paused after the fleet removal resumed");
        }
        ids
    }

    /// 「知道了」：只清空清单，主播保持此刻的暂停状态（与本机其他主播一样，暂停不跨重启）
    pub fn dismiss(&self) {
        let ids = self.ids();
        self.remove(&ids);
    }

    async fn is_paused(&self, id: i64) -> bool {
        match self.services.managers.get_room_by_id(id).await {
            Some(worker) => matches!(
                *worker.downloader_status.read().unwrap(),
                WorkerStatus::Pause
            ),
            None => false,
        }
    }
}

enum Tracked {
    /// `PUT /v1/streamers/{id}/pause`
    Toggle(i64),
    /// `DELETE /v1/streamers/{id}`
    Delete(i64),
    /// `PUT /v1/streamers`：保存会重建监控，暂停随之丢失
    Save,
}

fn tracked(method: &Method, path: &str) -> Option<Tracked> {
    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();
    match (method.as_str(), segments.as_slice()) {
        ("PUT", ["v1", "streamers", id, "pause"]) => id.parse().ok().map(Tracked::Toggle),
        ("DELETE", ["v1", "streamers", id]) => id.parse().ok().map(Tracked::Delete),
        ("PUT", ["v1", "streamers"]) => Some(Tracked::Save),
        _ => None,
    }
}

/// 本机对清单里主播的操作：恢复（暂停开关拨到恢复）或删除时拿出清单；保存设置后重新暂停
pub async fn track(State(revoked): State<RevokedHandle>, request: Request, next: Next) -> Response {
    let Some(action) = tracked(request.method(), request.uri().path()) else {
        return next.run(request).await;
    };
    if revoked.is_empty() {
        return next.run(request).await;
    }
    let (request, saved) = match action {
        Tracked::Save => {
            let (parts, body) = request.into_parts();
            let Ok(bytes) = axum::body::to_bytes(body, BODY_LIMIT).await else {
                return StatusCode::PAYLOAD_TOO_LARGE.into_response();
            };
            let id = serde_json::from_slice::<Value>(&bytes)
                .ok()
                .and_then(|body| body.get("id").and_then(Value::as_i64));
            (Request::from_parts(parts, Body::from(bytes)), id)
        }
        _ => (request, None),
    };
    let response = next.run(request).await;
    if !response.status().is_success() {
        return response;
    }
    match action {
        Tracked::Toggle(id) if revoked.contains(id) && !revoked.is_paused(id).await => {
            info!(
                streamer = id,
                "streamer paused after the fleet removal resumed by hand"
            );
            revoked.remove(&[id]);
        }
        Tracked::Delete(id) if revoked.contains(id) => revoked.remove(&[id]),
        Tracked::Save => {
            if let Some(id) = saved.filter(|id| revoked.contains(*id)) {
                apply_paused(&revoked.services, id, true).await;
            }
        }
        _ => {}
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::config::Config;
    use crate::server::core::download_manager::DownloadManager;
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use crate::server::infrastructure::models::live_streamer::LiveStreamer;
    use crate::server::services::streamers::{
        add_streamer, delete_streamer, toggle_pause, update_streamer,
    };
    use axum::extract::Path as UrlPath;
    use axum::routing::{delete, put};
    use axum::{Json, Router};
    use biliup::downloader::live::{LivePlugin, LiveRequest, LiveResult, LiveStatus};
    use ormlite::Model;
    use tower::ServiceExt;
    use tracing_subscriber::{EnvFilter, reload};

    /// 只认 `https://stuck.example/` 的平台；检测一直不返回，不会向任何真实平台发请求
    struct StuckPlatform;

    #[async_trait::async_trait]
    impl LivePlugin for StuckPlatform {
        fn name(&self) -> &'static str {
            "stuck"
        }

        fn matches(&self, url: &str) -> bool {
            url.starts_with("https://stuck.example/")
        }

        async fn check_stream(&self, _request: LiveRequest) -> LiveResult<LiveStatus> {
            std::future::pending().await
        }
    }

    async fn services(dir: &Path) -> ServiceRegister {
        let pool = ConnectionManager::new_pool(dir.join("data.sqlite3").to_str().unwrap())
            .await
            .unwrap();
        let config = Config::default();
        let managers = DownloadManager::new(config.pool1_size, config.pool2_size, pool.clone());
        managers.add_plugin(Arc::new(StuckPlatform)).await;
        let (_layer, log_handle) = reload::Layer::new(EnvFilter::new("info"));
        ServiceRegister::new(pool, Arc::new(RwLock::new(config)), managers, log_handle).await
    }

    async fn streamer(services: &ServiceRegister, url: &str) -> LiveStreamer {
        add_streamer(
            services,
            serde_json::from_value(json!({ "url": url, "remark": "r" })).unwrap(),
        )
        .await
        .unwrap()
    }

    /// 与 `router.rs` 里同路径的处理函数做同样的事，外面套上 [`track`]
    fn app(services: &ServiceRegister, revoked: &RevokedHandle) -> Router {
        let (save, remove, toggle) = (services.clone(), services.clone(), services.clone());
        Router::new()
            .route(
                "/v1/streamers",
                put(move |Json(body): Json<LiveStreamer>| async move {
                    update_streamer(&save, body)
                        .await
                        .map(|_| StatusCode::OK)
                        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
                }),
            )
            .route(
                "/v1/streamers/{id}",
                delete(move |UrlPath(id): UrlPath<i64>| async move {
                    delete_streamer(&remove.pool, &remove.managers, id)
                        .await
                        .map(|_| StatusCode::OK)
                        .unwrap_or(StatusCode::NOT_FOUND)
                }),
            )
            .route(
                "/v1/streamers/{id}/pause",
                put(move |UrlPath(id): UrlPath<i64>| async move {
                    toggle_pause(&toggle.managers, id).await;
                    StatusCode::OK
                }),
            )
            .layer(axum::middleware::from_fn_with_state(revoked.clone(), track))
    }

    async fn send(app: &Router, method: Method, uri: &str, body: String) -> StatusCode {
        app.clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
    }

    #[tokio::test]
    async fn resuming_or_deleting_by_hand_takes_a_streamer_off_the_list() {
        let dir = tempfile::tempdir().unwrap();
        let services = services(dir.path()).await;
        let a = streamer(&services, "https://stuck.example/a").await;
        let b = streamer(&services, "https://stuck.example/b").await;
        let other = streamer(&services, "https://stuck.example/other").await;
        let path = dir.path().join(REVOKED_FILE_NAME);
        let revoked: RevokedHandle = Arc::new(Revoked::load(path.clone(), services.clone()));
        let app = app(&services, &revoked);
        // 没被移除过：什么都不拦、不写文件
        assert_eq!(
            send(
                &app,
                Method::PUT,
                &format!("/v1/streamers/{}/pause", other.id),
                String::new()
            )
            .await,
            StatusCode::OK
        );
        assert!(!path.exists());
        assert!(revoked.is_paused(other.id).await);

        revoked
            .record(
                "10.0.0.2",
                BTreeMap::from([(a.id, a.url.clone()), (b.id, b.url.clone())]),
                1,
            )
            .await;
        assert!(path.exists());
        assert!(revoked.is_paused(a.id).await && revoked.is_paused(b.id).await);

        // 改设置会重建监控：重新暂停，仍在清单里
        let body = serde_json::to_string(
            &LiveStreamer::select()
                .where_("id = ?")
                .bind(a.id)
                .fetch_one(&services.pool)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            send(&app, Method::PUT, "/v1/streamers", body).await,
            StatusCode::OK
        );
        assert!(revoked.is_paused(a.id).await);
        assert!(revoked.contains(a.id));

        // 手动恢复：拿出清单
        let toggle = format!("/v1/streamers/{}/pause", a.id);
        assert_eq!(
            send(&app, Method::PUT, &toggle, String::new()).await,
            StatusCode::OK
        );
        assert!(!revoked.is_paused(a.id).await);
        assert_eq!(revoked.ids(), [b.id]);
        // 之后再暂停 / 恢复与清单无关
        assert_eq!(
            send(&app, Method::PUT, &toggle, String::new()).await,
            StatusCode::OK
        );
        assert_eq!(revoked.ids(), [b.id]);

        // 删除失败不动清单；删掉了就拿出清单，空了删文件
        assert_eq!(
            send(&app, Method::DELETE, "/v1/streamers/999", String::new()).await,
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            send(
                &app,
                Method::DELETE,
                &format!("/v1/streamers/{}", b.id),
                String::new()
            )
            .await,
            StatusCode::OK
        );
        assert!(revoked.is_empty());
        assert!(revoked.view().is_none());
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn dismissing_clears_the_list_and_leaves_streamers_paused() {
        let dir = tempfile::tempdir().unwrap();
        let services = services(dir.path()).await;
        let a = streamer(&services, "https://stuck.example/a").await;
        let path = dir.path().join(REVOKED_FILE_NAME);
        let revoked = Revoked::load(path.clone(), services.clone());
        revoked
            .record("c", BTreeMap::from([(a.id, a.url.clone())]), 1)
            .await;
        revoked.dismiss();
        assert!(revoked.is_empty());
        assert!(!path.exists());
        assert!(revoked.is_paused(a.id).await);

        // 清单里的 id 已换了主播（地址对不上）：启动时不暂停，拿掉
        save(
            &path,
            &RevokedFile {
                version: REVOKED_FILE_VERSION,
                streamers: BTreeMap::from([(a.id, "https://stuck.example/old".to_string())]),
                ..RevokedFile::default()
            },
        )
        .unwrap();
        toggle_pause(&services.managers, a.id).await;
        let revoked = Revoked::resume_if_present(path.clone(), &services).await;
        assert!(revoked.is_none());
        assert!(!path.exists());
        assert!(!Revoked::load(path, services.clone()).is_paused(a.id).await);
    }
}
