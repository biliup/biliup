//! 正在录制的直播间的写盘速率，供前端画码率曲线：
//!
//! - `GET /v1/ws/live-rates`：WebSocket，连上后每秒推一帧（JSON 数组），前端首选。
//!   WebSocket 不占浏览器对同一主机 HTTP/1.1 的六个并发连接（监视器 4 路视频 + 1 条弹幕 SSE
//!   已经用掉五个，再挂一条 SSE 会把列表轮询饿死），与日志页的 `/v1/ws/logs` 是同一套机制。
//! - `GET /v1/live-rates`：同一帧的一次性 HTTP 版本，供 WebSocket 连不上（反代没放行 upgrade）时
//!   每秒轮询回退，也方便 curl 核对。
//!
//! `/v1/streamers` 每次要查全表、拼齐每个主播的全部字段，10 s 一次的节奏不适合调快；
//! 这里只在内存里扫一遍 worker，对每个 `Working` 的房间读一次 [`RateMeter`] 的窗口均值——
//! 与 `/v1/streamers` 的 `live_bytes_per_sec` 是同一个数，同一秒读到的值相同。
//! 不落库、不新开采样任务、服务端不存历史：曲线的历史由浏览器自己累积（最近 3 分钟）。
//!
//! 带 `?session=` 时这条 WebSocket 还是该页面中转预览的租约心跳（见 `live_preview::lease`）：
//! 服务端每 15 s 发 Ping，45 s 收不到 Pong 就断开，断开即结束该页面的全部中转预览；页面在
//! 开着的预览变化时发来 `{"conns": [...], "seq": N}`，不在里面的预览结束。回退轮询带同一个
//! `session` 时每次都续约。
//!
//! 两个路由都注册在 `router()` 里，`--auth` 时与 `/v1/streamers` 同一道登录校验。
//!
//! [`RateMeter`]: crate::server::common::throughput::RateMeter

use crate::server::api::live_preview::lease::{LEASE_TIMEOUT, PING_INTERVAL, leases, valid_id};
use crate::server::api::ws::websocket_origin_allowed;
use crate::server::core::download_manager::DownloadManager;
use crate::server::infrastructure::context::{Worker, WorkerStatus};
use axum::Json;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{MissedTickBehavior, interval};
use tracing::debug;

/// WebSocket 推送间隔；后端 `RateMeter` 本身 1 s 采一次，再快没有新信息。
pub const PUSH_INTERVAL: Duration = Duration::from_secs(1);
/// 进程内同时允许的码率 WebSocket 连接数（每个标签页一条）。
pub const MAX_RATE_CONNECTIONS: usize = 16;

fn rate_connection_limit() -> &'static Arc<Semaphore> {
    static LIMIT: OnceLock<Arc<Semaphore>> = OnceLock::new();
    LIMIT.get_or_init(|| Arc::new(Semaphore::new(MAX_RATE_CONNECTIONS)))
}

fn acquire_rate_permit(limiter: Arc<Semaphore>) -> Option<OwnedSemaphorePermit> {
    limiter.try_acquire_owned().ok()
}

/// 页面经心跳 WebSocket 报来的「我此刻开着的中转预览」，见 `Leases::sync`。
#[derive(Debug, Deserialize)]
struct OpenConns {
    conns: Vec<String>,
    seq: u64,
}

#[derive(Debug, Default, Deserialize)]
pub struct LeaseQuery {
    /// 页面的中转预览会话号，见 `live_preview::lease`
    pub session: Option<String>,
}

impl LeaseQuery {
    fn session(&self) -> Option<&str> {
        self.session.as_deref().filter(|s| valid_id(s))
    }
}

/// 一个正在录制的直播间在某一秒的写盘速率。
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
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

/// 当前这一秒的一帧：所有录制中房间的速率。
async fn current_frames(managers: &DownloadManager) -> Vec<LiveRateFrame> {
    let workers = managers.get_rooms().await;
    live_rate_frames(&workers, chrono::Utc::now().timestamp_millis())
}

