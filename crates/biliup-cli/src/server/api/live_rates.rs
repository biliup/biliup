//! `GET /v1/live-rates`：正在录制的直播间的写盘速率，供前端每秒轮询一次画码率曲线。
//!
//! `/v1/streamers` 每次要查全表、拼齐每个主播的全部字段，10 s 一次的节奏不适合调快；
//! 这里只在内存里扫一遍 worker，对每个 `Working` 的房间读一次 [`RateMeter`] 的窗口均值——
//! 与 `/v1/streamers` 的 `live_bytes_per_sec` 是同一个数，同一秒读到的值相同。
//! 不落库、不新开采样任务、服务端不存历史：曲线的历史由浏览器自己累积（最近 3 分钟）。
//!
//! 选轮询而不是 SSE：监视器同屏 4 路视频 + 1 条弹幕 SSE 已经占掉浏览器对同一主机
//! HTTP/1.1 六个并发连接里的五个，再挂一条长连接会把列表轮询饿死；每秒一次的短请求
//! 与列表轮询共用剩下的那一个，没人看图时前端自然停掉。
//!
//! 路由注册在 `router()` 里，`--auth` 时与 `/v1/streamers` 同一道登录校验。
//!
//! [`RateMeter`]: crate::server::common::throughput::RateMeter

use crate::server::core::download_manager::DownloadManager;
use crate::server::infrastructure::context::{Worker, WorkerStatus};
use axum::Json;
use axum::extract::State;
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use std::sync::Arc;

/// 一个正在录制的直播间在某一秒的写盘速率。
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct LiveRateFrame {
    /// 直播间 id（`livestreamers.id`，与 `/v1/streamers` 一致）
    pub id: i64,
    /// 最近一个滑动窗口内的写盘速率（字节/秒）；尚无采样或下载器不经本进程写盘时为 `null`
    pub bytes_per_sec: Option<u64>,
    /// 服务端读数的时刻，Unix 毫秒；同一次响应里所有房间相同
    pub ts: i64,
}

/// 从 worker 列表里挑出正在录制的房间，按 id 升序给出各自的速率。
pub fn live_rate_frames(workers: &[Arc<Worker>], ts: i64) -> Vec<LiveRateFrame> {
    let mut frames: Vec<LiveRateFrame> = workers
        .iter()
        .filter_map(|worker| {
            let status = worker.downloader_status.read().unwrap();
            match &*status {
                WorkerStatus::Working(task) => Some(LiveRateFrame {
                    id: worker.id(),
                    bytes_per_sec: task.bytes_per_sec(),
                    ts,
                }),
                _ => None,
            }
        })
        .collect();
    frames.sort_by_key(|frame| frame.id);
    frames
}

