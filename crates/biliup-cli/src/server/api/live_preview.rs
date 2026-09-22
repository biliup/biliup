//! `GET /v1/streamers/{id}/live`：把正在录制的那一路流旁路给页面内的播放器。
//!
//! 响应是 chunked 的 `video/x-flv` 或 `video/mp2t` 字节流：先发「文件头 + 序列头 + 当前 GOP」
//! 的快照，再持续转发与写盘同步的实时分块。慢客户端落后到广播缓冲被覆盖（`Lagged`）时
//! 直接结束响应，由播放器重连拿新快照；写入端换代（断流重试 / 换直链）时同样结束响应。
//!
//! 连接数：每路 [`crate::server::common::download::PREVIEW_MAX_SUBSCRIBERS_PER_ROOM`]、
//! 进程 [`MAX_PREVIEW_CONNECTIONS`]，超出返回 429。
//! 路由注册在 `router()` 里，`--auth` 时与 `/v1/streamers` 同一道登录校验。

use crate::server::core::download_manager::DownloadManager;
use crate::server::infrastructure::context::WorkerStatus;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use biliup::downloader::preview::{PreviewHub, SubscribeError, Subscription};
use bytes::Bytes;
use danmaku_client::DanmakuEvent;
use std::collections::VecDeque;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tracing::{debug, info, warn};

/// 进程内同时允许的预览连接总数（所有直播间合计）。
pub const MAX_PREVIEW_CONNECTIONS: usize = 16;
/// 等待写入端给出关键帧对齐快照的最长时间，需长于一个 GOP / 一个 HLS 分片。
pub const SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(20);

fn connection_limit() -> &'static Arc<Semaphore> {
    static LIMIT: OnceLock<Arc<Semaphore>> = OnceLock::new();
    LIMIT.get_or_init(|| Arc::new(Semaphore::new(MAX_PREVIEW_CONNECTIONS)))
}

/// `GET /v1/streamers/{id}/live`
pub async fn get_live_stream(
    State(managers): State<Arc<DownloadManager>>,
    Path(id): Path<i64>,
) -> Response {
    let Some(worker) = managers.get_room_by_id(id).await else {
        return (StatusCode::NOT_FOUND, "直播间不存在").into_response();
    };
    // 只取 hub 句柄，不把状态锁带过 await
    let hub: PreviewHub = match &*worker.downloader_status.read().unwrap() {
        WorkerStatus::Working(task) => task.preview().clone(),
        _ => return (StatusCode::NOT_FOUND, "直播间未在录制").into_response(),
    };
    let Ok(global) = connection_limit().clone().try_acquire_owned() else {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            format!("预览连接数已达上限（进程内最多 {MAX_PREVIEW_CONNECTIONS} 路）"),
        )
            .into_response();
    };
    match hub.subscribe(SUBSCRIBE_TIMEOUT).await {
        Ok(subscription) => {
            info!(id, format = subscription.format.as_str(), "开始直播预览");
            live_response(subscription, global)
        }
        Err(error) => subscribe_error_response(id, error),
    }
}

fn subscribe_error_response(id: i64, error: SubscribeError) -> Response {
    debug!(id, %error, "直播预览订阅失败");
    let (status, message) = match &error {
        SubscribeError::Unavailable(reason) => (StatusCode::UNSUPPORTED_MEDIA_TYPE, reason.clone()),
        SubscribeError::TooManySubscribers(_) => (StatusCode::TOO_MANY_REQUESTS, error.to_string()),
        SubscribeError::NotAttached => (
            StatusCode::SERVICE_UNAVAILABLE,
            "录制尚未开始拉流或正在重连，请稍后重试".to_string(),
        ),
        SubscribeError::Timeout => (
            StatusCode::SERVICE_UNAVAILABLE,
            "等待关键帧超时（流可能已停滞），请稍后重试".to_string(),
        ),
    };
    let mut response = (status, message).into_response();
    if status == StatusCode::SERVICE_UNAVAILABLE {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, HeaderValue::from_static("3"));
    }
    response
}

