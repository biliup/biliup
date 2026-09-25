//! `GET /v1/streamers/{id}/live`：把正在录制的那一路流旁路给页面内的播放器。
//!
//! 响应是 chunked 的 `video/x-flv` / `video/mp2t` / `video/mp4` 字节流：先发「文件头 + 序列头 +
//! 最近几秒的 GOP」的快照，再持续转发与写盘同步的实时分块。慢客户端落后到广播缓冲被覆盖
//! （`Lagged`）时不断开，而是在同一条响应里从最近的关键帧重新对齐（再要一份不含文件头的快照）；
//! 写入端换代（断流重试 / 换直链）时结束响应，由播放器重连。
//!
//! 连接数：每路 [`crate::server::common::download::PREVIEW_MAX_SUBSCRIBERS_PER_ROOM`]、
//! 进程 [`MAX_PREVIEW_CONNECTIONS`]，都是固定槽位。满了时新打开的预览挤掉（该直播间 / 全进程）
//! 最早的那条，被挤掉的响应随即结束；只有播放器自动重连（`?reconnect=1`）才在满员时得到 429，
//! 免得几个真在看的页面互相挤来挤去。每条连接另有最长寿命（配置 `preview_max_minutes`），
//! 到点结束响应：服务端看不出客户端是否还在看——经过会替客户端读完上游的代理 / 隧道时，
//! 关掉播放器 TCP 也不断，许可只靠响应体 drop 释放就会一直占着。
//! 路由注册在 `router()` 里，`--auth` 时与 `/v1/streamers` 同一道登录校验。

use crate::server::core::download_manager::DownloadManager;
use crate::server::core::live::live_request;
use crate::server::infrastructure::context::WorkerStatus;
use crate::server::infrastructure::dto::DirectCapability;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use biliup::downloader::live::{LiveStatus, strip_ws_expire_override};
use biliup::downloader::preview::{
    Evicted, PreviewFormat, PreviewHub, PreviewSlot, PreviewSlots, PreviewTicket, SlotScope,
    SubscribeError, Subscription,
};
use bytes::{Bytes, BytesMut};
use danmaku_client::DanmakuEvent;
use std::collections::{HashMap, VecDeque};
use std::pin::Pin;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, broadcast};
use tokio::time::Sleep;
use tracing::{debug, info, warn};

/// 进程内同时允许的预览连接总数（所有直播间合计）。
pub const MAX_PREVIEW_CONNECTIONS: usize = 16;
/// 等待写入端给出关键帧对齐快照的最长时间，需长于一个 GOP / 一个 HLS 分片。
pub const SUBSCRIBE_TIMEOUT: Duration = Duration::from_secs(20);

fn connection_slots() -> &'static PreviewSlots {
    static SLOTS: OnceLock<PreviewSlots> = OnceLock::new();
    SLOTS.get_or_init(|| PreviewSlots::new(SlotScope::Process, MAX_PREVIEW_CONNECTIONS))
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct LiveQuery {
    /// 起播快照回溯多少毫秒的已完成 GOP（见 `PreviewHub::subscribe_with_depth`）：
    /// 播放器按自己要维持的缓冲深度来要；不带则给整个保留窗口（`SNAPSHOT_WINDOW`）
    pub snapshot_ms: Option<u64>,
    /// `1`：播放器断开后的自动重连。满员时返回 429 而不挤掉别人——被挤掉的页面若也去挤，
    /// 几个真在看的页面就会轮流把对方挤掉；用户手动打开 / 重试不带它，总能挤进来。
    pub reconnect: Option<String>,
}

impl LiveQuery {
    fn may_evict(&self) -> bool {
        !matches!(self.reconnect.as_deref(), Some("1" | "true"))
    }
}

/// 满员被拒时的 429；正文由前端原样显示，所以写清是哪一级满了。
fn too_many_response(scope: SlotScope, capacity: usize) -> Response {
    let message = match scope {
        SlotScope::Room => SubscribeError::TooManySubscribers(capacity).to_string(),
        SlotScope::Process => format!("预览连接数已达上限（进程内最多 {capacity} 路）"),
    };
    (StatusCode::TOO_MANY_REQUESTS, message).into_response()
}

fn log_eviction(id: i64, conn: u64, scope: SlotScope, capacity: usize, evicted: Evicted) {
    info!(
        id,
        conn,
        evicted_conn = evicted.id,
        evicted_age_secs = evicted.age.as_secs(),
        capacity,
        "{}预览连接已满，挤掉最早的一条",
        scope.as_str()
    );
}

