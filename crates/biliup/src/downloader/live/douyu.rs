use super::{
    DanmakuSource, DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus,
    LiveStream, media_ext_from_url,
};
use async_trait::async_trait;
use base64::Engine;
use chrono::Utc;
use rand::Rng;
use rand::seq::SliceRandom;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde::de::Deserializer;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, warn};
use url::{Url, form_urlencoded};

#[path = "douyu_signature.rs"]
mod signature;
const DOUYU_WEB_DOMAIN: &str = "www.douyu.com";
const DOUYU_MOBILE_DOMAIN: &str = "m.douyu.com";
const DOUYU_HUOS_DOMAIN: &str = "openflv-huos.douyucdn2.cn";
const DOUYU_HS_CDN: &str = "hs-h5";
const DOUYU_P2P_DOMAIN_TCT: &str = "hdltctwk.douyucdn.cn";
const DOUYU_P2PSDK_APIS: [&str; 2] = ["https://sdkapiv4.douyucdn.cn", "https://sdkapi.douyucdn.cn"];
const DOUYU_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
/// 网宿按直链里**最后一个** `expire` 限制单条连接寿命（`expire=300` 即 300 s 断一次），
/// `wsAuth` 却只校验**第一个**：保留原值、末尾再追加 `expire=0` 能过校验且不再按时断开，
/// 删掉或改写原值则 403。这是 CDN 未文档化的行为，被 403 时拉流端用
/// [`strip_ws_expire_override`] 退回原直链。
const WS_EXPIRE_OVERRIDE: &str = "&expire=0";

pub struct Douyu {
    re: Regex,
    /// 进程级 url -> 真实房间号缓存（对应 Python get_real_rid 的 alru_cache）
    real_room_id: RwLock<HashMap<String, String>>,
}

impl Default for Douyu {
    fn default() -> Self {
        Self::new()
    }
}

impl Douyu {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"https?://(?:(?:www|m)\.)?douyu\.com").unwrap(),
            real_room_id: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl LivePlugin for Douyu {
    fn name(&self) -> &'static str {
        "Douyu"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        DouyuLive::new(request, &self.real_room_id)
            .check_stream()
            .await
    }
}

struct DouyuLive<'a> {
    client: Client,
    url: String,
    name: String,
    douyu_cdn: String,
    douyu_force_hs: bool,
    douyu_rate: u32,
    douyu_device_id: String,
    douyu_codec: String,
    douyu_disable_interactive_game: bool,
    douyu_danmaku: bool,
    room_id: Option<String>,
    real_room_id_cache: &'a RwLock<HashMap<String, String>>,
}

