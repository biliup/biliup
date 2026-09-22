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
use crate::server::infrastructure::dto::DirectCapability;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use biliup::downloader::preview::{PreviewFormat, PreviewHub, SubscribeError, Subscription};
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

/// 浏览器能否直连 CDN 拉这一路——按各平台 CDN 的跨域放行与并发策略实测（PR #1712 review 1）：
///
/// | 平台 | ACAO | 同一直链第二连接 | 判定 |
/// | --- | --- | --- | --- |
/// | B 站 FLV | `*` | 正常 | 能 |
/// | 抖音 FLV | `*` | 正常 | 能 |
/// | 虎牙 FLV | `*` | 正常（边缘节点间歇 403，前端重试 / 回落） | 能 |
/// | 斗鱼 FLV | `*` | **收完 GOP 缓存即 EOF——一 token 一连接**，直连会挤掉录制 | 不能 |
/// | Twitch | 200 响应无 ACAO | — | 不能 |
/// | HLS（TS / fMP4） | — | — | 本版本不能：mpegts.js 放不了 m3u8，直连需 hls.js |
///
/// 其它平台没实测，按不能处理，回落中转。
pub fn direct_capability(
    platform: &str,
    stream_url: &str,
    format: Option<PreviewFormat>,
) -> DirectCapability {
    let no = |reason: &str| DirectCapability {
        capable: false,
        reason: Some(reason.to_string()),
    };
    let is_hls = matches!(format, Some(PreviewFormat::MpegTs | PreviewFormat::Fmp4))
        || stream_url
            .split('?')
            .next()
            .is_some_and(|path| path.ends_with(".m3u8"));
    if is_hls {
        return no("HLS 直连需 hls.js，本版本回落中转");
    }
    match platform {
        "bilibili" | "douyin" | "huya" => DirectCapability {
            capable: true,
            reason: None,
        },
        "douyu" => no("斗鱼 CDN 一个 token 只允许一条连接，直连会挤掉正在录制的那一路"),
        "twitch" => no("Twitch CDN 未放行跨域（响应无 Access-Control-Allow-Origin）"),
        _ => no("该平台的 CDN 跨域放行未验证"),
    }
}

/// 从直链的查询参数里估计过期时间（Unix 秒）。B 站 `expires=`、抖音 `expire=` 是十进制 Unix 秒；
/// 虎牙 `wsTime=`、腾讯云 `txTime=` 是十六进制。斗鱼 `expire=300` 这类相对秒数不算。
pub fn estimate_expiry(stream_url: &str) -> Option<i64> {
    let query = stream_url.split_once('?')?.1;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let parsed = match key {
            "expires" | "expire" | "exp" => value.parse::<i64>().ok(),
            "wsTime" | "txTime" => i64::from_str_radix(value, 16).ok(),
            _ => None,
        };
        // 小于 2001 年的数当作相对秒数或别的东西，不算
        if let Some(ts) = parsed
            && ts > 1_000_000_000
        {
            return Some(ts);
        }
    }
    None
}

/// `GET /v1/streamers/{id}/live-url` 的响应：当前录制中那条流的 CDN 直链。
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct LiveUrlResponse {
    /// 直链，含 CDN 参数；与录制用的是同一条
    pub url: String,
    /// `flv` / `mpegts` / `fmp4`；容器未定时按后缀猜（`.flv` → `flv`），猜不出为 `null`
    pub format: Option<&'static str>,
    pub platform: String,
    /// 过期时间估计（Unix 秒）；直链里没有可识别的过期参数时为 `null`
    pub expires_at: Option<i64>,
    pub direct: DirectCapability,
}