/// `GET /v1/live-rates`
pub async fn get_live_rates(State(managers): State<Arc<DownloadManager>>) -> Response {
    let workers = managers.get_rooms().await;
    let ts = chrono::Utc::now().timestamp_millis();
    let mut response = Json(live_rate_frames(&workers, ts)).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::common::download::DownloadTask;
    use crate::server::config::Config;
    use crate::server::core::downloader::{DownloaderRuntime, DownloaderType};
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use crate::server::infrastructure::context::Stage;
    use crate::server::infrastructure::models::live_streamer::LiveStreamer;
    use biliup::downloader::live::{DownloaderHint, LiveStream};
    use std::collections::HashMap;
    use std::sync::RwLock;
    use std::time::{Duration, Instant};

    fn streamer(id: i64) -> LiveStreamer {
        LiveStreamer {
            id,
            url: format!("https://live.bilibili.com/{id}"),
            remark: format!("room {id}"),
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

    fn live_stream() -> LiveStream {
        LiveStream {
            name: "room".into(),
            url: "https://live.bilibili.com/1".into(),
            title: "t".into(),
            date: chrono::Utc::now(),
            live_cover_url: String::new(),
            avatar_url: None,
            raw_stream_url: "https://cdn.example/live.flv".into(),
            platform: "bilibili".into(),
            stream_headers: HashMap::new(),
            suffix: "flv".into(),
            danmaku: None,
            downloader_hint: DownloaderHint::StreamGears,
            runtime_options: None,
        }
    }

    fn idle_worker(id: i64) -> Arc<Worker> {
        Arc::new(Worker::new(
            streamer(id),
            None,
            Arc::new(RwLock::new(Config::default())),
            Default::default(),
        ))
    }

    async fn working_worker(id: i64, downloader: DownloaderType) -> (Arc<Worker>, Arc<DownloadTask>) {
        let worker = idle_worker(id);
        let task = Arc::new(DownloadTask::new(
            DownloaderRuntime::from_type(downloader),
            &live_stream(),
        ));
        worker
            .change_status(Stage::Download, WorkerStatus::Working(task.clone()))
            .await;
        (worker, task)
    }

    /// 只报告正在录制的房间；刚开录、还没有两次采样时给 `null` 而不是 0。
    #[tokio::test]
    async fn only_recording_rooms_are_reported_sorted_by_id() {
        let (working_b, _) = working_worker(7, DownloaderType::StreamGears).await;
        let (working_a, _) = working_worker(3, DownloaderType::StreamGears).await;
        let workers = vec![idle_worker(1), working_b, working_a, idle_worker(9)];

        let frames = live_rate_frames(&workers, 1_700_000_000_123);
        assert_eq!(
            frames,
            vec![
                LiveRateFrame {
                    id: 3,
                    bytes_per_sec: None,
                    ts: 1_700_000_000_123,
                },
                LiveRateFrame {
                    id: 7,
                    bytes_per_sec: None,
                    ts: 1_700_000_000_123,
                },
            ]
        );
    }

    /// 曲线上的点与卡片数字必须是同一个数：都来自同一个 `RateMeter` 的同一次读数。
    #[tokio::test]
    async fn the_rate_is_the_same_number_the_streamer_list_reports() {
        let (worker, task) = working_worker(1, DownloaderType::StreamGears).await;
        let meter = task.rate_meter();
        let t0 = Instant::now() - Duration::from_secs(2);
        meter.sample_at(t0);
        meter.counter().add(2_000_000);
        meter.sample_at(t0 + Duration::from_secs(2));

        let frames = live_rate_frames(&[worker], 0);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].bytes_per_sec, Some(1_000_000));
        assert_eq!(frames[0].bytes_per_sec, task.bytes_per_sec());
    }

    /// 字节不经过本进程的下载器（边录边传 / yt-dlp）与列表一样报 `null`。
    #[tokio::test]
    async fn downloaders_without_a_counter_report_null() {
        let (worker, task) = working_worker(1, DownloaderType::YtDlp).await;
        task.rate_meter().counter().add(4096);
        task.rate_meter().sample();
        let frames = live_rate_frames(&[worker], 0);
        assert_eq!(frames[0].bytes_per_sec, None);
    }

    /// 前端按这三个键读：`id` / `bytes_per_sec` / `ts`。
    #[test]
    fn the_frame_serialises_to_the_shape_the_frontend_reads() {
        let json = serde_json::to_value(LiveRateFrame {
            id: 5,
            bytes_per_sec: Some(655_360),
            ts: 1_700_000_000_000,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "id": 5, "bytes_per_sec": 655_360, "ts": 1_700_000_000_000_i64 })
        );
        let json = serde_json::to_value(LiveRateFrame {
            id: 5,
            bytes_per_sec: None,
            ts: 0,
        })
        .unwrap();
        assert_eq!(json["bytes_per_sec"], serde_json::Value::Null);
    }

    /// 处理函数：空列表也是合法响应（`[]`），且禁止缓存——每秒都要拿到新读数。
    #[tokio::test]
    async fn the_endpoint_returns_an_uncacheable_json_array() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let managers = Arc::new(DownloadManager::new(1, 0, pool));

        let response = get_live_rates(State(managers)).await;
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .unwrap()
                .to_str()
                .unwrap(),
            "application/json"
        );
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"[]");
    }
}