impl<'a> DouyuLive<'a> {
    fn new(request: LiveRequest, real_room_id_cache: &'a RwLock<HashMap<String, String>>) -> Self {
        let options = request.options.douyu;
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
            douyu_cdn: options.cdn,
            douyu_force_hs: options.force_hs,
            douyu_rate: options.rate,
            douyu_device_id: options.device_id,
            douyu_codec: options.codec,
            douyu_disable_interactive_game: options.disable_interactive_game,
            douyu_danmaku: options.danmaku,
            room_id: None,
            real_room_id_cache,
        }
    }

    async fn check_stream(&mut self) -> LiveResult<LiveStatus> {
        let room_id = self.resolve_room_id().await?;
        self.room_id = Some(room_id.clone());
        let Some(room_info) = self.get_room_info(&room_id).await? else {
            return Ok(LiveStatus::Offline);
        };
        let play_info = self.get_app_play_info(&room_id).await?;
        let raw_stream_url = select_stream_url(play_info, &self.douyu_codec);
        let raw_stream_url = self.maybe_build_huos_url(raw_stream_url).await;
        let raw_stream_url = with_ws_expire_override(raw_stream_url);

        let avatar_url = room_info.avatar_url();
        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: room_info.room_name,
                date: Utc::now(),
                live_cover_url: room_info.room_pic.unwrap_or_default(),
                avatar_url,
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "flv".to_string()),
                raw_stream_url,
                platform: "douyu".to_string(),
                stream_headers: HashMap::new(),
                danmaku: self.danmaku_source(),
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }

    async fn maybe_build_huos_url(&self, raw_stream_url: String) -> String {
        if self.should_build_huos_url() {
            match self.build_huos_url(&raw_stream_url).await {
                Ok(huos_url) => huos_url,
                Err(err) => {
                    warn!(
                        error = ?err,
                        url = raw_stream_url,
                        "failed to build Douyu huos URL, falling back to original stream URL"
                    );
                    raw_stream_url
                }
            }
        } else {
            raw_stream_url
        }
    }

    async fn resolve_room_id(&self) -> LiveResult<String> {
        if let Ok(parsed) = Url::parse(&self.url)
            && let Some(rid) = parsed
                .query_pairs()
                .find(|(key, _)| key == "rid")
                .map(|(_, value)| value.to_string())
            && rid.chars().all(|ch| ch.is_ascii_digit())
        {
            return Ok(rid);
        }

        let Some(short_id) = self
            .url
            .split("douyu.com/")
            .nth(1)
            .and_then(|part| part.split('/').next())
            .and_then(|part| part.split('?').next())
            .filter(|part| !part.is_empty())
        else {
            return Err(LiveError::custom("直播间地址错误"));
        };

        // 对应 Python get_real_rid 的 alru_cache：同一 url 只请求一次移动端页面
        if let Some(rid) = self.real_room_id_cache.read().await.get(&self.url) {
            return Ok(rid.clone());
        }

        let mobile_url = format!("https://{DOUYU_MOBILE_DOMAIN}/{short_id}");
        let text = self
            .client
            .get(mobile_url)
            .header("user-agent", DOUYU_USER_AGENT)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取斗鱼真实房间号失败: {err}")))?
            .text()
            .await
            .map_err(|err| LiveError::custom(format!("读取斗鱼房间页面失败: {err}")))?;

        if let Some(caps) = Regex::new(r#"roomInfo":\{"rid":(\d+)"#)
            .unwrap()
            .captures(&text)
        {
            let rid = caps[1].to_string();
            self.real_room_id_cache
                .write()
                .await
                .insert(self.url.clone(), rid.clone());
            return Ok(rid);
        }

        if short_id.chars().all(|ch| ch.is_ascii_digit()) {
            return Ok(short_id.to_string());
        }

        Err(LiveError::custom("获取斗鱼房间号错误"))
    }

    async fn get_room_info(&self, room_id: &str) -> LiveResult<Option<RoomInfo>> {
        // 对应 douyu.py：网络层错误重试 3 次，缓解 #1376 海外请求失败问题；
        // 非网络错误（如 JSON 解析失败）不重试
        let mut body = None;
        let mut last_error = None;
        for _ in 0..3 {
            match self.fetch_room_info_text(room_id).await {
                Ok(text) => {
                    body = Some(text);
                    break;
                }
                Err(err) => {
                    debug!(error = ?err, room_id, "请求斗鱼直播间信息失败，重试");
                    last_error = Some(err);
                }
            }
        }
        let Some(body) = body else {
            let err = last_error.expect("betard retry loop runs at least once");
            return Err(LiveError::custom(format!(
                "获取斗鱼直播间信息失败 room_id: {room_id}: {err}"
            )));
        };

        let resp: BetardResponse = serde_json::from_str(&body).map_err(|err| {
            LiveError::custom(format!("解析斗鱼直播间信息失败 room_id: {room_id}: {err}"))
        })?;

        let Some(room) = resp.room else {
            return Ok(None);
        };
        if room.show_status != 1 {
            return Ok(None);
        }
        if self.douyu_disable_interactive_game && self.has_interactive_game(room_id).await? {
            return Ok(None);
        }
        Ok(Some(room))
    }

    async fn fetch_room_info_text(&self, room_id: &str) -> Result<String, reqwest::Error> {
        self.client
            .get(format!("https://{DOUYU_WEB_DOMAIN}/betard/{room_id}"))
            .header("referer", format!("https://{DOUYU_WEB_DOMAIN}"))
            .send()
            .await?
            .text()
            .await
    }

    async fn has_interactive_game(&self, room_id: &str) -> LiveResult<bool> {
        let data: Value = self
            .client
            .get(format!(
                "https://{DOUYU_WEB_DOMAIN}/api/interactive/web/v2/list?rid={room_id}"
            ))
            .header("referer", format!("https://{DOUYU_WEB_DOMAIN}"))
            .header("user-agent", DOUYU_USER_AGENT)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取斗鱼互动游戏信息失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析斗鱼互动游戏信息失败: {err}")))?;

        Ok(data
            .get("data")
            .map(|data| match data {
                Value::Null => false,
                Value::Array(items) => !items.is_empty(),
                Value::Object(map) => !map.is_empty(),
                Value::String(value) => !value.is_empty(),
                Value::Bool(value) => *value,
                Value::Number(_) => true,
            })
            .unwrap_or(false))
    }

    async fn get_app_play_info(&self, room_id: &str) -> LiveResult<PlayInfo> {
        let room_number: u32 = room_id
            .parse()
            .map_err(|_| LiveError::custom("斗鱼房间号无效"))?;
        let device_id = if self.douyu_device_id.trim().is_empty() {
            signature::DEFAULT_DEVICE_ID
        } else {
            self.douyu_device_id.trim()
        };
        if device_id.len() > 36 || !device_id.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return Err(LiveError::custom(
                "douyu_deviceId 应为 Cookie acf_did 的值（不超过 36 位字母或数字）",
            ));
        }
        let path = format!("/lapi/live/appGetPlayer/stream/{room_id}");
        let timestamp = unix_now()?;
        let device = random_android_device();
        let mut params = std::collections::BTreeMap::new();
        params.insert("txdw".to_string(), "0".to_string()); // 腾讯大王卡免流
        params.insert(
            "cdn".to_string(),
            self.douyu_cdn.trim_end_matches("-h5").to_string(),
        );
        params.insert("token".to_string(), String::new()); // 已登录账号Token
        params.insert("rate".to_string(), self.douyu_rate.to_string());
        params.insert("hevc".to_string(), "1".to_string()); // 设备 Codec 偏好
        params.insert("ilow".to_string(), "0".to_string()); // 低端设备
        params.insert("iar".to_string(), "0".to_string()); // 首屏加载（非 0 时忽略 rate，提供最低画质）
        params.insert("net".to_string(), "WIFI".to_string());
        params.insert("device".to_string(), device.clone());
        let csign = signature::csign(room_number, device_id, timestamp, &params);
        let amd = signature::amd(&csign, device_id);
        params.insert("csign".to_string(), csign);
        params.insert("cptl".to_string(), "0103".to_string());
        params.insert("amd".to_string(), amd);
        params.insert("client_sys".to_string(), "android".to_string());
        let auth = signature::header_auth(&path, timestamp, "android1", &params);
        let user_device =
            base64::engine::general_purpose::STANDARD.encode(format!("{device_id}|v8.2.2.0"));
        let rsp = self
            .client
            .get(format!("https://playclient.douyucdn.cn{path}"))
            .query(&params)
            .header("User-Device", user_device)
            .header("aid", "android1")
            .header("channel", "447") // 安装包下载渠道（447: H5移动端下载页）
            .header(
                "User-Agent",
                format!("android/8.2.2.0 (android 16; ; {device})"),
            )
            .header("time", timestamp.to_string())
            .header("auth", auth)
            .header("Cookie", format!("acf_did={device_id}"))
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("请求斗鱼播放信息失败: {err}")))?;
        let status = rsp.status();
        let body = rsp
            .text()
            .await
            .map_err(|err| LiveError::custom(format!("读取斗鱼播放信息失败: {err}")))?;
        let parsed: PlayResponse = serde_json::from_str(&body).map_err(|err| {
            LiveError::custom(format!("解析斗鱼播放信息失败 (HTTP {status}): {err}"))
        })?;
        play_info_from_response(parsed)
    }

    fn should_build_huos_url(&self) -> bool {
        self.douyu_force_hs && self.douyu_cdn.eq_ignore_ascii_case(DOUYU_HS_CDN)
    }

    async fn build_huos_url(&self, raw_stream_url: &str) -> LiveResult<String> {
        let (stream_id, params) = parse_stream_url(raw_stream_url)?;
        let tx_secret = self.get_txsecret(&stream_id).await?;
        Ok(build_huos_url(&stream_id, &params, &tx_secret))
    }

    async fn get_txsecret(&self, stream_id: &str) -> LiveResult<XP2PTxSecret> {
        let mut apis = DOUYU_P2PSDK_APIS;
        {
            let mut rng = rand::thread_rng();
            apis.shuffle(&mut rng);
        }

        let mut last_error = None;
        for api in apis {
            match self.request_txsecret(api, stream_id).await {
                Ok(tx_secret) => return Ok(tx_secret),
                Err(err) => last_error = Some(err),
            }
        }

        Err(last_error
            .unwrap_or_else(|| LiveError::custom(format!("获取 txSecret 失败: {stream_id}"))))
    }

    async fn request_txsecret(&self, api: &str, stream_id: &str) -> LiveResult<XP2PTxSecret> {
        let tx_secret: XP2PTxSecret = self
            .client
            .get(format!("{api}/p2p/get_txsecret"))
            .query(&[("lid", stream_id)])
            .header("user-agent", DOUYU_USER_AGENT)
            .send()
            .await
            .map_err(|err| {
                LiveError::custom(format!(
                    "获取 txSecret 失败 api: {api}, stream_id: {stream_id}: {err}"
                ))
            })?
            .error_for_status()
            .map_err(|err| {
                LiveError::custom(format!(
                    "获取 txSecret 响应异常 api: {api}, stream_id: {stream_id}: {err}"
                ))
            })?
            .json()
            .await
            .map_err(|err| {
                LiveError::custom(format!(
                    "解析 txSecret 失败 api: {api}, stream_id: {stream_id}: {err}"
                ))
            })?;

        if tx_secret.tx_secret.is_empty() || tx_secret.tx_time.is_empty() {
            return Err(LiveError::custom(format!("txSecret 为空: {stream_id}")));
        }

        Ok(tx_secret)
    }

    fn danmaku_source(&self) -> Option<DanmakuSource> {
        if !self.douyu_danmaku {
            return None;
        }
        Some(DanmakuSource {
            platform: "douyu".to_string(),
            url: self.url.clone(),
            room_id: self.room_id.clone(),
            cookie: None,
            raw: false,
            detail: false,
            extra: HashMap::new(),
            movie_id: None,
            password: None,
        })
    }
}