/// `GET /v1/streamers/{id}/live-url`：浏览器直连模式用，返回正在录制的那条流的直链。
/// 只在录制中可用（404 否则）；`direct.capable = false` 时仍返回直链与原因，由前端决定回落。
pub async fn get_live_url(
    State(managers): State<Arc<DownloadManager>>,
    Path(id): Path<i64>,
) -> Response {
    let Some(worker) = managers.get_room_by_id(id).await else {
        return (StatusCode::NOT_FOUND, "直播间不存在").into_response();
    };
    let (source, format) = match &*worker.downloader_status.read().unwrap() {
        WorkerStatus::Working(task) => (task.live_source(), task.preview().status().format),
        _ => return (StatusCode::NOT_FOUND, "直播间未在录制").into_response(),
    };
    let format = format.map(|f| f.as_str()).or_else(|| {
        let path = source.url.split('?').next().unwrap_or("");
        path.ends_with(".flv").then_some("flv")
    });
    let response = LiveUrlResponse {
        direct: direct_capability(&source.platform, &source.url, None),
        expires_at: estimate_expiry(&source.url),
        format,
        platform: source.platform,
        url: source.url,
    };
    let mut response = axum::Json(response).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
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
    /// 直播间 id：一条 SSE 可以复用给多个直播间（监视器），前端按它分发
    pub id: i64,
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
    pub fn from_event(id: i64, event: &DanmakuEvent) -> Option<Self> {
        Some(match event {
            DanmakuEvent::Chat(m) => Self {
                id,
                kind: "danmaku",
                text: m.content.clone(),
                name: m.name.clone(),
                color: m.color,
                ts: m.timestamp.timestamp_millis(),
            },
            DanmakuEvent::Gift(m) => Self {
                id,
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
                id,
                kind: "super_chat",
                text: m.content.clone(),
                name: Some(m.name.clone()),
                color: 0xff_69_b4,
                ts: m.timestamp.timestamp_millis(),
            },
            DanmakuEvent::GuardBuy(m) => Self {
                id,
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
        return danmaku_limit_reached();
    };
    info!(id, "开始实时弹幕");
    danmaku_response(vec![(id, rx)], permit)
}

/// 一条 SSE 里最多复用多少个直播间的弹幕。
pub const MAX_DANMAKU_ROOMS_PER_CONNECTION: usize = 16;

#[derive(Debug, serde::Deserialize)]
pub struct DanmakuQuery {
    /// 逗号分隔的直播间 id
    pub ids: String,
}

/// `GET /v1/danmaku?ids=1,3,9`：多个直播间的实时弹幕复用一条 SSE，事件里带 `id`。
///
/// 监视器同屏 N 路时用它：浏览器对同一主机的 HTTP/1.1 并发连接只有 6 个，N 路视频已经占了
/// N 个，弹幕不能再每路一条。没有弹幕客户端 / 未在录制的 id 静默跳过；一个都没有时 404。
pub async fn get_live_danmaku_multi(
    State(managers): State<Arc<DownloadManager>>,
    Query(query): Query<DanmakuQuery>,
) -> Response {
    let ids: Vec<i64> = query
        .ids
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .take(MAX_DANMAKU_ROOMS_PER_CONNECTION)
        .collect();
    let mut receivers = Vec::new();
    for id in ids {
        let Some(worker) = managers.get_room_by_id(id).await else {
            continue;
        };
        if let WorkerStatus::Working(task) = &*worker.downloader_status.read().unwrap()
            && let Some(rx) = task.subscribe_danmaku()
        {
            receivers.push((id, rx));
        }
    }
    if receivers.is_empty() {
        return (StatusCode::NOT_FOUND, "这些直播间都没有可用的实时弹幕").into_response();
    }
    let Ok(permit) = danmaku_connection_limit().clone().try_acquire_owned() else {
        return danmaku_limit_reached();
    };
    info!(
        ids = ?receivers.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        "开始实时弹幕（复用）"
    );
    danmaku_response(receivers, permit)
}

fn danmaku_limit_reached() -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        format!("实时弹幕连接数已达上限（进程内最多 {MAX_DANMAKU_CONNECTIONS} 路）"),
    )
        .into_response()
}

/// 把一路弹幕广播编成 SSE 事件流；掉队跳过，发送端 drop 后结束。
fn danmaku_events(
    id: i64,
    rx: tokio::sync::broadcast::Receiver<DanmakuEvent>,
) -> impl futures::Stream<Item = Result<Event, std::convert::Infallible>> {
    futures::stream::unfold(rx, move |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let Some(frame) = DanmakuFrame::from_event(id, &event) else {
                        continue;
                    };
                    let Ok(json) = serde_json::to_string(&frame) else {
                        continue;
                    };
                    return Some((Ok(Event::default().event(frame.kind).data(json)), rx));
                }
                Err(RecvError::Lagged(skipped)) => {
                    debug!(id, skipped, "实时弹幕订阅者掉队，跳过");
                }
                Err(RecvError::Closed) => return None,
            }
        }
    })
}