/// `GET /v1/streamers/{id}/live?snapshot_ms=2000[&reconnect=1]`
pub async fn get_live_stream(
    State(managers): State<Arc<DownloadManager>>,
    Path(id): Path<i64>,
    Query(query): Query<LiveQuery>,
) -> Response {
    let depth = query.snapshot_ms.map(Duration::from_millis);
    let evict = query.may_evict();
    let Some(worker) = managers.get_room_by_id(id).await else {
        return (StatusCode::NOT_FOUND, "直播间不存在").into_response();
    };
    // 只取 hub 句柄，不把状态锁带过 await
    let hub: PreviewHub = match &*worker.downloader_status.read().unwrap() {
        WorkerStatus::Working(task) => task.preview().clone(),
        _ => return (StatusCode::NOT_FOUND, "直播间未在录制").into_response(),
    };
    let lifetime = worker.get_config().preview_max_lifetime();
    let ticket = PreviewTicket::new();
    // 先占直播间一级：它挤掉的那条在进程一级随即不占名额，进程一级不会因此再多挤一条
    let slot = match hub.reserve(&ticket, evict) {
        Ok((slot, evicted)) => {
            if let Some(evicted) = evicted {
                log_eviction(
                    id,
                    ticket.id(),
                    SlotScope::Room,
                    hub.max_subscribers(),
                    evicted,
                );
            }
            slot
        }
        Err(SubscribeError::TooManySubscribers(capacity)) => {
            info!(
                id,
                occupied = hub.subscribers(),
                capacity,
                "该直播间预览连接已满，拒绝自动重连（429）"
            );
            return too_many_response(SlotScope::Room, capacity);
        }
        Err(error) => return subscribe_error_response(id, error),
    };
    let global = match connection_slots().acquire(&ticket, evict) {
        Ok((global, evicted)) => {
            if let Some(evicted) = evicted {
                log_eviction(
                    id,
                    ticket.id(),
                    SlotScope::Process,
                    MAX_PREVIEW_CONNECTIONS,
                    evicted,
                );
            }
            global
        }
        Err(occupied) => {
            info!(
                id,
                occupied,
                capacity = MAX_PREVIEW_CONNECTIONS,
                "进程内预览连接已满，拒绝自动重连（429）"
            );
            return too_many_response(SlotScope::Process, MAX_PREVIEW_CONNECTIONS);
        }
    };
    match hub.subscribe_reserved(slot, depth, SUBSCRIBE_TIMEOUT).await {
        Ok(subscription) => {
            info!(
                id,
                conn = ticket.id(),
                format = subscription.format.as_str(),
                snapshot_ms = depth.map(|d| d.as_millis() as u64),
                snapshot_chunks = subscription.snapshot.len(),
                snapshot_bytes = subscription.snapshot.iter().map(Bytes::len).sum::<usize>(),
                room_subscribers = hub.subscribers(),
                process_subscribers = connection_slots().occupied(),
                max_lifetime_secs = lifetime.map(|d| d.as_secs()),
                reconnect = !evict,
                "开始直播预览"
            );
            live_response(
                id,
                hub,
                subscription,
                LiveGuard {
                    ticket,
                    _global: global,
                },
                lifetime,
            )
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

/// 浏览器能否直连 CDN 拉这一路——按各平台 CDN 的跨域放行实测（PR #1712 review 1 / 2）。
///
/// 直连**不复用录制那条直链**而是向平台另取一条（新 token），所以「同一直链两条连接」的限制
/// （斗鱼一 token 一连接、部分虎牙节点对第二条连接只给 GOP 缓存即断）不再是问题——实测新 token
/// 的斗鱼 / 虎牙直链各读 25 s 稳定，录制那条不受影响。容器不影响判定：FLV 走 mpegts.js，
/// HLS（TS / fMP4 分片，`.m3u8`）走 hls.js。HLS 实测：B 站 fMP4 的 m3u8 / init / `.m4s` 同域同 `ACAO: *`；
/// 抖音 TS 的主清单（`pull-hls-*.douyinliving.com`）与媒体清单 + 分片（`*.100ycdn.com`）不同域，都 `*`；
/// 虎牙 TS 的 m3u8 与分片同域，回显 Origin（清单轮询有边缘节点间歇 403，靠重取 / 回落）。
///
/// | 平台 | 跨域 | 判定 |
/// | --- | --- | --- |
/// | B 站 / 抖音 / 虎牙 / 斗鱼 | FLV 与 HLS 的清单、分片都放行 | 能 |
/// | Twitch | usher 主清单无 ACAO；媒体清单 / 分片虽带 `ACAO: *`，但清单服务器按 Origin 白名单放行（只有 twitch.tv 与 localhost），其它站点 403 | 不能 |
///
/// 其它平台没实测，按不能处理，回落中转。
pub fn direct_capability(platform: &str) -> DirectCapability {
    let no = |reason: &str| DirectCapability {
        capable: false,
        reason: Some(reason.to_string()),
    };
    match platform {
        "bilibili" | "douyin" | "huya" | "douyu" => DirectCapability {
            capable: true,
            reason: None,
        },
        "twitch" => no(
            "Twitch 的清单服务器按 Origin 白名单放行（只有 twitch.tv 与 localhost），其它站点跨域请求 403",
        ),
        _ => no("该平台的 CDN 跨域放行未验证"),
    }
}

/// 直链在浏览器里该用哪个播放器：`.flv` → mpegts.js，`.m3u8` → hls.js，其它猜不出。
pub fn direct_format(stream_url: &str) -> Option<&'static str> {
    let path = stream_url.split('?').next().unwrap_or("");
    if path.ends_with(".flv") {
        Some("flv")
    } else if path.ends_with(".m3u8") {
        Some("hls")
    } else {
        None
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
    /// 向平台新取的直链，含 CDN 参数；与录制那条不是同一个 token
    pub url: String,
    /// 浏览器该用哪个播放器：`flv`（mpegts.js）/ `hls`（hls.js，TS 或 fMP4 分片都行）；后缀猜不出为 `null`
    pub format: Option<&'static str>,
    pub platform: String,
    /// 过期时间估计（Unix 秒）；直链里没有可识别的过期参数时为 `null`
    pub expires_at: Option<i64>,
    pub direct: DirectCapability,
    /// 这条是 [`LIVE_URL_DEBOUNCE`] 内复用的上一次结果（同一房间短时间内重复打开），不是新取的
    pub cached: bool,
}

/// 同一房间两次 `live-url` 之间少于这个间隔就复用上一次的直链，不再打平台 API
/// （StrictMode 双调用、连点、同一房间几个小窗同时打开）。播放失败后的重取带 `?fresh=1` 绕过它。
pub const LIVE_URL_DEBOUNCE: Duration = Duration::from_secs(5);

#[derive(Debug, Default, serde::Deserialize)]
pub struct LiveUrlQuery {
    /// 为 `1` / `true` 时忽略去抖缓存，一定向平台新取（前端播放失败后的重取用）
    #[serde(default)]
    pub fresh: Option<String>,
}

impl LiveUrlQuery {
    fn wants_fresh(&self) -> bool {
        matches!(self.fresh.as_deref(), Some("1") | Some("true"))
    }
}

/// 每个房间最近一次成功取到的直链与时刻。条目只在 [`LIVE_URL_DEBOUNCE`] 内有意义，插入时顺手
/// 清掉一分钟前的，表不会长。
fn live_url_cache() -> &'static std::sync::Mutex<HashMap<i64, (Instant, LiveUrlResponse)>> {
    static CACHE: OnceLock<std::sync::Mutex<HashMap<i64, (Instant, LiveUrlResponse)>>> =
        OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

/// 去抖窗口内有没有可复用的直链。
pub fn recent_live_url(id: i64, now: Instant) -> Option<LiveUrlResponse> {
    let cache = live_url_cache().lock().unwrap();
    cache
        .get(&id)
        .filter(|(at, _)| now.duration_since(*at) < LIVE_URL_DEBOUNCE)
        .map(|(_, response)| LiveUrlResponse {
            cached: true,
            ..response.clone()
        })
}

/// 记下这次取到的直链；顺手清掉过期很久的条目。
pub fn remember_live_url(id: i64, now: Instant, response: &LiveUrlResponse) {
    let mut cache = live_url_cache().lock().unwrap();
    cache.retain(|_, (at, _)| now.duration_since(*at) < Duration::from_secs(60));
    cache.insert(id, (now, response.clone()));
}

/// `GET /v1/streamers/{id}/live-url`：浏览器直连模式用。
///
/// **不复用录制那条直链**，而是向平台重新取一条（ForgQi，#1712）：斗鱼 CDN 一个 token 只允许一条连接、
/// 部分虎牙节点对同一直链的第二条连接只给 GOP 缓存即断——都是「同一直链两条连接」的问题，
/// 浏览器拿自己的 token 就绕开了，也不会挤掉录制。只在录制中可用（404 否则）；向平台取新直链
/// 失败时 503，由前端回落中转。
pub async fn get_live_url(
    State(managers): State<Arc<DownloadManager>>,
    Path(id): Path<i64>,
    Query(query): Query<LiveUrlQuery>,
) -> Response {
    let Some(worker) = managers.get_room_by_id(id).await else {
        return (StatusCode::NOT_FOUND, "直播间不存在").into_response();
    };
    let source = match &*worker.downloader_status.read().unwrap() {
        WorkerStatus::Working(task) => task.live_source(),
        _ => return (StatusCode::NOT_FOUND, "直播间未在录制").into_response(),
    };
    let capability = direct_capability(&source.platform);
    if !capability.capable {
        // 不能直连就不去平台多要一条，直接把原因给前端回落
        let response = LiveUrlResponse {
            direct: capability,
            expires_at: None,
            format: direct_format(&source.url),
            platform: source.platform,
            url: String::new(),
            cached: false,
        };
        return no_store(axum::Json(response).into_response());
    }
    let now = Instant::now();
    if !query.wants_fresh()
        && let Some(recent) = recent_live_url(id, now)
    {
        debug!(id, "预览直链在去抖窗口内，复用上一次的");
        return no_store(axum::Json(recent).into_response());
    }
    let room_url = worker.get_streamer().url.clone();
    let Some(plugin) = managers.plugin_for(&room_url).await else {
        return (StatusCode::SERVICE_UNAVAILABLE, "找不到该直播间的平台插件").into_response();
    };
    let fresh = match plugin.check_stream(live_request(&worker)).await {
        Ok(LiveStatus::Live { stream }) => stream,
        Ok(LiveStatus::Offline) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "平台返回未开播，取不到新直链",
            )
                .into_response();
        }
        Err(e) => {
            warn!(id, error = ?e, "向平台取预览直链失败");
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                format!("向平台取新直链失败：{e}"),
            )
                .into_response();
        }
    };
    // 浏览器那边没有 403 兜底，直连照旧给原直链
    let url = strip_ws_expire_override(&fresh.raw_stream_url)
        .map(str::to_owned)
        .unwrap_or(fresh.raw_stream_url);
    let response = LiveUrlResponse {
        direct: direct_capability(&fresh.platform),
        expires_at: estimate_expiry(&url),
        format: direct_format(&url),
        platform: fresh.platform,
        url,
        cached: false,
    };
    remember_live_url(id, now, &response);
    info!(id, fresh = query.wants_fresh(), "已向平台取到预览直链");
    no_store(axum::Json(response).into_response())
}