fn unix_now() -> LiveResult<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| LiveError::custom(format!("获取系统时间失败: {err}")))?
        .as_secs())
}

/// 伪装 HUAWEI 设备
fn random_android_device() -> String {
    let mut rng = rand::thread_rng();
    let letter = |rng: &mut rand::rngs::ThreadRng| (b'A' + rng.gen_range(0..26)) as char;
    format!(
        "{}{}{}-{}{}{}{}",
        letter(&mut rng),
        letter(&mut rng),
        letter(&mut rng),
        letter(&mut rng),
        letter(&mut rng),
        rng.gen_range(0..10),
        rng.gen_range(0..10)
    )
}

fn select_stream_url(play_info: PlayInfo, codec: &str) -> String {
    if codec == "HEVC" {
        if let Some(url) = play_info.player_1.filter(|url| !url.trim().is_empty()) {
            return url;
        }
        warn!("斗鱼未提供 HEVC 流，回退到 AVC");
    }
    format!(
        "{}/{}",
        play_info.rtmp_url.trim_end_matches('/'),
        play_info.rtmp_live.trim_start_matches('/')
    )
}

fn deserialize_optional_player_url<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| value.as_str().map(str::to_owned)))
}

fn play_info_from_response(response: PlayResponse) -> LiveResult<PlayInfo> {
    match response.error {
        0 => response
            .data
            .ok_or_else(|| LiveError::custom("斗鱼播放信息缺少 data")),
        -5 => Err(LiveError::custom("[closeRoom] 主播未开播")),
        -9 => Err(LiveError::custom(
            "[room_bus_checksevertime] 用户本机时间戳不对",
        )),
        126 => Err(LiveError::custom(format!(
            "版权原因，该地域不允许播放：{}",
            response.msg.unwrap_or_default()
        ))),
        _ => Err(LiveError::custom(format!(
            "斗鱼播放信息错误: code={}, msg={}",
            response.error,
            response.msg.unwrap_or_default()
        ))),
    }
}

