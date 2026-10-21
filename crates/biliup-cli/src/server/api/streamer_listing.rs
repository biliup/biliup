//! Bounded room listings. Neither endpoint reads the recording directory or full session history.
use super::{access::Caller, endpoints::streamer_responses};
use crate::server::{
    config::Config,
    core::download_manager::DownloadManager,
    infrastructure::{
        connection_pool::ConnectionPool,
        context::WorkerStatus,
        dto::LiveStreamerResponse,
        models::{StreamerInfo, live_streamer::LiveStreamer},
    },
};
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ormlite::Model;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    sync::{Arc, RwLock},
};

#[derive(Default, Deserialize)]
pub struct PageQuery {
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub search: Option<String>,
    pub platform: Option<String>,
}

#[derive(Serialize)]
pub struct StreamerPage {
    items: Vec<LiveStreamerResponse>,
    infos: Vec<StreamerInfo>,
    total: usize,
    page: usize,
    page_size: usize,
    platforms: BTreeSet<String>,
}

fn database_error(e: impl std::fmt::Display) -> Response {
    tracing::error!(error = %e, "读取直播间列表失败");
    (StatusCode::INTERNAL_SERVER_ERROR, "读取直播间列表失败").into_response()
}

fn platform_name(url: &str) -> String {
    let lower = url.to_lowercase();
    for (domains, name) in [
        (&["bilibili.com", "b23.tv"][..], "哔哩哔哩"),
        (&["huya.com"][..], "虎牙直播"),
        (&["douyu.com"][..], "斗鱼直播"),
        (&["youtube.com", "youtu.be"][..], "YouTube"),
        (&["twitch.tv"][..], "Twitch"),
        (&["cc.163.com"][..], "CC直播"),
        (&["kuaishou.com"][..], "快手直播"),
        (&["douyin.com"][..], "抖音直播"),
    ] {
        if domains.iter().any(|domain| lower.contains(domain)) {
            return name.into();
        }
    }
    url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_owned))
        .unwrap_or_else(|| "直播源".into())
}

async fn selected_rooms(pool: &ConnectionPool, ids: &[i64]) -> Result<Vec<LiveStreamer>, Response> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut rows = LiveStreamer::select()
        .where_("id IN (SELECT value FROM json_each(?))")
        .bind(serde_json::json!(ids).to_string())
        .fetch_all(pool)
        .await
        .map_err(database_error)?;
    rows.sort_by_key(|row| row.id);
    Ok(rows)
}

async fn latest_infos(pool: &ConnectionPool, ids: &[i64]) -> Result<Vec<StreamerInfo>, Response> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    StreamerInfo::select().where_(
        "id IN (SELECT (SELECT s.id FROM stream_sessions s WHERE s.url = l.url AND s.kind = 'source' ORDER BY s.date DESC, s.id DESC LIMIT 1) FROM livestreamers l WHERE l.id IN (SELECT value FROM json_each(?)))"
    ).bind(serde_json::json!(ids).to_string()).fetch_all(pool).await.map_err(database_error)
}

pub async fn page(
    caller: Caller,
    State(pool): State<ConnectionPool>,
    State(managers): State<Arc<DownloadManager>>,
    State(config): State<Arc<RwLock<Config>>>,
    Query(query): Query<PageQuery>,
) -> Result<Json<StreamerPage>, Response> {
    // Only the small identity projection is used for matching and platform facets. Configs,
    // hooks, worker details, session history and images are loaded for the selected page only.
    let identities: Vec<(i64, String, String)> =
        sqlx::query_as("SELECT id,url,remark FROM livestreamers ORDER BY id")
            .fetch_all(&pool)
            .await
            .map_err(database_error)?;
    let platforms = identities
        .iter()
        .map(|(_, url, _)| platform_name(url))
        .collect();
    let search = query
        .search
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    let ids: Vec<_> = identities
        .into_iter()
        .filter(|(_, url, remark)| {
            (query
                .platform
                .as_deref()
                .filter(|p| !p.is_empty())
                .is_none_or(|p| platform_name(url) == p))
                && (search.is_empty()
                    || remark.to_lowercase().contains(&search)
                    || url.to_lowercase().contains(&search))
        })
        .map(|(id, _, _)| id)
        .collect();
    let page_size = query.page_size.unwrap_or(24).clamp(1, 48);
    let total = ids.len();
    let page = query
        .page
        .unwrap_or(1)
        .clamp(1, total.div_ceil(page_size).max(1));
    let ids: Vec<_> = ids
        .into_iter()
        .skip((page - 1) * page_size)
        .take(page_size)
        .collect();
    let rows = selected_rooms(&pool, &ids).await?;
    let workers = managers.get_rooms().await;
    let items = streamer_responses(&caller, &pool, &config, rows, &workers).await;
    let infos = latest_infos(&pool, &ids).await?;
    Ok(Json(StreamerPage {
        items,
        infos,
        total,
        page,
        page_size,
        platforms,
    }))
}

