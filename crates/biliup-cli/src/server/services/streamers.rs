use crate::server::core::download_manager::DownloadManager;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::{Stage, WorkerStatus};
use crate::server::infrastructure::models::live_streamer::{InsertLiveStreamer, LiveStreamer};
use crate::server::infrastructure::repositories::{del_streamer, get_upload_config};
use crate::server::infrastructure::service_register::ServiceRegister;
use error_stack::{Report, ResultExt};
use ormlite::{Insert, Model};
use tracing::info;

/// 新增主播失败的原因
#[derive(Debug)]
pub enum AddStreamerError {
    /// 没有插件支持这个直播间地址。主播已经写入数据库，只是没有加入监控
    UnsupportedUrl,
    /// 落库或调度出错
    Internal(Report<AppError>),
}

impl From<Report<AppError>> for AddStreamerError {
    fn from(report: Report<AppError>) -> Self {
        AddStreamerError::Internal(report)
    }
}

/// 新增主播并加入监控
pub async fn add_streamer(
    services: &ServiceRegister,
    streamer: InsertLiveStreamer,
) -> Result<LiveStreamer, AddStreamerError> {
    let url = &streamer.url.clone();
    let live_streamers = streamer
        .insert(&services.pool)
        .await
        .change_context(AppError::Unknown)?;
    let upload_config = get_upload_config(&services.pool, live_streamers.id).await?;
    let Some(_) = services
        .managers
        .add_room(services.worker(live_streamers.clone(), upload_config))
        .await
    else {
        info!("not supported url: {}", url);
        return Err(AddStreamerError::UnsupportedUrl);
    };

    info!(url = url, "successfully inserted new live streamers");
    Ok(live_streamers)
}

/// 整体覆盖保存主播，并用新设置重建它的监控
pub async fn update_streamer(
    services: &ServiceRegister,
    streamer: LiveStreamer,
) -> AppResult<LiveStreamer> {
    let streamer = streamer
        .update_all_fields(&services.pool)
        .await
        .change_context(AppError::Unknown)?;

    let id = streamer.id;
    services.managers.del_room(id).await;

    let upload_config = get_upload_config(&services.pool, id).await?;

    services
        .managers
        .add_room(services.worker(streamer.clone(), upload_config))
        .await
        .ok_or(AppError::Unknown)?;

    info!(id = id, "successfully update live streamers");
    Ok(streamer)
}

/// 停止监控并删除主播，返回删掉的主播
pub async fn delete_streamer(
    pool: &ConnectionPool,
    managers: &DownloadManager,
    id: i64,
) -> AppResult<LiveStreamer> {
    managers.del_room(id).await;

    let live_streamers = del_streamer(pool, id).await?;
    info!(workers=?live_streamers, "successfully inserted new live streamers");
    Ok(live_streamers)
}