/// 斗鱼错误响应里 `data` 常为 `""` 而非 `null`/`object`（#1680），
/// 需容忍字符串/空值，否则 serde 在走到 error==-9/-5 分支前就失败。
fn deserialize_optional_play_info<'de, D>(deserializer: D) -> Result<Option<PlayInfo>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) if s.is_empty() => Ok(None),
        Some(Value::String(_)) => Ok(None),
        Some(obj @ Value::Object(_)) => serde_json::from_value(obj)
            .map(Some)
            .map_err(serde::de::Error::custom),
        Some(_) => Ok(None),
    }
}

/// 网宿直链：host 为 `ws*.douyucdn.cn`，或带 `fcdn=ws`。
/// 不看 `rtmp_cdn`：`douyu_force_hs` 构造火山直链后它仍是 `ws-h5`。
fn is_wangsu_stream(url: &Url) -> bool {
    let ws_host = url.host_str().is_some_and(|host| {
        host.starts_with("ws")
            && (host.ends_with(".douyucdn.cn") || host.ends_with(".douyucdn2.cn"))
    });
    ws_host
        || url
            .query_pairs()
            .any(|(key, value)| key == "fcdn" && value == "ws")
}

/// 网宿直链里只有一个 `expire` 且不为 0 时才需要追加。
fn ws_expire_needs_override(url: &str) -> bool {
    let Ok(parsed) = Url::parse(url) else {
        return false;
    };
    if !is_wangsu_stream(&parsed) {
        return false;
    }
    let mut expires = parsed.query_pairs().filter(|(key, _)| key == "expire");
    matches!((expires.next(), expires.next()), (Some((_, value)), None) if value != "0")
}