#[derive(Serialize)]
pub struct Overview {
    total: i64,
    recording: usize,
    pending: usize,
    uploading: usize,
    streamers: Vec<LiveStreamerResponse>,
    infos: Vec<StreamerInfo>,
    version: &'static str,
}

pub async fn overview(
    caller: Caller,
    State(pool): State<ConnectionPool>,
    State(managers): State<Arc<DownloadManager>>,
    State(config): State<Arc<RwLock<Config>>>,
) -> Result<Json<Overview>, Response> {
    let total = sqlx::query_scalar("SELECT COUNT(*) FROM livestreamers")
        .fetch_one(&pool)
        .await
        .map_err(database_error)?;
    let workers = managers.get_rooms().await;
    let mut active = Vec::new();
    let (mut pending, mut uploading) = (0, 0);
    for worker in &workers {
        if matches!(
            *worker.downloader_status.read().unwrap(),
            WorkerStatus::Working(_)
        ) {
            active.push(worker.id());
        }
        match &*worker.uploader_status.read().unwrap() {
            WorkerStatus::Pending => pending += 1,
            WorkerStatus::Working(_) => uploading += 1,
            _ => {}
        }
    }
    let recording = active.len();
    active.sort_unstable();
    active.truncate(6);
    let rows = selected_rooms(&pool, &active).await?;
    let streamers = streamer_responses(&caller, &pool, &config, rows, &workers).await;
    let infos = latest_infos(&pool, &active).await?;
    Ok(Json(Overview {
        total,
        recording,
        pending,
        uploading,
        streamers,
        infos,
        version: env!("CARGO_PKG_VERSION"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::infrastructure::{
        connection_pool::ConnectionManager, permissions::Role, policy::Subject,
    };
    use crate::server::{
        common::download::DownloadTask,
        core::downloader::{DownloaderRuntime, DownloaderType},
        infrastructure::context::{Stage, Worker},
    };
    use biliup::downloader::live::{
        DownloaderHint, LivePlugin, LiveRequest, LiveResult, LiveStatus, LiveStream,
    };

    struct Offline;
    #[async_trait::async_trait]
    impl LivePlugin for Offline {
        fn name(&self) -> &'static str {
            "listing-test"
        }
        fn matches(&self, _: &str) -> bool {
            true
        }
        async fn check_stream(&self, _: LiveRequest) -> LiveResult<LiveStatus> {
            panic!("listing must not trigger a live probe")
        }
    }

    #[tokio::test]
    async fn three_hundred_rooms_can_be_searched_paged_and_deleted_without_loading_history() {
        let dir = tempfile::tempdir().unwrap();
        let pool = ConnectionManager::new_pool(dir.path().join("rooms.db").to_str().unwrap())
            .await
            .unwrap();
        let mut tx = pool.begin().await.unwrap();
        for id in 1..=300 {
            let url = format!(
                "https://{}/{}",
                if id % 2 == 0 {
                    "live.bilibili.com"
                } else {
                    "www.douyu.com"
                },
                id
            );
            sqlx::query("INSERT INTO livestreamers(id,url,remark,preprocessor) VALUES(?,?,?,?)")
                .bind(id)
                .bind(&url)
                .bind(format!("主播 {id:03}"))
                .bind(r#"[{"run":"echo private"}]"#)
                .execute(&mut *tx)
                .await
                .unwrap();
            for day in 1..=10 {
                sqlx::query("INSERT INTO stream_sessions(name,url,title,date,live_cover_path,kind) VALUES('主播',?,?,?,'','source')")
                    .bind(&url).bind(format!("第 {day} 场"))
                    .bind(format!("2026-01-{day:02}T00:00:00Z")).execute(&mut *tx).await.unwrap();
            }
            sqlx::query("INSERT INTO stream_sessions(name,url,title,date,live_cover_path,kind) VALUES('合集',?,'不能当成最近直播','2026-02-01T00:00:00Z','','group')")
                .bind(&url).execute(&mut *tx).await.unwrap();
        }
        tx.commit().await.unwrap();
        let manager = Arc::new(DownloadManager::new(1, 0, pool.clone()));
        manager.set_probe_gate(Arc::new(tokio::sync::Semaphore::new(0)));
        manager.add_plugin(Arc::new(Offline)).await;
        let config = Arc::new(RwLock::new(Config::default()));
        let mut caller = Caller::unrestricted();
        caller.subject = Subject::user(1, Role::Viewer);
        let request = |query| {
            page(
                caller.clone(),
                State(pool.clone()),
                State(manager.clone()),
                State(config.clone()),
                Query(query),
            )
        };
        let started = std::time::Instant::now();
        let mut seen = BTreeSet::new();
        for number in 1..=13 {
            let Json(result) = request(PageQuery {
                page: Some(number),
                ..Default::default()
            })
            .await
            .unwrap();
            assert_eq!(result.total, 300);
            assert_eq!(result.items.len(), if number == 13 { 12 } else { 24 });
            assert_eq!(result.infos.len(), result.items.len());
            assert!(result.infos.iter().all(|info| info.title == "第 10 场"));
            for item in result.items {
                assert!(
                    item.inner.preprocessor.is_none(),
                    "viewer must not receive hook commands"
                );
                assert!(seen.insert(item.inner.id), "pages must not repeat rooms");
            }
        }
        assert_eq!(seen.len(), 300);
        let Json(found) = request(PageQuery {
            search: Some("主播 300".into()),
            page: Some(13),
            ..Default::default()
        })
        .await
        .unwrap();
        assert_eq!(found.page, 1);
        assert_eq!(found.items[0].inner.id, 300);
        let Json(filtered) = request(PageQuery {
            platform: Some("哔哩哔哩".into()),
            page_size: Some(500),
            ..Default::default()
        })
        .await
        .unwrap();
        assert_eq!(filtered.total, 150);
        assert_eq!(filtered.items.len(), 48);
        assert!(filtered.items.iter().all(|item| item.inner.id % 2 == 0));
        let Json(empty) = request(PageQuery {
            search: Some("%_' OR 1=1".into()),
            ..Default::default()
        })
        .await
        .unwrap();
        assert_eq!(empty.total, 0);
        assert!(empty.items.is_empty());
        let Json(summary) = overview(
            caller.clone(),
            State(pool.clone()),
            State(manager.clone()),
            State(config.clone()),
        )
        .await
        .unwrap();
        assert_eq!(summary.total, 300);
        assert!(
            summary.streamers.is_empty(),
            "overview must not load idle rooms"
        );
        assert!(summary.infos.is_empty());
        for row in selected_rooms(&pool, &(1..=12).collect::<Vec<_>>())
            .await
            .unwrap()
        {
            let stream = LiveStream {
                name: row.remark.clone(),
                url: row.url.clone(),
                title: "正在录制".into(),
                date: chrono::Utc::now(),
                live_cover_url: String::new(),
                avatar_url: None,
                raw_stream_url: "https://example.invalid/live.flv".into(),
                platform: "bilibili".into(),
                stream_headers: Default::default(),
                suffix: "flv".into(),
                danmaku: None,
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            };
            let worker = Worker::new(row, None, config.clone(), Default::default());
            let task = Arc::new(DownloadTask::new(
                DownloaderRuntime::from_type(DownloaderType::StreamGears),
                &stream,
            ));
            if worker.id() <= 10 {
                worker
                    .change_status(Stage::Download, WorkerStatus::Working(task.clone()))
                    .await;
            }
            if worker.id() <= 2 {
                *worker.uploader_status.write().unwrap() = WorkerStatus::Pending;
            }
            if worker.id() == 3 {
                *worker.uploader_status.write().unwrap() = WorkerStatus::Working(task);
            }
            manager
                .add_room(worker)
                .await
                .expect("test plugin must own the worker");
        }
        let Json(summary) = overview(
            caller.clone(),
            State(pool.clone()),
            State(manager.clone()),
            State(config.clone()),
        )
        .await
        .unwrap();
        assert_eq!(
            (
                summary.total,
                summary.recording,
                summary.pending,
                summary.uploading
            ),
            (300, 10, 2, 1)
        );
        assert_eq!(
            summary
                .streamers
                .iter()
                .map(|s| s.inner.id)
                .collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5, 6]
        );
        assert_eq!(summary.infos.len(), 6);
        assert!(
            summary
                .streamers
                .iter()
                .all(|s| s.status == "Working" && s.inner.preprocessor.is_none())
        );
        sqlx::query("DELETE FROM livestreamers WHERE id > 288")
            .execute(&pool)
            .await
            .unwrap();
        let Json(after_delete) = request(PageQuery {
            page: Some(13),
            ..Default::default()
        })
        .await
        .unwrap();
        assert_eq!(after_delete.page, 12);
        assert_eq!(after_delete.items.last().unwrap().inner.id, 288);
        eprintln!(
            "300 rooms / 3300 sessions: 18 listing requests in {:?}",
            started.elapsed()
        );
    }
}