/// 把若干路弹幕广播合成一条 SSE。许可随响应体活，客户端断开即释放；
/// 所有路的录制都结束后流结束。
pub fn danmaku_response(
    receivers: Vec<(i64, tokio::sync::broadcast::Receiver<DanmakuEvent>)>,
    permit: OwnedSemaphorePermit,
) -> Response {
    let merged = futures::stream::select_all(
        receivers
            .into_iter()
            .map(|(id, rx)| Box::pin(danmaku_events(id, rx))),
    );
    let stream = HoldPermit {
        inner: merged,
        _permit: permit,
    };
    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}

/// 让连接许可随 SSE 流一起活。
struct HoldPermit<S> {
    inner: S,
    _permit: OwnedSemaphorePermit,
}

impl<S: futures::Stream + Unpin> futures::Stream for HoldPermit<S> {
    type Item = S::Item;
    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self.inner).poll_next(cx)
    }
}

struct LiveBody {
    snapshot: VecDeque<Bytes>,
    /// 持有该路的连接许可；`rx` 是实时接收端
    subscription: Subscription,
    _global: OwnedSemaphorePermit,
}

impl Drop for LiveBody {
    /// 客户端断开、掉队或写入端换代都走到这里：两个许可随之释放
    fn drop(&mut self) {
        debug!("直播预览连接结束，释放许可");
    }
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

    #[test]
    fn direct_capability_follows_the_measured_platform_table() {
        let flv = "https://d1--ov-gotcha07.bilivideo.com/live-bvc/1/live_x_2500.flv?expires=1790073449&oi=1";
        assert!(direct_capability("bilibili", flv, Some(PreviewFormat::Flv)).capable);
        assert!(
            direct_capability(
                "douyin",
                "https://pull-flv-q11.douyincdn.com/x.flv?expire=1790678986&sign=a",
                None
            )
            .capable
        );
        assert!(
            direct_capability(
                "huya",
                "https://tx.flv.huya.com/src/x.flv?wsSecret=a&wsTime=6ab3aa97",
                Some(PreviewFormat::Flv)
            )
            .capable
        );
        let douyu = direct_capability(
            "douyu",
            "https://ws1a.douyucdn.cn/live/x.flv?wsAuth=a&token=b",
            Some(PreviewFormat::Flv),
        );
        assert!(!douyu.capable);
        assert!(douyu.reason.unwrap().contains("一条连接"));
        let twitch = direct_capability(
            "twitch",
            "https://usher.ttvnw.net/api/channel/hls/x.m3u8?sig=a",
            Some(PreviewFormat::MpegTs),
        );
        assert!(!twitch.capable);
        // HLS 的判定优先于平台：B 站 hls_fmp4 也回落
        let bili_hls = direct_capability(
            "bilibili",
            "https://x.bilivideo.com/live-bvc/1/index.m3u8?expires=1",
            None,
        );
        assert!(!bili_hls.capable);
        assert!(bili_hls.reason.unwrap().contains("hls.js"));
        assert!(!direct_capability("bilibili", flv, Some(PreviewFormat::Fmp4)).capable);
        assert!(!direct_capability("youtube", "https://x/y.flv", None).capable);
    }