fn no_store(mut response: Response) -> Response {
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

/// 一次响应里最多容忍多少次掉队重对齐；再多说明客户端带宽长期跟不上，结束响应交给播放器
/// （它会显示原因并按自己的策略重连）。
pub const MAX_RESYNCS_PER_RESPONSE: u32 = 20;
/// 实时转发时把已经攒在广播缓冲里的多个分块合并成一个响应块的上限：客户端追赶积压时
/// 少写几次 socket / 少几个 chunked 帧；没有积压时一个分块就是一个响应块，不加任何延迟。
pub const COALESCE_BYTES: usize = 64 * 1024;

/// 一条中转预览连接在进程一级的身份与槽位，随响应体一起活。
pub struct LiveGuard {
    ticket: PreviewTicket,
    _global: PreviewSlot,
}

/// 中转预览响应为什么结束，只用于日志。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EndReason {
    /// 响应体被 drop 而流自己没结束：客户端断开（或服务退出）
    ClientGone,
    /// 满员时被新打开的预览挤掉
    Evicted(SlotScope),
    /// 到了单条连接的最长寿命
    Expired,
    /// 写入端换代（断流重试 / 换直链 / 录制结束）
    SinkClosed,
    /// 客户端长期跟不上，掉队重对齐次数超限或重对齐失败
    Lagging,
}