/// `GET /v1/live-rates[?session=]`：带会话号时顺带给该页面的中转预览续约（WebSocket 连不上时的回退）。
pub async fn get_live_rates(
    State(managers): State<Arc<DownloadManager>>,
    Query(query): Query<LeaseQuery>,
) -> Response {
    if let Some(session) = query.session() {
        leases().renew(session);
    }
    let mut response = Json(current_frames(&managers).await).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

/// `GET /v1/ws/live-rates[?session=]`：WebSocket，连上后立刻发一帧，之后每秒一帧（与 `/v1/live-rates`
/// 同一形状的 JSON 数组）。客户端不需要发任何东西；服务端每 [`PING_INTERVAL`] 发 Ping，
/// [`LEASE_TIMEOUT`] 内没有 Pong 就断开；客户端的 Ping 回 Pong，Close 即结束。带 `session` 时
/// 连接存活期间持有该页面的中转预览租约。Origin 校验与连接数上限与 `/v1/ws/logs` 同款。
pub async fn ws_live_rates(
    caller: crate::server::api::access::Caller,
    ws: WebSocketUpgrade,
    State(managers): State<Arc<DownloadManager>>,
    Query(query): Query<LeaseQuery>,
    headers: HeaderMap,
) -> Response {
    if !websocket_origin_allowed(&headers) {
        return (StatusCode::FORBIDDEN, "WebSocket Origin 不受信任").into_response();
    }
    let Some(permit) = acquire_rate_permit(rate_connection_limit().clone()) else {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            format!("码率推送连接数已达上限（进程内最多 {MAX_RATE_CONNECTIONS} 条）"),
        )
            .into_response();
    };
    let session = query.session().map(str::to_string);
    ws.on_upgrade(move |socket| async move {
        let _permit = permit;
        let _lease = session.as_deref().map(|s| leases().heartbeat(s));
        tokio::select! {
            _ = push_live_rates(socket, managers, session) => {}
            _ = caller.revoked() => debug!("会话失效，关闭码率推送"),
        }
    })
}