    #[test]
    fn expiry_is_read_from_known_query_parameters() {
        assert_eq!(
            estimate_expiry("https://x.bilivideo.com/a.flv?a=1&expires=1790073449&len=0"),
            Some(1790073449)
        );
        assert_eq!(
            estimate_expiry("https://pull-flv-q11.douyincdn.com/a.flv?expire=1790678986&sign=x"),
            Some(1790678986)
        );
        // 虎牙 wsTime 是十六进制
        assert_eq!(
            estimate_expiry("https://tx.flv.huya.com/a.flv?wsSecret=x&wsTime=6ab3aa97"),
            Some(0x6ab3aa97)
        );
        // 斗鱼 expire=300 是相对秒数，不算
        assert_eq!(
            estimate_expiry("https://ws1a.douyucdn.cn/a.flv?wsAuth=x&expire=300&token=y"),
            None
        );
        assert_eq!(estimate_expiry("https://x/a.flv"), None);
        assert_eq!(estimate_expiry("https://x/a.flv?"), None);
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
        let response = danmaku_response(vec![(7, rx)], permit);
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
        assert!(
            text.contains(
                "event: danmaku\ndata: {\"id\":7,\"kind\":\"danmaku\",\"text\":\"你好\",\"name\":\"观众A\",\"color\":16711680"
            ),
            "{text}"
        );
        assert!(
            text.contains(
                "event: gift\ndata: {\"id\":7,\"kind\":\"gift\",\"text\":\"土豪 送出 小心心 ×3\""
            ),
            "{text}"
        );
        assert!(!text.contains("路人"), "{text}");
        assert_eq!(
            danmaku_connection_limit().available_permits(),
            MAX_DANMAKU_CONNECTIONS
        );
    }

    /// 复用：两路广播合成一条 SSE，每条事件带各自的 id；两路发送端都 drop 后流才结束。
    #[tokio::test]
    async fn multiplexed_danmaku_tags_each_event_with_its_room_id() {
        use danmaku_client::ChatMessage;
        let (tx_a, rx_a) = tokio::sync::broadcast::channel(8);
        let (tx_b, rx_b) = tokio::sync::broadcast::channel(8);
        let permit = danmaku_connection_limit()
            .clone()
            .try_acquire_owned()
            .unwrap();
        let response = danmaku_response(vec![(1, rx_a), (3, rx_b)], permit);
        tx_a.send(DanmakuEvent::Chat(ChatMessage::new("来自1".into())))
            .unwrap();
        tx_b.send(DanmakuEvent::Chat(ChatMessage::new("来自3".into())))
            .unwrap();
        drop(tx_a);
        // a 结束了 b 还在：流不能结束
        tx_b.send(DanmakuEvent::Chat(ChatMessage::new("b还在".into())))
            .unwrap();
        drop(tx_b);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let text = String::from_utf8(body.to_vec()).unwrap();
        assert!(
            text.contains("{\"id\":1,\"kind\":\"danmaku\",\"text\":\"来自1\""),
            "{text}"
        );
        assert!(
            text.contains("{\"id\":3,\"kind\":\"danmaku\",\"text\":\"来自3\""),
            "{text}"
        );
        assert!(text.contains("b还在"), "{text}");
        assert_eq!(
            danmaku_connection_limit().available_permits(),
            MAX_DANMAKU_CONNECTIONS
        );
    }

    #[test]
    fn danmaku_frame_skips_enter_and_other() {
        use danmaku_client::message::EnterMessage;
        assert!(
            DanmakuFrame::from_event(
                1,
                &DanmakuEvent::Other {
                    raw_data: "x".into()
                }
            )
            .is_none()
        );
        assert!(
            DanmakuFrame::from_event(
                1,
                &DanmakuEvent::Enter(EnterMessage {
                    name: "a".into(),
                    uid: Some(1),
                    timestamp: chrono::Utc::now(),
                })
            )
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