/// 进程内同时允许的实时弹幕连接总数。弹幕是文本流，比视频便宜得多，上限放宽一些。
pub const MAX_DANMAKU_CONNECTIONS: usize = 32;

fn danmaku_connection_limit() -> &'static Arc<Semaphore> {
    static LIMIT: OnceLock<Arc<Semaphore>> = OnceLock::new();
    LIMIT.get_or_init(|| Arc::new(Semaphore::new(MAX_DANMAKU_CONNECTIONS)))
}

/// SSE 里的一条弹幕事件，字段面向播放器弹幕层。
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct DanmakuFrame {
    /// `danmaku` / `gift` / `super_chat` / `guard_buy`
    pub kind: &'static str,
    /// 显示文本（礼物 / 上舰会拼成一句话）
    pub text: String,
    pub name: Option<String>,
    /// RGB 整数（白 16777215）
    pub color: u32,
    /// 消息时间，Unix 毫秒
    pub ts: i64,
}

impl DanmakuFrame {
    /// 进场消息与无法识别的原始数据不进预览。
    pub fn from_event(event: &DanmakuEvent) -> Option<Self> {
        Some(match event {
            DanmakuEvent::Chat(m) => Self {
                kind: "danmaku",
                text: m.content.clone(),
                name: m.name.clone(),
                color: m.color,
                ts: m.timestamp.timestamp_millis(),
            },
            DanmakuEvent::Gift(m) => Self {
                kind: "gift",
                text: if m.content.is_empty() {
                    format!("{} 送出 {} ×{}", m.name, m.gift_name, m.num)
                } else {
                    m.content.clone()
                },
                name: Some(m.name.clone()),
                color: 0xff_a5_00,
                ts: m.timestamp.timestamp_millis(),
            },
            DanmakuEvent::SuperChat(m) => Self {
                kind: "super_chat",
                text: m.content.clone(),
                name: Some(m.name.clone()),
                color: 0xff_69_b4,
                ts: m.timestamp.timestamp_millis(),
            },
            DanmakuEvent::GuardBuy(m) => Self {
                kind: "guard_buy",
                text: format!("{} 开通了 {} ×{}", m.name, m.gift_name, m.num),
                name: Some(m.name.clone()),
                color: 0x87_ce_eb,
                ts: m.timestamp.timestamp_millis(),
            },
            DanmakuEvent::Enter(_) | DanmakuEvent::Other { .. } => return None,
        })
    }
}

/// `GET /v1/streamers/{id}/danmaku`：正在录制的直播间的实时弹幕，Server-Sent Events。
///
/// 每条事件是一个 [`DanmakuFrame`] 的 JSON；掉队时跳过丢掉的那几条继续（弹幕丢几条无妨），
/// 录制任务结束时流结束。房间不存在 / 未录制 404，平台没有弹幕客户端 415，连接数超限 429。
pub async fn get_live_danmaku(
    State(managers): State<Arc<DownloadManager>>,
    Path(id): Path<i64>,
) -> Response {
    let Some(worker) = managers.get_room_by_id(id).await else {
        return (StatusCode::NOT_FOUND, "直播间不存在").into_response();
    };
    let rx = match &*worker.downloader_status.read().unwrap() {
        WorkerStatus::Working(task) => match task.subscribe_danmaku() {
            Some(rx) => rx,
            None => {
                return (
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    "该平台没有弹幕客户端，无法提供实时弹幕",
                )
                    .into_response();
            }
        },
        _ => return (StatusCode::NOT_FOUND, "直播间未在录制").into_response(),
    };
    let Ok(permit) = danmaku_connection_limit().clone().try_acquire_owned() else {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            format!("实时弹幕连接数已达上限（进程内最多 {MAX_DANMAKU_CONNECTIONS} 路）"),
        )
            .into_response();
    };
    info!(id, "开始实时弹幕");
    danmaku_response(rx, permit)
}

struct DanmakuBody {
    rx: tokio::sync::broadcast::Receiver<DanmakuEvent>,
    _permit: OwnedSemaphorePermit,
}