fn with_ws_expire_override(url: String) -> String {
    if ws_expire_needs_override(&url) {
        url + WS_EXPIRE_OVERRIDE
    } else {
        url
    }
}

/// 若是追加过 `expire=0` 的网宿直链，返回追加前的原直链；否则 `None`。
pub fn strip_ws_expire_override(url: &str) -> Option<&str> {
    url.strip_suffix(WS_EXPIRE_OVERRIDE)
        .filter(|original| ws_expire_needs_override(original))
}

fn parse_stream_url(input: &str) -> LiveResult<(String, Vec<(String, String)>)> {
    let parsed = Url::parse(input)
        .map_err(|err| LiveError::custom(format!("解析斗鱼 huos 源链接失败: {err}")))?;
    let stream_name = parsed
        .path()
        .rsplit('/')
        .find(|part| !part.is_empty())
        .ok_or_else(|| LiveError::custom("斗鱼 huos 源链接缺少 stream_id"))?;
    let stream_id = stream_name
        .split_once('.')
        .map(|(stream_id, _)| stream_id)
        .unwrap_or(stream_name);
    if stream_id.is_empty() {
        return Err(LiveError::custom("斗鱼 huos 源链接 stream_id 为空"));
    }
    let params = parsed
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    Ok((stream_id.to_string(), params))
}

fn build_huos_url(
    stream_id: &str,
    params: &[(String, String)],
    tx_secret: &XP2PTxSecret,
) -> String {
    let mut next_params = Vec::with_capacity(params.len() + 3);
    let mut has_fcdn = false;

    for (key, value) in params {
        if matches!(key.as_str(), "txSecret" | "txTime" | "domain") {
            continue;
        }
        if key == "fcdn" {
            if !has_fcdn {
                next_params.push((key.clone(), "hs".to_string()));
                has_fcdn = true;
            }
            continue;
        }
        next_params.push((key.clone(), value.clone()));
    }

    if !has_fcdn {
        next_params.push(("fcdn".to_string(), "hs".to_string()));
    }
    next_params.push(("txSecret".to_string(), tx_secret.tx_secret.clone()));
    next_params.push(("txTime".to_string(), tx_secret.tx_time.clone()));
    next_params.push(("domain".to_string(), DOUYU_P2P_DOMAIN_TCT.to_string()));

    let mut serializer = form_urlencoded::Serializer::new(String::new());
    for (key, value) in next_params {
        serializer.append_pair(&key, &value);
    }
    let query = serializer.finish();

    format!("http://{DOUYU_HUOS_DOMAIN}/live/{stream_id}.xs?{query}")
}

#[derive(Deserialize)]
struct BetardResponse {
    room: Option<RoomInfo>,
}

#[derive(Deserialize)]
struct RoomInfo {
    room_name: String,
    show_status: i64,
    /// 直播间封面（betard 里的 `room_pic`，通常是 avif），缺失时为空
    #[serde(default)]
    room_pic: Option<String>,
    /// 主播头像直链
    #[serde(default)]
    owner_avatar: Option<String>,
    /// 头像的多尺寸版本，形如 `{"big": ..., "middle": ..., "small": ...}`；
    /// 个别房间只给这一份，作为 `owner_avatar` 缺失时的后备
    #[serde(default)]
    avatar: Option<Value>,
}

impl RoomInfo {
    fn avatar_url(&self) -> Option<String> {
        let non_empty = |url: &str| (!url.is_empty()).then(|| url.to_string());
        self.owner_avatar
            .as_deref()
            .and_then(non_empty)
            .or_else(|| {
                let avatar = self.avatar.as_ref()?;
                ["middle", "big", "small"]
                    .iter()
                    .filter_map(|size| avatar.get(size).and_then(Value::as_str))
                    .find_map(non_empty)
                    .or_else(|| avatar.as_str().and_then(non_empty))
            })
    }
}

#[derive(Deserialize)]
struct PlayResponse {
    error: i64,
    msg: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_play_info")]
    data: Option<PlayInfo>,
}

