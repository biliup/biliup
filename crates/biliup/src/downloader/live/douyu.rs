use super::{
    DanmakuSource, DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus,
    LiveStream, media_ext_from_url,
};
use async_trait::async_trait;
use base64::Engine;
use chrono::Utc;
use md5::{Digest, Md5};
use rand::Rng;
use rand::seq::SliceRandom;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde::de::Deserializer;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{debug, warn};
use url::{Url, form_urlencoded};

#[path = "douyu_cookie.rs"]
mod cookie;
#[path = "douyu_refresh.rs"]
mod refresh;
pub use refresh::{
    DouyuCookieInput, DouyuCookieRefresh, DouyuLoginIdentity, DouyuRefreshClient, DouyuRefreshError,
};
#[path = "douyu_signature.rs"]
mod signature;
const DOUYU_WEB_DOMAIN: &str = "www.douyu.com";
const DOUYU_WEB_DEVICE_ID: &str = "10000000000000000000000000001501";
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

    pub async fn validate_cookie(cookie: &str, client: &Client) -> LiveResult<bool> {
        cookie::validate_cookie(cookie, client).await
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
    douyu_cookie: Option<String>,
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
            douyu_cookie: request.credentials.douyu_cookie,
            room_id: None,
            real_room_id_cache,
        }
    }

    async fn check_stream(&mut self) -> LiveResult<LiveStatus> {
        self.douyu_cookie = self
            .douyu_cookie
            .as_deref()
            .map(cookie::normalize_cookie_header)
            .transpose()?
            .filter(|header| !header.is_empty());
        let room_id = self.resolve_room_id().await?;
        self.room_id = Some(room_id.clone());
        let Some(room_info) = self.get_room_info(&room_id).await? else {
            return Ok(LiveStatus::Offline);
        };
        let play_info = self.get_play_info(&room_id).await?;
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

        if let Some(rid) = room_id_from_mobile_page(&text) {
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
        if !room.is_live() {
            return Ok(None);
        }
        if self.douyu_disable_interactive_game && self.has_interactive_game(room_id).await? {
            return Ok(None);
        }
        Ok(Some(room))
    }

    async fn fetch_room_info_text(&self, room_id: &str) -> Result<String, reqwest::Error> {
        let mut request = self
            .client
            .get(format!("https://{DOUYU_WEB_DOMAIN}/betard/{room_id}"))
            .header("referer", format!("https://{DOUYU_WEB_DOMAIN}"))
            .header("user-agent", DOUYU_USER_AGENT);

        // 如果提供了cookie，添加到请求头以获取高清流质量
        if let Some(ref cookie) = self.douyu_cookie {
            request = request.header("cookie", cookie);
        }

        request.send().await?.text().await
    }

    async fn has_interactive_game(&self, room_id: &str) -> LiveResult<bool> {
        let mut request = self
            .client
            .get(format!(
                "https://{DOUYU_WEB_DOMAIN}/api/interactive/web/v2/list?rid={room_id}"
            ))
            .header("referer", format!("https://{DOUYU_WEB_DOMAIN}"))
            .header("user-agent", DOUYU_USER_AGENT);

        // 添加Cookie以获取完整的互动游戏信息
        if let Some(ref cookie) = self.douyu_cookie {
            request = request.header("cookie", cookie);
        }

        let data: Value = request
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

    async fn get_play_info(&self, room_id: &str) -> LiveResult<PlayInfo> {
        self.get_play_info_at(
            room_id,
            &format!("https://{DOUYU_WEB_DOMAIN}"),
            "https://playclient.douyucdn.cn",
        )
        .await
    }

    async fn get_play_info_at(
        &self,
        room_id: &str,
        web_origin: &str,
        app_origin: &str,
    ) -> LiveResult<PlayInfo> {
        let play_info = match self.get_web_play_response_at(room_id, web_origin).await {
            Ok(response) if response.error == 0 && response.data.is_none() => {
                let error = LiveError::custom("斗鱼网页播放信息缺少 data");
                warn!(
                    room_id,
                    error = %error,
                    "斗鱼网页播放接口返回不完整响应，回退 App 接口"
                );
                self.get_app_play_info_at(room_id, app_origin).await?
            }
            Ok(response) => play_info_from_response(response)?,
            Err(WebPlayError::Rejected(error)) => return Err(error),
            Err(WebPlayError::Protocol(error)) => {
                warn!(
                    room_id,
                    error = %error,
                    "斗鱼网页播放接口请求失败，回退 App 接口；App 接口可能限制登录画质"
                );
                self.get_app_play_info_at(room_id, app_origin).await?
            }
        };
        self.log_stream_quality(room_id, &play_info);
        Ok(play_info)
    }

    fn device_id(&self, web: bool) -> LiveResult<&str> {
        let configured = self.douyu_device_id.trim();
        let device_id = if !configured.is_empty() {
            configured
        } else {
            self.douyu_cookie
                .as_deref()
                .and_then(|header| {
                    if web {
                        cookie::cookie_value(header, "dy_did")
                            .or_else(|| cookie::cookie_value(header, "acf_did"))
                    } else {
                        cookie::cookie_value(header, "acf_did")
                    }
                })
                .unwrap_or(if web {
                    DOUYU_WEB_DEVICE_ID
                } else {
                    signature::DEFAULT_DEVICE_ID
                })
        };
        if device_id.len() > 36 || !device_id.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return Err(LiveError::custom(
                "斗鱼设备 ID 应为不超过 36 位的字母或数字",
            ));
        }
        Ok(device_id)
    }

    async fn get_web_play_response_at(
        &self,
        room_id: &str,
        web_origin: &str,
    ) -> Result<PlayResponse, WebPlayError> {
        let room_number: u32 = room_id
            .parse()
            .map_err(|_| WebPlayError::Rejected(LiveError::custom("斗鱼房间号无效")))?;
        let device_id = self.device_id(true).map_err(WebPlayError::Rejected)?;
        // Browser exports can include unrelated application and tracking cookies.
        // Sending those to the playback API can trigger HTTP 403 even with a valid
        // login, causing the anonymous App fallback to lose access to original quality.
        let playback_cookie = self
            .douyu_cookie
            .as_deref()
            .map(web_playback_cookie_header)
            .filter(|header| !header.is_empty());
        let referer = format!("{web_origin}/{room_id}");
        let mut key_request = self
            .client
            .get(format!(
                "{web_origin}/wgapi/livenc/liveweb/websec/getEncryption"
            ))
            .query(&[("did", device_id)])
            .header("user-agent", DOUYU_USER_AGENT)
            .header("referer", &referer)
            .timeout(Duration::from_secs(15));
        if let Some(cookie) = &playback_cookie {
            key_request = key_request.header("cookie", cookie);
        }
        let response = key_request
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|err| web_protocol_error("获取网页播放密钥失败", err))?;
        let timestamp = response
            .headers()
            .get(reqwest::header::DATE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| chrono::DateTime::parse_from_rfc2822(value).ok())
            .and_then(|value| u64::try_from(value.timestamp()).ok())
            .map(Ok)
            .unwrap_or_else(unix_now)
            .map_err(WebPlayError::Rejected)?;
        let encrypted: WebEncryptionResponse = response
            .json()
            .await
            .map_err(|err| web_protocol_error("解析网页播放密钥失败", err))?;
        if encrypted.error != 0 {
            return Err(WebPlayError::Rejected(LiveError::custom(format!(
                "斗鱼网页播放密钥错误: code={}",
                encrypted.error
            ))));
        }
        let key = encrypted.data.ok_or_else(|| {
            WebPlayError::Protocol(LiveError::custom("斗鱼网页播放密钥缺少 data"))
        })?;
        let auth =
            web_signature_auth(&key, room_number, timestamp).map_err(WebPlayError::Protocol)?;
        let params = [
            ("enc_data", key.enc_data.as_str()),
            ("tt", &timestamp.to_string()),
            ("did", device_id),
            ("auth", &auth),
            ("cdn", self.douyu_cdn.as_str()),
            ("ver", "Douyu_new"),
            ("rate", &self.douyu_rate.to_string()),
            (
                "hevc",
                if self.douyu_codec.eq_ignore_ascii_case("HEVC") {
                    "1"
                } else {
                    "0"
                },
            ),
            ("iar", "0"),
            ("ive", "0"),
            ("sov", "0"),
            ("fa", "0"),
        ];
        let mut play_request = self
            .client
            .post(format!("{web_origin}/lapi/live/getH5PlayV1/{room_id}"))
            .form(&params)
            .header("user-agent", DOUYU_USER_AGENT)
            .header("referer", referer)
            .timeout(Duration::from_secs(15));
        if let Some(cookie) = &playback_cookie {
            play_request = play_request.header("cookie", cookie);
        }
        play_request
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|err| web_protocol_error("请求网页播放信息失败", err))?
            .json()
            .await
            .map_err(|err| web_protocol_error("解析网页播放信息失败", err))
    }

    fn log_stream_quality(&self, room_id: &str, info: &PlayInfo) {
        let actual = info
            .rate
            .and_then(|rate| info.multirates.iter().find(|quality| quality.rate == rate));
        debug!(
            room_id,
            requested_rate = self.douyu_rate,
            actual_rate = ?info.rate,
            quality = actual.map(|quality| quality.name.as_str()),
            bitrate_kbps = actual.map(|quality| quality.bit),
            has_hevc = info.player_1.as_ref().is_some_and(|url| !url.trim().is_empty()),
            "斗鱼播放接口返回画质"
        );
        if let Some(actual_rate) = info.rate {
            if actual_rate != self.douyu_rate {
                warn!(
                    room_id,
                    requested_rate = self.douyu_rate,
                    actual_rate,
                    quality = actual.map(|quality| quality.name.as_str()),
                    "斗鱼返回的画质与请求不同；原画可能需要有效的网页版登录 Cookie"
                );
            }
        }
    }

    async fn get_app_play_info_at(&self, room_id: &str, app_origin: &str) -> LiveResult<PlayInfo> {
        let room_number: u32 = room_id
            .parse()
            .map_err(|_| LiveError::custom("斗鱼房间号无效"))?;
        let device_id = self.device_id(false)?;
        let path = format!("/lapi/live/appGetPlayer/stream/{room_id}");
        let timestamp = unix_now()?;
        let device = random_android_device();
        let mut params = std::collections::BTreeMap::new();
        params.insert("txdw".to_string(), "0".to_string());
        params.insert(
            "cdn".to_string(),
            self.douyu_cdn.trim_end_matches("-h5").to_string(),
        );
        // A web acf_auth cookie is not an Android login token.
        params.insert("token".to_string(), String::new());
        params.insert("rate".to_string(), self.douyu_rate.to_string());
        params.insert(
            "hevc".to_string(),
            if self.douyu_codec.eq_ignore_ascii_case("HEVC") {
                "1"
            } else {
                "0"
            }
            .to_string(),
        );
        params.insert("ilow".to_string(), "0".to_string());
        params.insert("iar".to_string(), "0".to_string());
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
        let cookie_header =
            build_cookie_header(self.douyu_cookie.as_deref().unwrap_or_default(), device_id);
        let response = self
            .client
            .get(format!("{app_origin}{path}"))
            .query(&params)
            .header("User-Device", user_device)
            .header("aid", "android1")
            .header("channel", "447")
            .header(
                "User-Agent",
                format!("android/8.2.2.0 (android 16; ; {device})"),
            )
            .header("time", timestamp.to_string())
            .header("auth", auth)
            .header("Cookie", cookie_header)
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|err| {
                LiveError::custom(format!("请求斗鱼 App 播放信息失败: {}", err.without_url()))
            })?;
        let parsed: PlayResponse = response.json().await.map_err(|err| {
            LiveError::custom(format!("解析斗鱼 App 播放信息失败: {}", err.without_url()))
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
        let device_id = self.device_id(false)?;

        // 构建Cookie：如果用户提供了完整cookie，使用它；否则只使用device_id
        let cookie_header = if let Some(ref cookie) = self.douyu_cookie {
            build_cookie_header(cookie.trim(), device_id)
        } else {
            format!("acf_did={device_id}")
        };

        let tx_secret: XP2PTxSecret = self
            .client
            .get(format!("{api}/p2p/get_txsecret"))
            .query(&[("lid", stream_id)])
            .header("user-agent", DOUYU_USER_AGENT)
            .header("Cookie", cookie_header)
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

/// The App signature and User-Device header must use the same acf_did.
fn build_cookie_header(user_cookie: &str, device_id: &str) -> String {
    let mut parts = user_cookie
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .filter(|part| !part.starts_with("acf_did="))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    parts.push(format!("acf_did={device_id}"));
    parts.join("; ")
}

/// The official Web playback endpoints need the login cookies and their Web DID.
/// Keep the full imported source separately for renewal and other request types.
fn web_playback_cookie_header(cookie: &str) -> String {
    cookie
        .split(';')
        .map(str::trim)
        .filter(|part| {
            part.split_once('=')
                .is_some_and(|(name, _)| name.starts_with("acf_") || name == "dy_did")
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn room_id_from_mobile_page(page: &str) -> Option<String> {
    Regex::new(r#"roomInfo"\s*:\s*\{\s*"rid"\s*:\s*(\d+)"#)
        .unwrap()
        .captures(page)
        .map(|captures| captures[1].to_string())
}

// Current official first-stream bundle: web-encrypt-57bbddd0.js.
// Unlike the older ub98484234 JS signer, this key API uses repeated MD5.
fn web_signature_auth(key: &WebEncryptionKey, room_id: u32, timestamp: u64) -> LiveResult<String> {
    if key.enc_time > 10_000
        || key.key.is_empty()
        || key.rand_str.is_empty()
        || key.enc_data.is_empty()
    {
        return Err(LiveError::custom("斗鱼网页播放密钥格式错误"));
    }
    let digest = |source: String| format!("{:x}", Md5::digest(source.as_bytes()));
    let mut auth = key.rand_str.clone();
    for _ in 0..key.enc_time {
        auth = digest(format!("{auth}{}", key.key));
    }
    let suffix = if key.is_special == 1 {
        String::new()
    } else {
        format!("{room_id}{timestamp}")
    };
    Ok(digest(format!("{auth}{}{suffix}", key.key)))
}

#[derive(Debug)]
enum WebPlayError {
    Protocol(LiveError),
    Rejected(LiveError),
}

fn web_protocol_error(context: &str, error: reqwest::Error) -> WebPlayError {
    WebPlayError::Protocol(LiveError::custom(format!(
        "斗鱼{context}: {}",
        error.without_url()
    )))
}

#[derive(Deserialize)]
struct WebEncryptionResponse {
    error: i64,
    data: Option<WebEncryptionKey>,
}

#[derive(Deserialize)]
struct WebEncryptionKey {
    key: String,
    rand_str: String,
    enc_time: u32,
    is_special: u32,
    enc_data: String,
}

fn select_stream_url(play_info: PlayInfo, codec: &str) -> String {
    if play_info.is_mixed
        && !play_info.mixed_url.trim().is_empty()
        && !play_info.mixed_live.trim().is_empty()
    {
        return format!(
            "{}/{}",
            play_info.mixed_url.trim_end_matches('/'),
            play_info.mixed_live.trim_start_matches('/')
        );
    }
    if codec.eq_ignore_ascii_case("HEVC") {
        if let Some(url) = play_info.player_1.filter(|url| !url.trim().is_empty()) {
            debug!("使用 HEVC 流: player_1");
            return url;
        }
        warn!("斗鱼未提供 HEVC 流 (player_1 为空)，回退到 AVC 流");
    }
    let avc_url = format!(
        "{}/{}",
        play_info.rtmp_url.trim_end_matches('/'),
        play_info.rtmp_live.trim_start_matches('/')
    );
    debug!("使用 AVC 流: rtmp_url + rtmp_live");
    avc_url
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
    #[serde(rename = "videoLoop")]
    video_loop: i64,
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
    fn is_live(&self) -> bool {
        self.show_status == 1 && self.video_loop == 0
    }

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

#[derive(Deserialize, Debug, Default)]
struct PlayInfo {
    rtmp_url: String,
    rtmp_live: String,
    #[serde(default, deserialize_with = "deserialize_optional_player_url")]
    player_1: Option<String>,
    // Web calls this multirates, Android calls the same list rateSetting.
    #[serde(default, alias = "rateSetting")]
    multirates: Vec<StreamQuality>,
    #[serde(default)]
    rate: Option<u32>,
    #[serde(default)]
    is_mixed: bool,
    #[serde(default)]
    mixed_url: String,
    #[serde(default)]
    mixed_live: String,
}

#[derive(Deserialize, Debug)]
struct StreamQuality {
    rate: u32,
    #[serde(default)]
    name: String,
    #[serde(default)]
    bit: u32,
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
            ..PlayInfo::default()
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
            ..PlayInfo::default()
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
            douyu_cookie: None,
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

    #[test]
    fn betard_video_loop_is_not_live() {
        for (show_status, video_loop, expected) in [(1, 0, true), (1, 1, false), (0, 0, false)] {
            let body = format!(
                r#"{{"room":{{"room_name":"room","show_status":{show_status},"videoLoop":{video_loop}}}}}"#
            );
            let response: BetardResponse = serde_json::from_str(&body).unwrap();
            assert_eq!(response.room.unwrap().is_live(), expected);
        }
    }

    #[test]
    fn app_cookie_uses_the_signed_device_id() {
        let header = build_cookie_header("acf_auth=encoded%2F==; acf_did=old", "selected");
        assert_eq!(cookie::cookie_value(&header, "acf_did"), Some("selected"));
        assert_eq!(
            cookie::cookie_value(&header, "acf_auth"),
            Some("encoded%2F==")
        );
    }

    #[test]
    fn web_playback_cookie_omits_unrelated_fields_and_preserves_encoded_login() {
        let header = web_playback_cookie_header(
            " huya_ua={\"source\":\"browser\"}; acf_auth=encoded%2F==; PHPSESSID=other; acf_uid=42; dy_did=device; LTP0=passport-only; _ga=tracking; acf_did=app-device ",
        );
        assert_eq!(
            header,
            "acf_auth=encoded%2F==; acf_uid=42; dy_did=device; acf_did=app-device"
        );
        assert!(web_playback_cookie_header("PHPSESSID=other; _ga=tracking").is_empty());
    }

    #[test]
    fn hevc_codec_selection_respects_user_choice() {
        // 测试HEVC选择
        let hevc_info = PlayInfo {
            rtmp_url: "https://cdn.example/live/".into(),
            rtmp_live: "/avc.flv".into(),
            player_1: Some("https://cdn.example/hevc.flv".into()),
            ..PlayInfo::default()
        };
        assert_eq!(
            select_stream_url(hevc_info, "HEVC"),
            "https://cdn.example/hevc.flv"
        );

        // 测试AVC选择
        let avc_info = PlayInfo {
            rtmp_url: "https://cdn.example/live/".into(),
            rtmp_live: "/avc.flv".into(),
            player_1: Some("https://cdn.example/hevc.flv".into()),
            ..PlayInfo::default()
        };
        assert_eq!(
            select_stream_url(avc_info, "AVC"),
            "https://cdn.example/live/avc.flv"
        );

        // 测试空字符串也被当作AVC
        let default_info = PlayInfo {
            rtmp_url: "https://cdn.example/live/".into(),
            rtmp_live: "/avc.flv".into(),
            player_1: Some("https://cdn.example/hevc.flv".into()),
            ..PlayInfo::default()
        };
        assert_eq!(
            select_stream_url(default_info, ""),
            "https://cdn.example/live/avc.flv"
        );
    }

    fn encryption_fixture() -> &'static str {
        r#"{"error":0,"data":{"key":"public-test-key","rand_str":"seed","enc_time":2,"is_special":0,"enc_data":"fixture+data/=="}}"#
    }

    #[test]
    fn web_signature_matches_official_algorithm_fixture() {
        let mut key: WebEncryptionKey =
            serde_json::from_str::<WebEncryptionResponse>(encryption_fixture())
                .unwrap()
                .data
                .unwrap();
        // Independent known values for the official repeated-MD5 algorithm.
        assert_eq!(
            web_signature_auth(&key, 6979222, 1700000000).unwrap(),
            "c084ce69399fa5d87fbda97be8b7befa"
        );
        key.is_special = 1;
        assert_eq!(
            web_signature_auth(&key, 6979222, 1700000000).unwrap(),
            "73810131d86c959e9ba9c32e82577866"
        );
        key.enc_time = 10001;
        assert!(web_signature_auth(&key, 6979222, 1700000000).is_err());
    }

    #[test]
    fn mobile_alias_fixture_resolves_6657_and_keeps_loop_policy() {
        let page = r#"<script id="vike_pageContext" type="application/json">{"pageProps":{"room":{"roomInfo":{"encInfo":"","roomInfo":{"rid":6979222,"vipId":6657}}}}}</script>"#;
        assert_eq!(room_id_from_mobile_page(page).as_deref(), Some("6979222"));
        let room: BetardResponse = serde_json::from_str(
            r#"{"room":{"room_name":"fixture","show_status":1,"videoLoop":1}}"#,
        )
        .unwrap();
        assert!(!room.room.unwrap().is_live());
    }

    #[test]
    fn device_id_comes_from_cookie_unless_explicitly_configured() {
        let cache = RwLock::new(HashMap::new());
        let mut live = make_live("https://www.douyu.com/288016", &cache);
        live.douyu_cookie = Some("dy_did=webDevice; acf_did=appDevice".into());
        assert_eq!(live.device_id(true).unwrap(), "webDevice");
        assert_eq!(live.device_id(false).unwrap(), "appDevice");
        live.douyu_device_id = "overrideDevice".into();
        assert_eq!(live.device_id(true).unwrap(), "overrideDevice");
        assert_eq!(live.device_id(false).unwrap(), "overrideDevice");
    }

    #[test]
    fn play_info_uses_real_quality_fields_and_mixed_stream() {
        let info: PlayInfo = serde_json::from_str(r#"{"rtmp_url":"https://cdn.example/live","rtmp_live":"regular.flv","rate":4,"rateSetting":[{"rate":0,"name":"原画","bit":8000},{"rate":4,"name":"蓝光4M","bit":4000}]}"#).unwrap();
        assert_eq!(info.rate, Some(4));
        assert_eq!(info.multirates.len(), 2);
        assert_eq!(info.multirates[1].bit, 4000);
        let mixed = PlayInfo {
            rtmp_url: "https://cdn.example/live".into(),
            rtmp_live: "regular.flv".into(),
            player_1: Some("https://cdn.example/hevc.flv".into()),
            is_mixed: true,
            mixed_url: "https://mixed.example/live/".into(),
            mixed_live: "/mixed.flv".into(),
            ..PlayInfo::default()
        };
        assert_eq!(
            select_stream_url(mixed, "HEVC"),
            "https://mixed.example/live/mixed.flv"
        );
    }

    #[derive(Debug)]
    struct RecordedRequest {
        path: String,
        method: String,
        cookie: Option<String>,
        query: HashMap<String, String>,
        body: HashMap<String, String>,
    }

    struct MockApi {
        web_status: axum::http::StatusCode,
        web_body: String,
        requests: std::sync::Mutex<Vec<RecordedRequest>>,
    }

    async fn mock_api_request(
        axum::extract::State(mock): axum::extract::State<std::sync::Arc<MockApi>>,
        request: axum::extract::Request,
    ) -> axum::response::Response {
        use axum::response::IntoResponse;
        let path = request.uri().path().to_string();
        let method = request.method().to_string();
        let cookie = request
            .headers()
            .get("cookie")
            .map(|value| value.to_str().unwrap().to_string());
        let unrelated_cookie = cookie.as_deref().is_some_and(|header| {
            cookie::cookie_value(header, "huya_ua").is_some()
                || cookie::cookie_value(header, "PHPSESSID").is_some()
        });
        let query = form_urlencoded::parse(request.uri().query().unwrap_or_default().as_bytes())
            .into_owned()
            .collect();
        let body = axum::body::to_bytes(request.into_body(), 32 * 1024)
            .await
            .unwrap();
        let body = form_urlencoded::parse(&body).into_owned().collect();
        mock.requests.lock().unwrap().push(RecordedRequest {
            path: path.clone(),
            method,
            cookie,
            query,
            body,
        });
        let (status, body) = if path.ends_with("/getEncryption") {
            (axum::http::StatusCode::OK, encryption_fixture().to_string())
        } else if path.contains("getH5PlayV1") {
            if unrelated_cookie {
                // Reproduce the observed Web rejection of an otherwise valid
                // full browser export, before the App fallback loses login quality.
                (axum::http::StatusCode::FORBIDDEN, r#""forbidden""#.into())
            } else {
                (mock.web_status, mock.web_body.clone())
            }
        } else if path.contains("appGetPlayer") {
            (axum::http::StatusCode::OK, r#"{"error":0,"data":{"rtmp_url":"https://cdn.example/live","rtmp_live":"fallback.flv","rate":3,"rateSetting":[{"name":"超清","rate":3,"bit":2000}]}}"#.into())
        } else {
            (axum::http::StatusCode::NOT_FOUND, String::new())
        };
        (
            status,
            [
                ("content-type", "application/json"),
                ("date", "Tue, 14 Nov 2023 22:13:20 GMT"),
            ],
            body,
        )
            .into_response()
    }

    async fn start_mock_api(
        status: axum::http::StatusCode,
        body: &str,
    ) -> (String, std::sync::Arc<MockApi>, tokio::task::JoinHandle<()>) {
        let mock = std::sync::Arc::new(MockApi {
            web_status: status,
            web_body: body.to_string(),
            requests: std::sync::Mutex::new(Vec::new()),
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let app = axum::Router::new()
            .fallback(mock_api_request)
            .with_state(mock.clone());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (origin, mock, task)
    }

    #[tokio::test]
    async fn web_api_contract_preserves_cookie_codec_rate_and_server_downgrade() {
        let body = r#"{"error":0,"data":{"rtmp_url":"https://cdn.example/live","rtmp_live":"source_4000.flv","player_1":"https://cdn.example/source_4000h.flv","rate":4,"multirates":[{"name":"蓝光4M","rate":4,"bit":4000}]}}"#;
        for logged_in in [false, true] {
            let (origin, mock, task) = start_mock_api(axum::http::StatusCode::OK, body).await;
            let cache = RwLock::new(HashMap::new());
            let mut live = make_live("https://www.douyu.com/6979222", &cache);
            live.douyu_codec = "hevc".into();
            live.douyu_cdn = "hw-h5".into();
            // Dynamic room qualities must not be rejected by a fixed whitelist.
            live.douyu_rate = 8;
            if logged_in {
                live.douyu_cookie = Some(cookie::normalize_cookie_header(r#"[{"name":"acf_auth","value":"fixture%2F==","domain":"www.douyu.com"},{"name":"dy_did","value":"fixtureDevice","domain":".douyu.com"},{"name":"huya_ua","value":"{\"source\":\"browser\"}","domain":".douyu.com"},{"name":"PHPSESSID","value":"other-session","domain":".douyu.com"}]"#).unwrap());
            }
            let info = live
                .get_play_info_at("6979222", &origin, &origin)
                .await
                .unwrap();
            assert_eq!(info.rate, Some(4));
            assert_eq!(info.multirates[0].bit, 4000);
            assert_eq!(
                select_stream_url(info, "hevc"),
                "https://cdn.example/source_4000h.flv"
            );
            let requests = mock.requests.lock().unwrap();
            assert_eq!(
                requests.len(),
                2,
                "successful downgraded Web response must not trigger App fallback"
            );
            assert_eq!(requests[0].method, "GET");
            assert_eq!(requests[1].method, "POST");
            let expected_did = if logged_in {
                "fixtureDevice"
            } else {
                DOUYU_WEB_DEVICE_ID
            };
            assert_eq!(requests[0].query["did"], expected_did);
            assert_eq!(requests[1].body["did"], expected_did);
            assert_eq!(requests[1].body["auth"], "c084ce69399fa5d87fbda97be8b7befa");
            assert_eq!(requests[1].body["tt"], "1700000000");
            assert_eq!(requests[1].body["enc_data"], "fixture+data/==");
            assert_eq!(requests[1].body["rate"], "8");
            assert_eq!(requests[1].body["hevc"], "1");
            assert_eq!(requests[1].body["cdn"], "hw-h5");
            assert_eq!(requests[1].body["ver"], "Douyu_new");
            assert_eq!(requests[1].body["iar"], "0");
            for request in requests.iter() {
                assert_eq!(request.cookie.is_some(), logged_in);
                if logged_in {
                    assert_eq!(
                        cookie::cookie_value(request.cookie.as_deref().unwrap(), "acf_auth"),
                        Some("fixture%2F==")
                    );
                    assert_eq!(
                        request.cookie.as_deref(),
                        Some("acf_auth=fixture%2F==; dy_did=fixtureDevice")
                    );
                }
            }
            task.abort();
        }
    }

    #[tokio::test]
    async fn web_business_errors_do_not_fall_back_to_app() {
        for (code, message) in [
            (-5, "closeRoom"),
            (-9, "serverTime"),
            (126, "restricted"),
            (113, "banned"),
        ] {
            let body = format!(r#"{{"error":{code},"msg":"{message}","data":""}}"#);
            let (origin, mock, task) = start_mock_api(axum::http::StatusCode::OK, &body).await;
            let cache = RwLock::new(HashMap::new());
            let live = make_live("https://www.douyu.com/6979222", &cache);
            assert!(
                live.get_play_info_at("6979222", &origin, &origin)
                    .await
                    .is_err()
            );
            assert_eq!(mock.requests.lock().unwrap().len(), 2);
            task.abort();
        }
    }

    #[tokio::test]
    async fn web_http_and_protocol_failures_fall_back_to_app() {
        for (status, body) in [
            (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                "temporarily unavailable",
            ),
            (axum::http::StatusCode::OK, "not JSON"),
            (
                axum::http::StatusCode::OK,
                r#"{"error":0,"msg":"ok","data":null}"#,
            ),
        ] {
            let (origin, mock, task) = start_mock_api(status, body).await;
            let cache = RwLock::new(HashMap::new());
            let live = make_live("https://www.douyu.com/6979222", &cache);
            let info = live
                .get_play_info_at("6979222", &origin, &origin)
                .await
                .unwrap();
            assert_eq!(info.rtmp_live, "fallback.flv");
            assert_eq!(info.rate, Some(3));
            let requests = mock.requests.lock().unwrap();
            assert_eq!(requests.len(), 3);
            assert!(requests[2].path.contains("appGetPlayer"));
            assert_eq!(requests[2].query["hevc"], "0");
            assert_eq!(requests[2].query["rate"], "0");
            assert_eq!(requests[2].query["cdn"], "hs");
            task.abort();
        }
    }
}