/// 把弹幕广播编成 SSE。许可随响应体活，客户端断开即释放。
pub fn danmaku_response(
    rx: tokio::sync::broadcast::Receiver<DanmakuEvent>,
    permit: OwnedSemaphorePermit,
) -> Response {
    let stream = futures::stream::unfold(
        DanmakuBody {
            rx,
            _permit: permit,
        },
        |mut state| async move {
            loop {
                match state.rx.recv().await {
                    Ok(event) => {
                        let Some(frame) = DanmakuFrame::from_event(&event) else {
                            continue;
                        };
                        let Ok(json) = serde_json::to_string(&frame) else {
                            continue;
                        };
                        let event = Event::default().event(frame.kind).data(json);
                        return Some((Ok::<Event, std::convert::Infallible>(event), state));
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        debug!(skipped, "实时弹幕订阅者掉队，跳过");
                    }
                    Err(RecvError::Closed) => return None,
                }
            }
        },
    );
    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}

struct LiveBody {
    snapshot: VecDeque<Bytes>,
    /// 持有该路的连接许可；`rx` 是实时接收端
    subscription: Subscription,
    _global: OwnedSemaphorePermit,
}

/// 把订阅编成 chunked 响应：快照分块先发，之后逐个转发实时分块。
///
/// 两个许可（该路、进程）都随响应体一起活，客户端断开或响应结束即释放。
pub fn live_response(mut subscription: Subscription, global: OwnedSemaphorePermit) -> Response {
    let format = subscription.format;
    let state = LiveBody {
        snapshot: VecDeque::from(std::mem::take(&mut subscription.snapshot)),
        subscription,
        _global: global,
    };
    let stream = futures::stream::unfold(state, |mut state| async move {
        if let Some(chunk) = state.snapshot.pop_front() {
            return Some((Ok::<Bytes, std::io::Error>(chunk), state));
        }
        match state.subscription.rx.recv().await {
            Ok(chunk) => Some((Ok(chunk), state)),
            Err(RecvError::Lagged(skipped)) => {
                // 慢客户端：丢掉的数据没法补，结束响应让播放器重连拿新快照
                warn!(skipped, "预览客户端落后于录制进度，断开以便其重连");
                None
            }
            // 写入端换代（拉流结束 / 断流重试）：新连接会拿到新的序列头
            Err(RecvError::Closed) => None,
        }
    });
    let mut response = Body::from_stream(stream).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(format.content_type()),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    // 反向代理不要攒着再发
    headers.insert("X-Accel-Buffering", HeaderValue::from_static("no"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use biliup::downloader::preview::{ChunkKind, PreviewFormat, flv};

    /// 响应体 = 快照（文件头 + 序列头 + GOP）+ 实时分块，写入端 drop 后响应结束；
    /// 头部声明 FLV 类型、禁止缓存、不带 Content-Length（chunked）。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn live_response_streams_snapshot_then_live_chunks_and_ends_with_the_sink() {
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Flv);
        sink.push(ChunkKind::Header, Bytes::from_static(&flv::FILE_HEADER));
        sink.push(ChunkKind::SequenceHeader(9), Bytes::from_static(b"avc"));
        sink.push(ChunkKind::Keyframe, Bytes::from_static(b"K1"));
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, Bytes::from_static(b"p2"));
        let subscription = pending.await.unwrap().unwrap();
        let global = connection_limit().clone().try_acquire_owned().unwrap();
        let response = live_response(subscription, global);

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "video/x-flv"
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store"
        );
        assert!(response.headers().get(header::CONTENT_LENGTH).is_none());

        sink.push(ChunkKind::Media, Bytes::from_static(b"p3"));
        sink.push(ChunkKind::Keyframe, Bytes::from_static(b"K4"));
        drop(sink);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let mut expected = Vec::new();
        expected.extend_from_slice(&flv::FILE_HEADER);
        expected.extend_from_slice(b"avc");
        expected.extend_from_slice(b"K1");
        expected.extend_from_slice(b"p2");
        expected.extend_from_slice(b"p3");
        expected.extend_from_slice(b"K4");
        assert_eq!(&body[..], &expected[..]);
    }

    #[test]
    fn subscribe_errors_map_to_http_statuses() {
        assert_eq!(
            subscribe_error_response(1, SubscribeError::Unavailable("ffmpeg".into())).status(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
        assert_eq!(
            subscribe_error_response(1, SubscribeError::TooManySubscribers(4)).status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        let not_attached = subscribe_error_response(1, SubscribeError::NotAttached);
        assert_eq!(not_attached.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            not_attached.headers().get(header::RETRY_AFTER).unwrap(),
            "3"
        );
        assert_eq!(
            subscribe_error_response(1, SubscribeError::Timeout).status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    /// 弹幕 SSE：Chat / Gift / SuperChat / GuardBuy 各成一条 `event:` + JSON `data:`，
    /// Enter / Other 不出现；掉队跳过；发送端全部 drop 后流结束。
    #[tokio::test]
    async fn danmaku_sse_maps_events_and_ends_when_the_recorder_is_gone() {
        use danmaku_client::message::EnterMessage;
        use danmaku_client::{ChatMessage, GiftMessage};
        let (tx, rx) = tokio::sync::broadcast::channel(8);
        let permit = danmaku_connection_limit()
            .clone()
            .try_acquire_owned()
            .unwrap();
        let response = danmaku_response(rx, permit);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/event-stream"
        );
        tx.send(DanmakuEvent::Chat(
            ChatMessage::new("你好".into())
                .with_name("观众A")
                .with_color(0xff0000),
        ))
        .unwrap();
        tx.send(DanmakuEvent::Enter(EnterMessage {
            name: "路人".into(),
            uid: None,
            timestamp: chrono::Utc::now(),
        }))
        .unwrap();
        tx.send(DanmakuEvent::Gift(GiftMessage {
            name: "土豪".into(),
            uid: 1,
            gift_name: "小心心".into(),
            price: 0,
            num: 3,
            content: String::new(),
            timestamp: chrono::Utc::now(),
        }))
        .unwrap();
        drop(tx);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(text.contains("event: danmaku\ndata: {\"kind\":\"danmaku\",\"text\":\"你好\",\"name\":\"观众A\",\"color\":16711680"), "{text}");
        assert!(
            text.contains("event: gift\ndata: {\"kind\":\"gift\",\"text\":\"土豪 送出 小心心 ×3\""),
            "{text}"
        );
        assert!(!text.contains("路人"), "{text}");
        assert_eq!(
            danmaku_connection_limit().available_permits(),
            MAX_DANMAKU_CONNECTIONS
        );
    }

    #[test]
    fn danmaku_frame_skips_enter_and_other() {
        use danmaku_client::message::EnterMessage;
        assert!(
            DanmakuFrame::from_event(&DanmakuEvent::Other {
                raw_data: "x".into()
            })
            .is_none()
        );
        assert!(
            DanmakuFrame::from_event(&DanmakuEvent::Enter(EnterMessage {
                name: "a".into(),
                uid: Some(1),
                timestamp: chrono::Utc::now(),
            }))
            .is_none()
        );
    }

    /// 进程级连接许可与该路的接收端都随响应体存在，响应体 drop 后立刻释放。
    #[tokio::test]
    async fn global_connection_limit_is_released_with_the_response_body() {
        let limit = connection_limit();
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::MpegTs);
        sink.push(ChunkKind::Keyframe, Bytes::from_static(&[0x47, 0, 0, 0]));
        let pending = tokio::spawn({
            let hub = hub.clone();
            async move { hub.subscribe(Duration::from_secs(5)).await }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, Bytes::from_static(&[0x47, 1, 1, 1]));
        let subscription = pending.await.unwrap().unwrap();
        let before = limit.available_permits();
        let global = limit.clone().try_acquire_owned().unwrap();
        let response = live_response(subscription, global);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "video/mp2t"
        );
        assert_eq!(limit.available_permits(), before - 1);
        assert_eq!(sink.receiver_count(), 1);
        drop(response);
        assert_eq!(limit.available_permits(), before);
        assert_eq!(sink.receiver_count(), 0);
    }
}