#[derive(Deserialize)]
struct PlayInfo {
    rtmp_url: String,
    rtmp_live: String,
    #[serde(default, deserialize_with = "deserialize_optional_player_url")]
    player_1: Option<String>,
}

#[derive(Deserialize)]
struct XP2PTxSecret {
    #[serde(rename = "xp2p_txSecret")]
    tx_secret: String,
    #[serde(rename = "xp2p_txTime")]
    tx_time: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn android_device_has_expected_format() {
        for _ in 0..20 {
            let device = random_android_device();
            let bytes = device.as_bytes();
            assert_eq!(bytes.len(), 8);
            assert_eq!(bytes[3], b'-');
            assert!(
                bytes[..3]
                    .iter()
                    .chain(&bytes[4..6])
                    .all(u8::is_ascii_uppercase)
            );
            assert!(bytes[6..].iter().all(u8::is_ascii_digit));
        }
    }

    #[test]
    fn codec_selects_expected_stream_field() {
        let info = || PlayInfo {
            rtmp_url: "https://cdn.example/live/".into(),
            rtmp_live: "/avc.flv".into(),
            player_1: Some("https://cdn.example/hevc.flv".into()),
        };
        for codec in ["", "AVC", "unexpected"] {
            assert_eq!(
                select_stream_url(info(), codec),
                "https://cdn.example/live/avc.flv"
            );
        }
        assert_eq!(
            select_stream_url(info(), "HEVC"),
            "https://cdn.example/hevc.flv"
        );
        for player_1 in [None, Some(String::new()), Some("  ".into())] {
            let mut play_info = info();
            play_info.player_1 = player_1;
            assert_eq!(
                select_stream_url(play_info, "HEVC"),
                "https://cdn.example/live/avc.flv"
            );
        }
        let null_player: PlayInfo = serde_json::from_str(
            r#"{"rtmp_url":"https://cdn.example/live","rtmp_live":"avc.flv","player_1":null}"#,
        )
        .unwrap();
        assert_eq!(
            select_stream_url(null_player, "HEVC"),
            "https://cdn.example/live/avc.flv"
        );
        let invalid_player: PlayInfo = serde_json::from_str(
            r#"{"rtmp_url":"https://cdn.example/live","rtmp_live":"avc.flv","player_1":false}"#,
        )
        .unwrap();
        assert_eq!(
            select_stream_url(invalid_player, "HEVC"),
            "https://cdn.example/live/avc.flv"
        );
        let sample = PlayInfo {
            rtmp_url: "http://hwa.douyucdn2.cn/live".into(),
            rtmp_live: "48699rh53o1mcUfL.flv?wsAuth=".into(),
            player_1: None,
        };
        assert_eq!(
            select_stream_url(sample, "HEVC"),
            "http://hwa.douyucdn2.cn/live/48699rh53o1mcUfL.flv?wsAuth="
        );
    }