impl EndReason {
    fn as_str(self) -> &'static str {
        match self {
            EndReason::ClientGone => "客户端断开",
            EndReason::Evicted(SlotScope::Room) => "被挤掉（该直播间已满）",
            EndReason::Evicted(SlotScope::Process) => "被挤掉（进程内已满）",
            EndReason::Expired => "到达最长时长",
            EndReason::SinkClosed => "录制端换代或结束",
            EndReason::Lagging => "客户端持续掉队",
        }
    }
}

struct LiveBody {
    id: i64,
    format: PreviewFormat,
    hub: PreviewHub,
    snapshot: VecDeque<Bytes>,
    /// 持有该路的连接许可；`rx` 是实时接收端。掉队重对齐时被 take 走再换新的
    subscription: Option<Subscription>,
    /// 合并分块时 `try_recv` 撞上的掉队，留到下一轮按掉队处理
    pending_lag: Option<u64>,
    resyncs: u32,
    /// 最长寿命的计时器；`None` 为不限
    deadline: Option<Pin<Box<Sleep>>>,
    opened: Instant,
    bytes_sent: u64,
    /// 流自己结束时记下原因；`None` 表示是被 drop 的（客户端断开）
    end: Option<EndReason>,
    guard: LiveGuard,
}

impl LiveBody {
    /// 结束响应：记下原因后返回 `None`，`unfold` 随即 drop 状态、释放两个许可。
    fn finish<T>(&mut self, reason: EndReason) -> Option<T> {
        self.end = Some(reason);
        None
    }
}

async fn expiry(deadline: &mut Option<Pin<Box<Sleep>>>) {
    match deadline {
        Some(sleep) => sleep.as_mut().await,
        None => std::future::pending().await,
    }
}

/// 实时分块到手后，把广播缓冲里已经攒着的后续分块（客户端在追赶积压时才会有）
/// 一起合并成一个响应块，最多 [`COALESCE_BYTES`]；缓冲空着就原样返回、不复制。
/// 途中 `try_recv` 报告掉队时停止合并，把掉队数交回调用方处理。
fn coalesce(rx: &mut broadcast::Receiver<Bytes>, first: Bytes) -> (Bytes, Option<u64>) {
    use tokio::sync::broadcast::error::TryRecvError;
    if first.len() >= COALESCE_BYTES || rx.is_empty() {
        return (first, None);
    }
    let mut out = BytesMut::with_capacity(COALESCE_BYTES);
    out.extend_from_slice(&first);
    loop {
        match rx.try_recv() {
            Ok(chunk) => {
                out.extend_from_slice(&chunk);
                if out.len() >= COALESCE_BYTES {
                    return (out.freeze(), None);
                }
            }
            Err(TryRecvError::Lagged(skipped)) => return (out.freeze(), Some(skipped)),
            Err(TryRecvError::Empty | TryRecvError::Closed) => return (out.freeze(), None),
        }
    }
}

impl Drop for LiveBody {
    /// 客户端断开、被挤掉、到期、掉队或写入端换代都走到这里：两个许可随之释放
    fn drop(&mut self) {
        info!(
            id = self.id,
            conn = self.guard.ticket.id(),
            format = self.format.as_str(),
            duration_secs = self.opened.elapsed().as_secs(),
            bytes_sent = self.bytes_sent,
            resyncs = self.resyncs,
            reason = self.end.unwrap_or(EndReason::ClientGone).as_str(),
            "直播预览结束，释放许可"
        );
    }
}

enum Next {
    Received(Result<Bytes, RecvError>),
    End(EndReason),
}