/// 暂停或恢复主播的监控：暂停中的恢复，其他状态（包括正在录制）一律暂停。
/// 主播不在监控里时什么也不做。
pub async fn toggle_pause(managers: &DownloadManager, id: i64) {
    let worker = managers.get_room_by_id(id).await;
    if let Some(w) = worker {
        let worker_status = w.downloader_status.read().unwrap().clone();
        match worker_status {
            WorkerStatus::Working(_) => {
                w.change_status(Stage::Download, WorkerStatus::Pause).await;
                info!(url=?&w.live_streamer.url, "successfully pause live streamers");
                managers.make_waker(id).await;
            }
            WorkerStatus::Pause => {
                w.change_status(Stage::Download, WorkerStatus::Idle).await;
                managers.wake_waker(id).await;
                info!(url=?&w.live_streamer.url, "successfully start live streamers");
            }
            WorkerStatus::Pending => {
                w.change_status(Stage::Download, WorkerStatus::Pause).await;
                managers.make_waker(id).await;
                info!(url=?&w.live_streamer.url, "successfully pause live streamers");
            }
            WorkerStatus::Idle => {
                w.change_status(Stage::Download, WorkerStatus::Pause).await;
                managers.make_waker(id).await;
                info!(url=?&w.live_streamer.url, "successfully pause live streamers");
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::api::live_media::ImageProxy;
    use crate::server::common::system_stats::SystemMonitor;
    use crate::server::config::Config;
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use async_trait::async_trait;
    use biliup::downloader::live::{LivePlugin, LiveRequest, LiveResult, LiveStatus};
    use std::sync::{Arc, RwLock};
    use std::time::Duration;
    use tokio::sync::mpsc;
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::reload;

    /// 只认 `https://stuck.example/` 的平台；检测时把房间地址报给测试，然后一直不返回，
    /// 房间停在「检测中」，状态不会被监控循环改动
    struct StuckPlatform {
        probed: mpsc::UnboundedSender<String>,
    }

    #[async_trait]
    impl LivePlugin for StuckPlatform {
        fn name(&self) -> &'static str {
            "stuck"
        }

        fn matches(&self, url: &str) -> bool {
            url.starts_with("https://stuck.example/")
        }

        async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
            let _ = self.probed.send(request.url);
            std::future::pending().await
        }
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        services: ServiceRegister,
        probed: mpsc::UnboundedReceiver<String>,
    }

    /// 只装了 [`StuckPlatform`] 的服务，不会向任何真实平台发请求
    async fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let config = Config::default();
        let managers = DownloadManager::new(config.pool1_size, config.pool2_size, pool.clone());
        let (probed_tx, probed) = mpsc::unbounded_channel();
        managers
            .add_plugin(Arc::new(StuckPlatform { probed: probed_tx }))
            .await;
        let (_layer, log_handle) = reload::Layer::new(EnvFilter::new("info"));
        let services = ServiceRegister {
            pool,
            managers: Arc::new(managers),
            config: Arc::new(RwLock::new(config)),
            client: Default::default(),
            log_handle,
            image_proxy: Arc::new(ImageProxy::new()),
            system: SystemMonitor::spawn(),
        };
        Fixture {
            _dir: dir,
            services,
            probed,
        }
    }

    fn insert(url: &str) -> InsertLiveStreamer {
        serde_json::from_value(serde_json::json!({ "url": url, "remark": "原备注" })).unwrap()
    }

    async fn saved(pool: &ConnectionPool, id: i64) -> Option<LiveStreamer> {
        LiveStreamer::select()
            .where_("id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn adding_a_streamer_saves_it_and_starts_monitoring() {
        let mut f = fixture().await;
        let added = add_streamer(&f.services, insert("https://stuck.example/1"))
            .await
            .expect("支持的地址应添加成功");

        assert_eq!(
            saved(&f.services.pool, added.id).await.unwrap().url,
            "https://stuck.example/1"
        );
        let room = f.services.managers.get_room_by_id(added.id).await;
        assert_eq!(room.unwrap().live_streamer.url, "https://stuck.example/1");
        let probed = tokio::time::timeout(Duration::from_secs(5), f.probed.recv())
            .await
            .expect("新房间应很快轮到检测");
        assert_eq!(probed.as_deref(), Some("https://stuck.example/1"));
    }

    /// 没有插件支持的地址报 `UnsupportedUrl`；主播仍留在库里，只是不加入监控
    #[tokio::test]
    async fn an_unsupported_url_is_saved_but_not_monitored() {
        let f = fixture().await;
        let result = add_streamer(&f.services, insert("https://unknown.example/1")).await;

        assert!(
            matches!(result, Err(AddStreamerError::UnsupportedUrl)),
            "{result:?}"
        );
        let rows = LiveStreamer::select()
            .fetch_all(&f.services.pool)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].url, "https://unknown.example/1");
        assert!(f.services.managers.get_rooms().await.is_empty());
    }

    #[tokio::test]
    async fn updating_a_streamer_rebuilds_its_room_with_the_new_settings() {
        let f = fixture().await;
        let added = add_streamer(&f.services, insert("https://stuck.example/1"))
            .await
            .unwrap();

        let edited = LiveStreamer {
            remark: "新备注".to_string(),
            ..added.clone()
        };
        let updated = update_streamer(&f.services, edited).await.unwrap();

        assert_eq!(updated.remark, "新备注");
        assert_eq!(
            saved(&f.services.pool, added.id).await.unwrap().remark,
            "新备注"
        );
        let rooms = f.services.managers.get_rooms().await;
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].live_streamer.remark, "新备注");
    }

    #[tokio::test]
    async fn deleting_a_streamer_stops_monitoring_and_removes_it() {
        let f = fixture().await;
        let added = add_streamer(&f.services, insert("https://stuck.example/1"))
            .await
            .unwrap();

        let deleted = delete_streamer(&f.services.pool, &f.services.managers, added.id)
            .await
            .unwrap();

        assert_eq!(deleted.id, added.id);
        assert!(saved(&f.services.pool, added.id).await.is_none());
        assert!(f.services.managers.get_rooms().await.is_empty());
        assert!(
            delete_streamer(&f.services.pool, &f.services.managers, added.id)
                .await
                .is_err(),
            "删除不存在的主播应报错"
        );
    }

    #[tokio::test]
    async fn toggling_pause_alternates_between_paused_and_idle() {
        let mut f = fixture().await;
        let added = add_streamer(&f.services, insert("https://stuck.example/1"))
            .await
            .unwrap();
        // 等房间进入检测，之后监控循环卡在这次检测里，不会再改它的状态
        tokio::time::timeout(Duration::from_secs(5), f.probed.recv())
            .await
            .expect("新房间应很快轮到检测");
        let managers = &f.services.managers;
        let room = managers.get_room_by_id(added.id).await.unwrap();
        let status = || room.downloader_status.read().unwrap().clone();
        assert!(matches!(status(), WorkerStatus::Pending));

        toggle_pause(managers, added.id).await;
        assert!(matches!(status(), WorkerStatus::Pause));

        toggle_pause(managers, added.id).await;
        assert!(matches!(status(), WorkerStatus::Idle));

        toggle_pause(managers, added.id).await;
        assert!(matches!(status(), WorkerStatus::Pause));

        // 不在监控里的主播什么也不做
        toggle_pause(managers, added.id + 1).await;
    }
}