    fn make_live<'a>(
        url: &str,
        real_room_id_cache: &'a RwLock<HashMap<String, String>>,
    ) -> DouyuLive<'a> {
        DouyuLive {
            client: Client::new(),
            url: url.to_string(),
            name: "test".to_string(),
            douyu_cdn: DOUYU_HS_CDN.to_string(),
            douyu_force_hs: true,
            douyu_rate: 0,
            douyu_device_id: String::new(),
            douyu_codec: String::new(),
            douyu_disable_interactive_game: false,
            douyu_danmaku: false,
            room_id: None,
            real_room_id_cache,
        }
    }

    #[test]
    fn build_huos_url_rewrites_fcdn_and_appends_secret() {
        let (stream_id, params) = parse_stream_url(
            "https://hw3.douyucdn2.cn/live/6925114rIDrEEuKo.flv?wsAuth=auth&fcdn=hw&isp=",
        )
        .unwrap();
        let tx_secret = XP2PTxSecret {
            tx_secret: "secret".to_string(),
            tx_time: "time".to_string(),
        };

        let url = build_huos_url(&stream_id, &params, &tx_secret);

        assert_eq!(
            url,
            "http://openflv-huos.douyucdn2.cn/live/6925114rIDrEEuKo.xs?wsAuth=auth&fcdn=hs&isp=&txSecret=secret&txTime=time&domain=hdltctwk.douyucdn.cn"
        );
    }

    #[test]
    fn build_huos_url_removes_existing_secret_params() {
        let (stream_id, params) = parse_stream_url(
            "https://hw3.douyucdn2.cn/live/abc.flv?fcdn=hw&txSecret=old&txTime=old&domain=old.example",
        )
        .unwrap();
        let tx_secret = XP2PTxSecret {
            tx_secret: "new".to_string(),
            tx_time: "next".to_string(),
        };

        let url = build_huos_url(&stream_id, &params, &tx_secret);

        assert_eq!(
            url,
            "http://openflv-huos.douyucdn2.cn/live/abc.xs?fcdn=hs&txSecret=new&txTime=next&domain=hdltctwk.douyucdn.cn"
        );
    }

    #[test]
    fn build_huos_url_adds_missing_fcdn() {
        let (stream_id, params) =
            parse_stream_url("https://hw3.douyucdn2.cn/live/abc.flv?token=value").unwrap();
        let tx_secret = XP2PTxSecret {
            tx_secret: "secret".to_string(),
            tx_time: "time".to_string(),
        };

        let url = build_huos_url(&stream_id, &params, &tx_secret);

        assert_eq!(
            url,
            "http://openflv-huos.douyucdn2.cn/live/abc.xs?token=value&fcdn=hs&txSecret=secret&txTime=time&domain=hdltctwk.douyucdn.cn"
        );
    }

    const WS_URL: &str = "https://ws1a.douyucdn.cn/live/24422abc_4000.flv?wsAuth=a1&token=t1&logo=0&expire=300&did=d&origin=dy&fcdn=ws&fo=0&mix=0&isp=";

    #[test]
    fn ws_expire_override_appends_after_original_expire() {
        let overridden = with_ws_expire_override(WS_URL.to_string());
        assert_eq!(overridden, format!("{WS_URL}&expire=0"));
        assert_eq!(strip_ws_expire_override(&overridden), Some(WS_URL));
        // 已追加过的不再追加
        assert_eq!(with_ws_expire_override(overridden.clone()), overridden);
        // 只靠 fcdn=ws 也能认出网宿
        let by_fcdn = "https://cdn.example/live/a.flv?wsAuth=a&expire=300&fcdn=ws";
        assert_eq!(
            with_ws_expire_override(by_fcdn.to_string()),
            format!("{by_fcdn}&expire=0")
        );
    }

    #[test]
    fn ws_expire_override_skips_expire_zero() {
        let url = WS_URL.replace("expire=300", "expire=0");
        assert_eq!(with_ws_expire_override(url.clone()), url);
        assert_eq!(strip_ws_expire_override(&url), None);
    }

    #[test]
    fn ws_expire_override_skips_non_wangsu() {
        for url in [
            "https://hw3.douyucdn2.cn/live/abc_4000.flv?wsAuth=a&token=t&expire=300&fcdn=hw",
            "http://openflv-huos.douyucdn2.cn/live/abc_4000.xs?wsAuth=a&expire=300&fcdn=hs&txSecret=s&txTime=t&domain=hdltctwk.douyucdn.cn",
            "https://tc-tct.douyucdn2.cn/dyliveflv1/abc_4000.flv?wsAuth=a&expire=300&fcdn=tct",
        ] {
            assert_eq!(with_ws_expire_override(url.to_string()), url);
            let appended = format!("{url}&expire=0");
            assert_eq!(strip_ws_expire_override(&appended), None);
        }
    }

    #[test]
    fn ws_expire_override_skips_url_without_expire() {
        let url = "https://ws1a.douyucdn.cn/live/abc_4000.flv?wsAuth=a&token=t&fcdn=ws";
        assert_eq!(with_ws_expire_override(url.to_string()), url);
        assert_eq!(strip_ws_expire_override(&format!("{url}&expire=0")), None);
    }

    #[tokio::test]
    async fn maybe_build_huos_url_falls_back_when_build_fails() {
        let real_room_id_cache = RwLock::new(HashMap::new());
        let live = make_live("https://www.douyu.com/10568722", &real_room_id_cache);

        let raw_stream_url = "not a url".to_string();

        assert_eq!(
            live.maybe_build_huos_url(raw_stream_url.clone()).await,
            raw_stream_url
        );
    }

    #[tokio::test]
    async fn resolve_room_id_uses_cached_real_room_id() {
        let url = "https://www.douyu.com/somename";
        let real_room_id_cache =
            RwLock::new(HashMap::from([(url.to_string(), "10568722".to_string())]));
        let live = make_live(url, &real_room_id_cache);

        // 命中缓存时不请求移动端页面
        assert_eq!(live.resolve_room_id().await.unwrap(), "10568722");
    }

    #[tokio::test]
    async fn resolve_room_id_prefers_rid_query_param() {
        let real_room_id_cache = RwLock::new(HashMap::new());
        let live = make_live(
            "https://www.douyu.com/topic/xyz?rid=123456",
            &real_room_id_cache,
        );

        assert_eq!(live.resolve_room_id().await.unwrap(), "123456");
    }

    #[test]
    fn play_response_tolerates_empty_string_data() {
        // #1680：错误响应 data:"" 必须能反序列化，才能走到 error==-9 分支
        let rsp: PlayResponse =
            serde_json::from_str(r#"{"error":-9,"msg":"时间戳错误","data":""}"#).unwrap();
        assert_eq!(rsp.error, -9);
        assert_eq!(rsp.msg.as_deref(), Some("时间戳错误"));
        assert!(rsp.data.is_none());
    }

    #[test]
    fn play_response_tolerates_null_data() {
        let rsp: PlayResponse =
            serde_json::from_str(r#"{"error":-5,"msg":"房间未开播","data":null}"#).unwrap();
        assert_eq!(rsp.error, -5);
        assert!(rsp.data.is_none());
    }

    #[test]
    fn app_play_errors_keep_specific_messages() {
        for (body, expected) in [
            (
                r#"{"error":-5,"msg":"closeRoom","data":""}"#,
                "[closeRoom] 主播未开播",
            ),
            (
                r#"{"error":-9,"msg":"invalid timestamp","data":null}"#,
                "[room_bus_checksevertime] 用户本机时间戳不对",
            ),
            (
                r#"{"error":126,"msg":"区域限制","data":null}"#,
                "版权原因，该地域不允许播放：区域限制",
            ),
        ] {
            let response: PlayResponse = serde_json::from_str(body).unwrap();
            assert_eq!(
                play_info_from_response(response)
                    .err()
                    .expect("error")
                    .to_string(),
                expected
            );
        }
    }

    #[test]
    fn play_response_deserializes_valid_play_info() {
        let rsp: PlayResponse = serde_json::from_str(
            r#"{"error":0,"msg":"","data":{"rtmp_url":"https://example.com","rtmp_live":"live/abc.flv","player_1":"https://example.com/hevc.flv"}}"#,
        )
        .unwrap();
        assert_eq!(rsp.error, 0);
        let data = rsp.data.expect("play info");
        assert_eq!(data.rtmp_url, "https://example.com");
        assert_eq!(data.rtmp_live, "live/abc.flv");
        assert_eq!(
            data.player_1.as_deref(),
            Some("https://example.com/hevc.flv")
        );
    }

    /// betard 的 room 对象里 `room_pic` 是封面、`owner_avatar` / `avatar.{big,middle,small}` 是头像；
    /// 老样本没有这些键时仍要能反序列化
    #[test]
    fn betard_room_exposes_cover_and_avatar_with_fallbacks() {
        let full: BetardResponse = serde_json::from_str(
            r#"{"room":{"room_name":"n","show_status":1,"videoLoop":0,
                "room_pic":"https://rpic.douyucdn.cn/asrpic/260922/1_src.avif/dy4",
                "owner_avatar":"https://apic.douyucdn.cn/face_big.jpg",
                "avatar":{"big":"https://apic.douyucdn.cn/face_big.jpg","middle":"https://apic.douyucdn.cn/face_middle.jpg","small":""}}}"#,
        )
        .unwrap();
        let room = full.room.unwrap();
        assert_eq!(
            room.room_pic.as_deref(),
            Some("https://rpic.douyucdn.cn/asrpic/260922/1_src.avif/dy4")
        );
        assert_eq!(
            room.avatar_url().as_deref(),
            Some("https://apic.douyucdn.cn/face_big.jpg")
        );

        let only_sizes: BetardResponse = serde_json::from_str(
            r#"{"room":{"room_name":"n","show_status":1,"videoLoop":0,
                "avatar":{"big":"","middle":"https://apic.douyucdn.cn/face_middle.jpg"}}}"#,
        )
        .unwrap();
        assert_eq!(
            only_sizes.room.unwrap().avatar_url().as_deref(),
            Some("https://apic.douyucdn.cn/face_middle.jpg")
        );

        let legacy: BetardResponse =
            serde_json::from_str(r#"{"room":{"room_name":"n","show_status":1,"videoLoop":0}}"#)
                .unwrap();
        let room = legacy.room.unwrap();
        assert_eq!(room.room_pic, None);
        assert_eq!(room.avatar_url(), None);
    }
}