/// 把订阅编成 chunked 响应：快照分块先发，之后逐个转发实时分块；掉队时原地重对齐。
///
/// 两个许可（该路、进程）都随响应体一起活，客户端断开或响应结束即释放。响应在这些情况下
/// 主动结束（流返回 `None`，chunked 正常收尾，连接可以继续 keep-alive）：`guard` 的票被挤掉、
/// 到了 `lifetime`、写入端换代、客户端持续掉队。
pub fn live_response(
    id: i64,
    hub: PreviewHub,
    mut subscription: Subscription,
    guard: LiveGuard,
    lifetime: Option<Duration>,
) -> Response {
    let format = subscription.format;
    let state = LiveBody {
        id,
        format,
        hub,
        snapshot: VecDeque::from(std::mem::take(&mut subscription.snapshot)),
        subscription: Some(subscription),
        pending_lag: None,
        resyncs: 0,
        deadline: lifetime.map(|d| Box::pin(tokio::time::sleep(d))),
        opened: Instant::now(),
        bytes_sent: 0,
        end: None,
        guard,
    };
    let stream = futures::stream::unfold(state, move |mut state| async move {
        loop {
            if let Some(scope) = state.guard.ticket.evicted_by() {
                return state.finish(EndReason::Evicted(scope));
            }
            if let Some(chunk) = state.snapshot.pop_front() {
                state.bytes_sent += chunk.len() as u64;
                return Some((Ok::<Bytes, std::io::Error>(chunk), state));
            }
            let next = match state.pending_lag.take() {
                Some(skipped) => Next::Received(Err(RecvError::Lagged(skipped))),
                None => {
                    let Some(subscription) = state.subscription.as_mut() else {
                        return state.finish(EndReason::Lagging);
                    };
                    tokio::select! {
                        biased;
                        scope = state.guard.ticket.evicted() => Next::End(EndReason::Evicted(scope)),
                        _ = expiry(&mut state.deadline) => Next::End(EndReason::Expired),
                        received = subscription.rx.recv() => Next::Received(received),
                    }
                }
            };
            match next {
                Next::End(reason) => return state.finish(reason),
                Next::Received(Ok(chunk)) => {
                    let Some(subscription) = state.subscription.as_mut() else {
                        return state.finish(EndReason::Lagging);
                    };
                    let (chunk, lagged) = coalesce(&mut subscription.rx, chunk);
                    state.pending_lag = lagged;
                    state.bytes_sent += chunk.len() as u64;
                    return Some((Ok(chunk), state));
                }
                Next::Received(Err(RecvError::Lagged(skipped))) => {
                    // 慢客户端：丢掉的数据没法补，但不必断开——从最近的关键帧重新对齐，
                    // 快照里不再带文件头（fMP4 的 init segment 除外，MSE 接受中途再来一份）
                    state.resyncs += 1;
                    if state.resyncs > MAX_RESYNCS_PER_RESPONSE {
                        warn!(
                            id = state.id,
                            skipped,
                            resyncs = state.resyncs,
                            "预览客户端持续落后于录制进度，结束响应"
                        );
                        return state.finish(EndReason::Lagging);
                    }
                    warn!(
                        id = state.id,
                        skipped,
                        resyncs = state.resyncs,
                        "预览客户端落后于录制进度，从最近的关键帧重新对齐"
                    );
                    let Some(previous) = state.subscription.take() else {
                        return state.finish(EndReason::Lagging);
                    };
                    let again = tokio::select! {
                        biased;
                        scope = state.guard.ticket.evicted() => {
                            return state.finish(EndReason::Evicted(scope));
                        }
                        _ = expiry(&mut state.deadline) => return state.finish(EndReason::Expired),
                        again = state.hub.resubscribe(previous, SUBSCRIBE_TIMEOUT) => again,
                    };
                    let Ok(again) = again else {
                        return state.finish(EndReason::Lagging);
                    };
                    state.snapshot = if format == PreviewFormat::Fmp4 {
                        again.snapshot.iter().cloned().collect()
                    } else {
                        again.snapshot_after_header().iter().cloned().collect()
                    };
                    state.subscription = Some(again);
                }
                // 写入端换代（拉流结束 / 断流重试）：新连接会拿到新的序列头
                Next::Received(Err(RecvError::Closed)) => {
                    return state.finish(EndReason::SinkClosed);
                }
            }
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
        let response = live_response(1, hub.clone(), subscription, test_guard(), None);

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

    /// 掉队时不结束响应：从最近的关键帧重新对齐，续上的快照不再带 FLV 文件头，
    /// 但带序列头（掉队期间可能换过）；响应体里文件头只出现一次。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn lagged_client_is_realigned_in_place_instead_of_disconnected() {
        use biliup::downloader::preview::BROADCAST_CAPACITY_FLV;
        let hub = PreviewHub::new(4);
        let mut sink = hub
            .attach(PreviewFormat::Flv)
            .with_snapshot_window(Duration::ZERO);
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
        let response = live_response(1, hub.clone(), subscription, test_guard(), None);

        // 响应体还没被读，写入端推满缓冲再多一些：接收端必然掉队
        for _ in 0..(BROADCAST_CAPACITY_FLV + 100) {
            sink.push(ChunkKind::Media, Bytes::from_static(b"x"));
        }
        // 读取任务：先拿到快照，撞上掉队 → 向写入端要新快照（要等下一个关键帧）
        let reader = tokio::spawn(async move {
            axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        sink.push(ChunkKind::Keyframe, Bytes::from_static(b"K9"));
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, Bytes::from_static(b"p10"));
        drop(sink);
        let body = reader.await.unwrap();
        let text = String::from_utf8_lossy(&body);
        assert!(text.starts_with("FLV"), "{text}");
        assert!(
            text.ends_with("avcK9p10"),
            "resync = seq header + new GOP, got {text}"
        );
        assert_eq!(text.matches("FLV").count(), 1, "file header only once");
        assert!(
            !text.contains("xx"),
            "no stale chunks after the lag: {text}"
        );
    }

    /// 合并：缓冲里攒着的分块合成一个响应块（不超过上限）；缓冲空时原样返回；
    /// 途中掉队交回调用方。
    #[tokio::test]
    async fn coalesce_merges_backlog_and_reports_lag() {
        let (tx, mut rx) = broadcast::channel::<Bytes>(4);
        tx.send(Bytes::from_static(b"a")).unwrap();
        let first = rx.recv().await.unwrap();
        let (out, lag) = coalesce(&mut rx, first);
        assert_eq!(&out[..], b"a");
        assert!(lag.is_none());

        tx.send(Bytes::from_static(b"b")).unwrap();
        tx.send(Bytes::from_static(b"c")).unwrap();
        tx.send(Bytes::from_static(b"d")).unwrap();
        let first = rx.recv().await.unwrap();
        let (out, lag) = coalesce(&mut rx, first);
        assert_eq!(&out[..], b"bcd");
        assert!(lag.is_none());
        assert!(rx.is_empty());

        // 上限：单个大分块不合并
        let big = Bytes::from(vec![0u8; COALESCE_BYTES]);
        tx.send(big.clone()).unwrap();
        tx.send(Bytes::from_static(b"e")).unwrap();
        let first = rx.recv().await.unwrap();
        let (out, _) = coalesce(&mut rx, first);
        assert_eq!(out.len(), COALESCE_BYTES);
        assert_eq!(rx.recv().await.unwrap(), Bytes::from_static(b"e"));

        // 掉队：合并到掉队处为止，把跳过数交回
        for i in 0..8u8 {
            tx.send(Bytes::from(vec![i])).unwrap();
        }
        assert!(matches!(rx.recv().await, Err(RecvError::Lagged(_))));
        let first = rx.recv().await.unwrap();
        tx.send(Bytes::from_static(b"f")).unwrap();
        for _ in 0..6 {
            tx.send(Bytes::from_static(b"g")).unwrap();
        }
        let (out, lag) = coalesce(&mut rx, first);
        assert!(!out.is_empty());
        assert!(lag.is_some(), "lag during coalescing must be reported");
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
        for platform in ["bilibili", "douyin", "huya", "douyu"] {
            let cap = direct_capability(platform);
            assert!(cap.capable, "{platform}");
            assert!(cap.reason.is_none());
        }
        let twitch = direct_capability("twitch");
        assert!(!twitch.capable);
        assert!(twitch.reason.unwrap().contains("Origin"));
        assert!(!direct_capability("youtube").capable);
        // 容器由直链后缀决定播放器：flv → mpegts.js，m3u8 → hls.js
        assert_eq!(
            direct_format("https://x.bilivideo.com/a_2500.flv?expires=1"),
            Some("flv")
        );
        assert_eq!(
            direct_format("https://x.bilivideo.com/live-bvc/1/index.m3u8?expires=1"),
            Some("hls")
        );
        assert_eq!(
            direct_format("https://usher.ttvnw.net/api/channel/hls/x.m3u8?sig=a"),
            Some("hls")
        );
        assert_eq!(direct_format("https://x/y.mp4"), None);
    }

    /// 去抖：5 s 内同一房间复用上一次的直链（标 `cached`），不同房间互不影响，过了窗口就不复用。
    #[test]
    fn live_url_is_debounced_per_room() {
        let sample = |url: &str| LiveUrlResponse {
            url: url.into(),
            format: Some("flv"),
            platform: "douyu".into(),
            expires_at: None,
            direct: DirectCapability {
                capable: true,
                reason: None,
            },
            cached: false,
        };
        let t0 = Instant::now();
        assert!(recent_live_url(9001, t0).is_none());
        remember_live_url(9001, t0, &sample("https://cdn/a.flv?token=1"));
        let hit = recent_live_url(9001, t0 + Duration::from_secs(4)).unwrap();
        assert!(hit.cached);
        assert_eq!(hit.url, "https://cdn/a.flv?token=1");
        assert!(recent_live_url(9002, t0 + Duration::from_secs(1)).is_none());
        assert!(recent_live_url(9001, t0 + LIVE_URL_DEBOUNCE).is_none());
        // 新取的覆盖旧的
        remember_live_url(
            9001,
            t0 + Duration::from_secs(10),
            &sample("https://cdn/a.flv?token=2"),
        );
        assert_eq!(
            recent_live_url(9001, t0 + Duration::from_secs(11))
                .unwrap()
                .url,
            "https://cdn/a.flv?token=2"
        );
        assert!(
            LiveUrlQuery {
                fresh: Some("1".into())
            }
            .wants_fresh()
        );
        assert!(
            LiveUrlQuery {
                fresh: Some("true".into())
            }
            .wants_fresh()
        );
        assert!(
            !LiveUrlQuery {
                fresh: Some("0".into())
            }
            .wants_fresh()
        );
        assert!(!LiveUrlQuery::default().wants_fresh());
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
        // 各测试用自己的信号量，别和并行跑的其它测试抢全局那把
        let limit = Arc::new(Semaphore::new(1));
        let permit = limit.clone().try_acquire_owned().unwrap();
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
        // 响应体读完即 drop，许可随之释放
        assert_eq!(limit.available_permits(), 1);
    }

    /// 复用：两路广播合成一条 SSE，每条事件带各自的 id；两路发送端都 drop 后流才结束。
    #[tokio::test]
    async fn multiplexed_danmaku_tags_each_event_with_its_room_id() {
        use danmaku_client::ChatMessage;
        let (tx_a, rx_a) = tokio::sync::broadcast::channel(8);
        let (tx_b, rx_b) = tokio::sync::broadcast::channel(8);
        // 各测试用自己的信号量，别和并行跑的其它测试抢全局那把
        let limit = Arc::new(Semaphore::new(1));
        let permit = limit.clone().try_acquire_owned().unwrap();
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
        // 响应体读完即 drop，许可随之释放
        assert_eq!(limit.available_permits(), 1);
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

    /// 测试用的进程一级身份：各测试用自己的池，别和并行跑的其它测试抢全局那个
    fn test_guard() -> LiveGuard {
        guard_in(
            &PreviewSlots::new(SlotScope::Process, MAX_PREVIEW_CONNECTIONS),
            &PreviewTicket::new(),
        )
    }

    fn guard_in(process: &PreviewSlots, ticket: &PreviewTicket) -> LiveGuard {
        let (global, _) = process.acquire(ticket, true).unwrap();
        LiveGuard {
            ticket: ticket.clone(),
            _global: global,
        }
    }

    /// 以 `ticket` 订阅；写入端要在请求发出后再推一块才会回应，所以放进任务里
    fn open(
        hub: &PreviewHub,
        ticket: &PreviewTicket,
    ) -> tokio::task::JoinHandle<Result<Subscription, SubscribeError>> {
        let (hub, ticket) = (hub.clone(), ticket.clone());
        tokio::spawn(async move {
            let (slot, _) = hub.reserve(&ticket, true)?;
            hub.subscribe_reserved(slot, None, Duration::from_secs(5))
                .await
        })
    }

    /// 进程级槽位与该路的接收端都随响应体存在，响应体 drop 后立刻释放。
    #[tokio::test]
    async fn global_slot_is_released_with_the_response_body() {
        let process = PreviewSlots::new(SlotScope::Process, MAX_PREVIEW_CONNECTIONS);
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::MpegTs);
        sink.push(ChunkKind::Keyframe, Bytes::from_static(&[0x47, 0, 0, 0]));
        let ticket = PreviewTicket::new();
        let pending = open(&hub, &ticket);
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, Bytes::from_static(&[0x47, 1, 1, 1]));
        let subscription = pending.await.unwrap().unwrap();
        let response = live_response(
            1,
            hub.clone(),
            subscription,
            guard_in(&process, &ticket),
            None,
        );
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "video/mp2t"
        );
        assert_eq!(process.occupied(), 1);
        assert_eq!(hub.subscribers(), 1);
        assert_eq!(sink.receiver_count(), 1);
        drop(response);
        assert_eq!(process.occupied(), 0);
        assert_eq!(hub.subscribers(), 0);
        assert_eq!(sink.receiver_count(), 0);
    }

    /// 同一直播间满员时新打开的预览挤掉最早的那条：旧响应自己正常结束（不是被掐断），
    /// 两级槽位与接收端随之释放，槽位里只剩新来的。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn evicted_response_ends_and_releases_both_slots() {
        let process = PreviewSlots::new(SlotScope::Process, MAX_PREVIEW_CONNECTIONS);
        let hub = PreviewHub::new(1);
        let mut sink = hub.attach(PreviewFormat::Flv);
        sink.push(ChunkKind::Header, Bytes::from_static(&flv::FILE_HEADER));
        sink.push(ChunkKind::Keyframe, Bytes::from_static(b"K1"));
        let old = PreviewTicket::new();
        let pending = open(&hub, &old);
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, Bytes::from_static(b"p2"));
        let subscription = pending.await.unwrap().unwrap();
        let response = live_response(1, hub.clone(), subscription, guard_in(&process, &old), None);
        let reader =
            tokio::spawn(
                async move { axum::body::to_bytes(response.into_body(), usize::MAX).await },
            );
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(process.occupied(), 1);

        let newcomer = PreviewTicket::new();
        let (_slot, evicted) = hub.reserve(&newcomer, true).unwrap();
        assert_eq!(evicted.map(|e| e.id), Some(old.id()));
        let body = tokio::time::timeout(Duration::from_secs(2), reader)
            .await
            .expect("the evicted response must end on its own")
            .unwrap()
            .expect("a clean end of stream, not an error");
        assert!(body.starts_with(&flv::FILE_HEADER));
        assert_eq!(process.occupied(), 0);
        assert_eq!(
            hub.subscribers(),
            1,
            "only the newcomer holds the room slot"
        );
        assert_eq!(sink.receiver_count(), 0, "the evicted receiver is gone");
    }

    /// 到了最长寿命就结束响应，哪怕写入端还在、客户端还在读；两级槽位随之释放。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn response_ends_when_its_lifetime_runs_out() {
        let process = PreviewSlots::new(SlotScope::Process, MAX_PREVIEW_CONNECTIONS);
        let hub = PreviewHub::new(4);
        let mut sink = hub.attach(PreviewFormat::Flv);
        sink.push(ChunkKind::Header, Bytes::from_static(&flv::FILE_HEADER));
        sink.push(ChunkKind::Keyframe, Bytes::from_static(b"K1"));
        let ticket = PreviewTicket::new();
        let pending = open(&hub, &ticket);
        tokio::time::sleep(Duration::from_millis(20)).await;
        sink.push(ChunkKind::Media, Bytes::from_static(b"p2"));
        let subscription = pending.await.unwrap().unwrap();
        let started = Instant::now();
        let response = live_response(
            1,
            hub.clone(),
            subscription,
            guard_in(&process, &ticket),
            Some(Duration::from_millis(300)),
        );
        let body = tokio::time::timeout(
            Duration::from_secs(3),
            axum::body::to_bytes(response.into_body(), usize::MAX),
        )
        .await
        .expect("the response must end at its lifetime")
        .unwrap();
        assert!(started.elapsed() >= Duration::from_millis(300));
        let mut expected = flv::FILE_HEADER.to_vec();
        expected.extend_from_slice(b"K1p2");
        assert_eq!(&body[..], &expected[..]);
        assert_eq!(ticket.evicted_by(), None);
        assert!(hub.is_attached(), "the sink is still there");
        assert_eq!(process.occupied(), 0);
        assert_eq!(hub.subscribers(), 0);
        drop(sink);
    }

    /// 429 的正文写清是哪一级满了（前端原样显示）；只有自动重连才不挤别人。
    #[tokio::test]
    async fn too_many_bodies_name_the_full_scope() {
        let room = too_many_response(SlotScope::Room, 4);
        assert_eq!(room.status(), StatusCode::TOO_MANY_REQUESTS);
        let room = axum::body::to_bytes(room.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&room),
            "该直播间的预览连接数已达上限（每个直播间最多 4 路）"
        );
        let process = too_many_response(SlotScope::Process, MAX_PREVIEW_CONNECTIONS);
        assert_eq!(process.status(), StatusCode::TOO_MANY_REQUESTS);
        let process = axum::body::to_bytes(process.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&process),
            "预览连接数已达上限（进程内最多 16 路）"
        );

        assert!(LiveQuery::default().may_evict());
        let query = |v: &str| LiveQuery {
            snapshot_ms: None,
            reconnect: Some(v.into()),
        };
        assert!(!query("1").may_evict());
        assert!(!query("true").may_evict());
        assert!(query("0").may_evict());
    }

    /// 被挤掉的响应以 chunked 终止块正常收尾，同一条 keep-alive 连接接着能发下一个请求
    /// （不会因为流返回 `None` 卡住连接）。
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn keep_alive_connection_is_reusable_after_an_evicted_response() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        async fn read_until(socket: &mut tokio::net::TcpStream, buf: &mut Vec<u8>, needle: &[u8]) {
            let found = |buf: &[u8]| buf.windows(needle.len()).any(|w| w == needle);
            tokio::time::timeout(Duration::from_secs(3), async {
                let mut chunk = [0u8; 4096];
                while !found(buf) {
                    let n = socket.read(&mut chunk).await.unwrap();
                    assert!(n > 0, "connection closed: {}", String::from_utf8_lossy(buf));
                    buf.extend_from_slice(&chunk[..n]);
                }
            })
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for {needle:?}"));
        }

        let process = PreviewSlots::new(SlotScope::Process, MAX_PREVIEW_CONNECTIONS);
        let hub = PreviewHub::new(1);
        let mut sink = hub.attach(PreviewFormat::Flv);
        sink.push(ChunkKind::Header, Bytes::from_static(&flv::FILE_HEADER));
        sink.push(ChunkKind::Keyframe, Bytes::from_static(b"K1"));
        let live = {
            let (hub, process) = (hub.clone(), process.clone());
            move || async move {
                let ticket = PreviewTicket::new();
                let (slot, _) = hub.reserve(&ticket, true).unwrap();
                let subscription = hub
                    .subscribe_reserved(slot, None, Duration::from_secs(5))
                    .await
                    .unwrap();
                live_response(1, hub, subscription, guard_in(&process, &ticket), None)
            }
        };
        let app = axum::Router::new()
            .route("/live", axum::routing::get(live))
            .route("/ping", axum::routing::get(|| async { "pong" }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let mut socket = tokio::net::TcpStream::connect(addr).await.unwrap();
        socket
            .write_all(b"GET /live HTTP/1.1\r\nHost: test\r\n\r\n")
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        sink.push(ChunkKind::Media, Bytes::from_static(b"p2"));
        let mut buf = Vec::new();
        read_until(&mut socket, &mut buf, b"p2").await;
        let head = String::from_utf8_lossy(&buf).to_ascii_lowercase();
        assert!(head.starts_with("http/1.1 200"), "{head}");
        assert!(head.contains("transfer-encoding: chunked"), "{head}");

        let (_slot, evicted) = hub.reserve(&PreviewTicket::new(), true).unwrap();
        assert!(evicted.is_some());
        read_until(&mut socket, &mut buf, b"\r\n0\r\n\r\n").await;
        assert_eq!(process.occupied(), 0);

        socket
            .write_all(b"GET /ping HTTP/1.1\r\nHost: test\r\n\r\n")
            .await
            .unwrap();
        let mut next = Vec::new();
        read_until(&mut socket, &mut next, b"pong").await;
        assert!(String::from_utf8_lossy(&next).starts_with("HTTP/1.1 200"));
    }
}