async fn push_live_rates(
    mut ws: WebSocket,
    managers: Arc<DownloadManager>,
    session: Option<String>,
) {
    let mut tick = interval(PUSH_INTERVAL);
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut ping =
        tokio::time::interval_at(tokio::time::Instant::now() + PING_INTERVAL, PING_INTERVAL);
    ping.set_missed_tick_behavior(MissedTickBehavior::Skip);
    // 最近一次收到客户端的任何帧（浏览器对 Ping 自动回 Pong，后台标签页也一样）
    let mut last_heard = Instant::now();
    debug!("开始码率推送");
    loop {
        tokio::select! {
            maybe_msg = ws.recv() => {
                match maybe_msg {
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        last_heard = Instant::now();
                        if ws.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Text(text))) => {
                        last_heard = Instant::now();
                        if let Some(session) = &session
                            && let Ok(open) = serde_json::from_str::<OpenConns>(&text)
                        {
                            let ended = leases().sync(session, &open.conns, open.seq);
                            debug!(ended, seq = open.seq, "页面报来开着的中转预览");
                        }
                    }
                    Some(Ok(_)) => last_heard = Instant::now(),
                }
            }
            _ = ping.tick() => {
                if ws.send(Message::Ping(Default::default())).await.is_err() {
                    break;
                }
            }
            _ = tick.tick() => {
                if last_heard.elapsed() > LEASE_TIMEOUT {
                    debug!("码率推送 {} s 没有收到 Pong，断开", LEASE_TIMEOUT.as_secs());
                    break;
                }
                let frames = current_frames(&managers).await;
                let Ok(json) = serde_json::to_string(&frames) else { continue };
                if ws.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    }
    let _ = ws.send(Message::Close(None)).await;
    debug!("码率推送结束");
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

    async fn working_worker(
        id: i64,
        downloader: DownloaderType,
    ) -> (Arc<Worker>, Arc<DownloadTask>) {
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

    /// 手写一个最小的 WebSocket 客户端（握手 + 读服务端未掩码的帧），不为测试引新依赖。
    mod ws_client {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        pub async fn handshake(addr: &str, path: &str, origin: &str) -> (TcpStream, u16) {
            let mut stream = TcpStream::connect(addr).await.unwrap();
            let request = format!(
                "GET {path} HTTP/1.1\r\nHost: {addr}\r\nOrigin: {origin}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
            );
            stream.write_all(request.as_bytes()).await.unwrap();
            // 逐字节读到响应头结束
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).await.unwrap();
                head.push(byte[0]);
            }
            let status: u16 = std::str::from_utf8(&head)
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap()
                .parse()
                .unwrap();
            (stream, status)
        }

        /// 读一帧文本（服务端发出的帧不带掩码），跳过 Ping；Close 帧返回 None。
        pub async fn read_text(stream: &mut TcpStream) -> Option<String> {
            loop {
                if let Some(frame) = read_frame(stream).await {
                    return frame;
                }
            }
        }

        /// 读一帧：文本 → `Some(Some(text))`，Close → `Some(None)`，Ping → `None`
        async fn read_frame(stream: &mut TcpStream) -> Option<Option<String>> {
            let mut header = [0u8; 2];
            stream.read_exact(&mut header).await.unwrap();
            let opcode = header[0] & 0x0f;
            let mut len = (header[1] & 0x7f) as u64;
            if len == 126 {
                let mut ext = [0u8; 2];
                stream.read_exact(&mut ext).await.unwrap();
                len = u16::from_be_bytes(ext) as u64;
            } else if len == 127 {
                let mut ext = [0u8; 8];
                stream.read_exact(&mut ext).await.unwrap();
                len = u64::from_be_bytes(ext);
            }
            let mut payload = vec![0u8; len as usize];
            stream.read_exact(&mut payload).await.unwrap();
            match opcode {
                0x1 => Some(Some(String::from_utf8(payload).unwrap())),
                0x8 => Some(None),
                0x9 => None,
                other => panic!("unexpected opcode {other}"),
            }
        }
    }

    async fn serve(managers: Arc<DownloadManager>) -> String {
        use axum::Router;
        use axum::routing::get;
        let app = Router::new()
            .route("/v1/ws/live-rates", get(ws_live_rates))
            .route("/v1/live-rates", get(get_live_rates))
            .route_layer(axum::middleware::from_fn(
                crate::server::api::access::unrestricted,
            ))
            .with_state(managers);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        addr
    }

    /// WebSocket：连上立刻一帧、之后每秒一帧，形状与 HTTP 版一致；Origin 与 Host 不同源时 403。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_websocket_pushes_one_frame_per_second_in_the_same_shape() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let managers = Arc::new(DownloadManager::new(1, 0, pool));
        let addr = serve(managers).await;

        let (_stream, status) =
            ws_client::handshake(&addr, "/v1/ws/live-rates", "https://attacker.example").await;
        assert_eq!(status, 403, "Origin 与 Host 不同源应被拒绝");

        let (mut stream, status) =
            ws_client::handshake(&addr, "/v1/ws/live-rates", &format!("http://{addr}")).await;
        assert_eq!(status, 101);
        let t0 = Instant::now();
        let first = ws_client::read_text(&mut stream).await.unwrap();
        assert!(t0.elapsed() < Duration::from_millis(500), "第一帧应立刻到");
        let frames: Vec<LiveRateFrame> = serde_json::from_str(&first).unwrap();
        assert!(frames.is_empty(), "没有录制中的房间时是空数组");
        let second = ws_client::read_text(&mut stream).await.unwrap();
        let elapsed = t0.elapsed();
        assert!(
            elapsed >= Duration::from_millis(800) && elapsed < Duration::from_millis(1800),
            "第二帧应在约 1 s 后到，实际 {elapsed:?}"
        );
        let _: Vec<LiveRateFrame> = serde_json::from_str(&second).unwrap();
    }

    /// 带 `session` 的码率 WebSocket 持有该页面的中转预览租约：连着时预览不到期，
    /// 断开（页面关闭）时该页面的预览立即结束。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_session_websocket_holds_the_preview_lease_until_it_closes() {
        use biliup::downloader::preview::{PreviewTicket, TicketEnd};
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let managers = Arc::new(DownloadManager::new(1, 0, pool));
        let addr = serve(managers).await;
        let session = format!("ws-lease-test-{}", std::process::id());

        let (mut stream, status) = ws_client::handshake(
            &addr,
            &format!("/v1/ws/live-rates?session={session}"),
            &format!("http://{addr}"),
        )
        .await;
        assert_eq!(status, 101);
        ws_client::read_text(&mut stream).await.unwrap();
        let ticket = PreviewTicket::new();
        let _binding = leases().bind(Some(&session), Some(&format!("{session}.1")), &ticket);
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(ticket.ended_by(), None);

        drop(stream);
        let deadline = Instant::now() + Duration::from_secs(3);
        while ticket.ended_by().is_none() && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(ticket.ended_by(), Some(TicketEnd::LeaseLost));
    }

    /// 码率 WebSocket 连接数有上限，断开后释放。
    /// 用独立的信号量而不是进程级那个：同一测试二进制里的 WebSocket 测试会并发占着一个许可。
    #[test]
    fn websocket_rate_connections_are_bounded() {
        let limiter = Arc::new(Semaphore::new(MAX_RATE_CONNECTIONS));
        let permits: Vec<_> = (0..MAX_RATE_CONNECTIONS)
            .map(|_| acquire_rate_permit(limiter.clone()).unwrap())
            .collect();
        assert!(acquire_rate_permit(limiter.clone()).is_none());
        drop(permits);
        assert!(acquire_rate_permit(limiter).is_some());
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

        let response = get_live_rates(State(managers), Query(LeaseQuery::default())).await;
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
