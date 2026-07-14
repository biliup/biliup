use super::{
    DanmakuSource, DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus,
    LiveStream, media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use md5::{Digest, Md5};
use rand::Rng;
use rand::seq::SliceRandom;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, warn};
use url::{Url, form_urlencoded};

const DOUYU_DEFAULT_DID: &str = "10000000000000000000000000001501";
const DOUYU_WEB_DOMAIN: &str = "www.douyu.com";
const DOUYU_MOBILE_DOMAIN: &str = "m.douyu.com";
const DOUYU_HUOS_DOMAIN: &str = "openflv-huos.douyucdn2.cn";
const DOUYU_HS_CDN: &str = "hs-h5";
const DOUYU_P2P_DOMAIN_TCT: &str = "hdltctwk.douyucdn.cn";
const DOUYU_P2PSDK_APIS: [&str; 2] = ["https://sdkapiv4.douyucdn.cn", "https://sdkapi.douyucdn.cn"];
const DOUYU_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct Douyu {
    re: Regex,
    /// 进程级加密密钥 + 配套 UA 缓存（对应 Python DouyuUtils.WhiteEncryptKey / UserAgent 类级缓存）。
    /// Mutex 同时充当 single-flight 锁（对应 Python DouyuUtils._lock / _update_key_event）：
    /// 并发刷新在此串行，后到者直接命中前者刷新的缓存。
    encrypt_key: Mutex<Option<CachedEncryptKey>>,
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
            encrypt_key: Mutex::new(None),
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
        DouyuLive::new(request, &self.encrypt_key, &self.real_room_id)
            .check_stream()
            .await
    }
}

/// 缓存的加密密钥与刷新时使用的随机 UA。enc_data 会校验 UA，
/// 因此 getEncryption 与 getH5PlayV1 必须使用同一 UA，密钥与 UA 需成对缓存。
#[derive(Clone)]
struct CachedEncryptKey {
    key: WhiteEncryptKey,
    user_agent: String,
}

impl CachedEncryptKey {
    /// 对应 Python DouyuUtils.is_key_valid：expire_at 取自 getEncryption 响应，
    /// 缺失时默认 0（永远过期，每次都会重新请求）。
    fn is_valid(&self, now: u64) -> bool {
        self.key.expire_at > now
    }
}

struct DouyuLive<'a> {
    client: Client,
    url: String,
    name: String,
    douyu_cdn: String,
    douyu_force_hs: bool,
    douyu_rate: u32,
    douyu_disable_interactive_game: bool,
    douyu_danmaku: bool,
    room_id: Option<String>,
    encrypt_key_cache: &'a Mutex<Option<CachedEncryptKey>>,
    real_room_id_cache: &'a RwLock<HashMap<String, String>>,
}

impl<'a> DouyuLive<'a> {
    fn new(
        request: LiveRequest,
        encrypt_key_cache: &'a Mutex<Option<CachedEncryptKey>>,
        real_room_id_cache: &'a RwLock<HashMap<String, String>>,
    ) -> Self {
        let options = request.options.douyu;
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
            douyu_cdn: options.cdn,
            douyu_force_hs: options.force_hs,
            douyu_rate: options.rate,
            douyu_disable_interactive_game: options.disable_interactive_game,
            douyu_danmaku: options.danmaku,
            room_id: None,
            encrypt_key_cache,
            real_room_id_cache,
        }
    }

    async fn check_stream(&mut self) -> LiveResult<LiveStatus> {
        let room_id = self.resolve_room_id().await?;
        self.room_id = Some(room_id.clone());
        let Some(room_info) = self.get_room_info(&room_id).await? else {
            return Ok(LiveStatus::Offline);
        };
        let play_info = self.get_web_play_info(&room_id).await?;
        let raw_stream_url = format!("{}/{}", play_info.rtmp_url, play_info.rtmp_live);
        let raw_stream_url = self.maybe_build_huos_url(raw_stream_url).await;

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: room_info.room_name,
                date: Utc::now(),
                live_cover_url: String::new(),
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
        if room.show_status != 1 || room.video_loop != 0 {
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

    async fn get_web_play_info(&self, room_id: &str) -> LiveResult<PlayInfo> {
        let mut cdn = self.douyu_cdn.clone();
        let mut last_error = None;
        // 鉴权失败时最多刷新一次加密密钥后重试（对齐 rust-srec get_play_info_fallback）：
        // 缓存密钥可能在本地 expire_at 之前就被服务端判为失效，此时需强制刷新。
        let mut key_refreshed = false;

        // scdn 规避最多重试 2 次；叠加一次鉴权刷新，循环上界取 3
        for _ in 0..3 {
            match self.request_web_play_info(room_id, &cdn).await {
                Ok(PlayOutcome::Ok(play_info)) => {
                    if play_info.rtmp_cdn.starts_with("scdn")
                        && let Some(next_cdn) = play_info
                            .cdns_with_name
                            .iter()
                            .rev()
                            .find_map(|cdn| cdn.cdn.clone())
                    {
                        cdn = next_cdn;
                        continue;
                    }
                    return Ok(play_info);
                }
                Ok(PlayOutcome::AuthFailed) => {
                    if !key_refreshed {
                        self.invalidate_key().await;
                        key_refreshed = true;
                        continue;
                    }
                    last_error = Some(LiveError::custom("斗鱼播放信息鉴权失败"));
                    break;
                }
                Err(err) => last_error = Some(err),
            }
        }

        Err(last_error.unwrap_or_else(|| LiveError::custom("获取斗鱼播放信息失败")))
    }

    /// 清空缓存的加密密钥，强制下次 update_key 重新向 getEncryption 请求。
    async fn invalidate_key(&self) {
        *self.encrypt_key_cache.lock().await = None;
    }

    async fn request_web_play_info(&self, room_id: &str, cdn: &str) -> LiveResult<PlayOutcome> {
        let now = unix_now()?;
        let (encrypt_key, user_agent) = self.update_key().await?;
        let auth = sign_stream(&encrypt_key, room_id, now);

        let form = vec![
            ("cdn", cdn.to_string()),
            ("rate", self.douyu_rate.to_string()),
            ("ver", "Douyu_new".to_string()),
            ("iar", "0".to_string()),
            ("ive", "0".to_string()),
            ("rid", room_id.to_string()),
            ("hevc", "0".to_string()),
            ("fa", "0".to_string()),
            ("sov", "0".to_string()),
            ("enc_data", encrypt_key.enc_data),
            ("tt", now.to_string()),
            ("did", DOUYU_DEFAULT_DID.to_string()),
            ("auth", auth),
        ];

        let rsp = self
            .client
            .post(format!(
                "https://{DOUYU_WEB_DOMAIN}/lapi/live/getH5PlayV1/{room_id}"
            ))
            .header("referer", format!("https://{DOUYU_WEB_DOMAIN}"))
            .header("user-agent", &user_agent)
            .form(&form)
            .send()
            .await
            .map_err(|err| {
                LiveError::custom(format!("请求斗鱼播放信息失败 room_id: {room_id}: {err}"))
            })?;

        let status = rsp.status();
        let body = rsp.text().await.map_err(|err| {
            LiveError::custom(format!("读取斗鱼播放信息失败 room_id: {room_id}: {err}"))
        })?;

        // 鉴权失败：HTTP 403 或响应体含「鉴权失败」，由调用方刷新密钥后重试
        // （对齐 rust-srec is_douyu_auth_failed）
        if is_douyu_auth_failed(status.as_u16(), &body) {
            return Ok(PlayOutcome::AuthFailed);
        }

        let rsp: PlayResponse = serde_json::from_str(&body).map_err(|err| {
            LiveError::custom(format!("解析斗鱼播放信息失败 room_id: {room_id}: {err}"))
        })?;

        if rsp.error == 0
            && let Some(data) = rsp.data
        {
            return Ok(PlayOutcome::Ok(data));
        }

        // JSON 形式返回的鉴权失败同样触发刷新重试
        if rsp
            .msg
            .as_deref()
            .is_some_and(|msg| msg.contains("鉴权失败"))
        {
            return Ok(PlayOutcome::AuthFailed);
        }

        if rsp.error == -5 {
            return Err(LiveError::custom("[closeRoom] 主播未开播"));
        }
        if rsp.error == -9 {
            return Err(LiveError::custom(
                "[room_bus_checksevertime] 用户本机时间戳不对",
            ));
        }
        if rsp.error == 126 {
            return Err(LiveError::custom(format!(
                "版权原因，该地域不允许播放：{}",
                rsp.msg.unwrap_or_default()
            )));
        }
        Err(LiveError::custom(format!(
            "获取斗鱼播放信息错误: code={}, msg={}",
            rsp.error,
            rsp.msg.unwrap_or_default()
        )))
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

        Err(last_error.unwrap_or_else(|| {
            LiveError::custom(format!("获取 txSecret 失败: {stream_id}"))
        }))
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

    /// 获取加密密钥及其配套 UA。对应 Python DouyuUtils.sign 中
    /// `is_key_valid() or update_key()` 的缓存逻辑：密钥未过期直接复用，
    /// 过期才重新请求 getEncryption。整个过程持有 Mutex，天然 single-flight。
    async fn update_key(&self) -> LiveResult<(WhiteEncryptKey, String)> {
        let mut cache = self.encrypt_key_cache.lock().await;

        let now = unix_now()?;
        if let Some(cached) = cache.as_ref()
            && cached.is_valid(now)
        {
            return Ok((cached.key.clone(), cached.user_agent.clone()));
        }

        // 防风控，每次刷新密钥随机 UA（对应 douyu.py DouyuUtils.update_key）
        let user_agent = random_chrome_user_agent();
        let rsp: EncryptionResponse = self
            .client
            .get(format!(
                "https://{DOUYU_WEB_DOMAIN}/wgapi/livenc/liveweb/websec/getEncryption"
            ))
            .query(&[("did", DOUYU_DEFAULT_DID)])
            .header("user-agent", &user_agent)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取斗鱼加密密钥失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析斗鱼加密密钥失败: {err}")))?;

        if rsp.error != 0 {
            return Err(LiveError::custom(format!(
                "getEncryption error: code={}, msg={}",
                rsp.error,
                rsp.msg.unwrap_or_default()
            )));
        }

        let key = rsp
            .data
            .ok_or_else(|| LiveError::custom("斗鱼加密密钥为空"))?;
        *cache = Some(CachedEncryptKey {
            key: key.clone(),
            user_agent: user_agent.clone(),
        });
        Ok((key, user_agent))
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

/// getH5PlayV1 请求结果：正常拿到播放信息，或需刷新密钥重试的鉴权失败。
enum PlayOutcome {
    Ok(PlayInfo),
    AuthFailed,
}

/// 判定 getH5PlayV1 是否鉴权失败：HTTP 403，或响应体（去空白、去引号后）含「鉴权失败」
/// （对齐 rust-srec is_douyu_auth_failed / normalize_douyu_error_body）。
fn is_douyu_auth_failed(status: u16, body: &str) -> bool {
    if status == 403 {
        return true;
    }
    let normalized: String = body
        .trim()
        .trim_matches('"')
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect();
    normalized.contains("鉴权失败")
}

/// 生成随机 Chrome UA（对应 biliup/plugins/__init__.py 的 random_user_agent()，
/// 桌面端格式，Chrome 大版本随机 100~120）
fn random_chrome_user_agent() -> String {
    let chrome_version: u32 = rand::thread_rng().gen_range(100..=120);
    format!(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{chrome_version}.0.0.0 Safari/537.36"
    )
}

fn sign_stream(encrypt_key: &WhiteEncryptKey, room_id: &str, ts: u64) -> String {
    let salt = if encrypt_key.is_special == 1 {
        String::new()
    } else {
        format!("{room_id}{ts}")
    };

    let mut secret = encrypt_key.rand_str.clone();
    for _ in 0..encrypt_key.enc_time {
        secret = md5_hex(format!("{}{}", secret, encrypt_key.key));
    }
    md5_hex(format!("{}{}{}", secret, encrypt_key.key, salt))
}

fn md5_hex(input: String) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
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
}

#[derive(Deserialize)]
struct EncryptionResponse {
    error: i64,
    msg: Option<String>,
    data: Option<WhiteEncryptKey>,
}

#[derive(Clone, Deserialize)]
struct WhiteEncryptKey {
    rand_str: String,
    enc_time: u32,
    is_special: i64,
    key: String,
    enc_data: String,
    /// getEncryption 响应携带的过期时间（10 位 Unix 时间戳）。
    /// 对应 Python `WhiteEncryptKey.get('expire_at', 0)`，缺失时默认 0
    #[serde(default)]
    expire_at: u64,
}

#[derive(Deserialize)]
struct PlayResponse {
    error: i64,
    msg: Option<String>,
    data: Option<PlayInfo>,
}

#[derive(Deserialize)]
struct PlayInfo {
    rtmp_url: String,
    rtmp_live: String,
    #[serde(default)]
    rtmp_cdn: String,
    #[serde(default, rename = "cdnsWithName")]
    cdns_with_name: Vec<CdnInfo>,
}

#[derive(Deserialize)]
struct CdnInfo {
    cdn: Option<String>,
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

    fn make_live<'a>(
        url: &str,
        encrypt_key_cache: &'a Mutex<Option<CachedEncryptKey>>,
        real_room_id_cache: &'a RwLock<HashMap<String, String>>,
    ) -> DouyuLive<'a> {
        DouyuLive {
            client: Client::new(),
            url: url.to_string(),
            name: "test".to_string(),
            douyu_cdn: DOUYU_HS_CDN.to_string(),
            douyu_force_hs: true,
            douyu_rate: 0,
            douyu_disable_interactive_game: false,
            douyu_danmaku: false,
            room_id: None,
            encrypt_key_cache,
            real_room_id_cache,
        }
    }

    fn make_key(expire_at: u64) -> WhiteEncryptKey {
        WhiteEncryptKey {
            rand_str: "rand".to_string(),
            enc_time: 1,
            is_special: 0,
            key: "key".to_string(),
            enc_data: "enc".to_string(),
            expire_at,
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

    #[tokio::test]
    async fn maybe_build_huos_url_falls_back_when_build_fails() {
        let encrypt_key_cache = Mutex::new(None);
        let real_room_id_cache = RwLock::new(HashMap::new());
        let live = make_live(
            "https://www.douyu.com/10568722",
            &encrypt_key_cache,
            &real_room_id_cache,
        );

        let raw_stream_url = "not a url".to_string();

        assert_eq!(
            live.maybe_build_huos_url(raw_stream_url.clone()).await,
            raw_stream_url
        );
    }

    #[test]
    fn random_chrome_user_agent_matches_python_format() {
        // 对应 Python random_user_agent()：Chrome 大版本 100~120，桌面端固定格式
        let re = Regex::new(
            r"^Mozilla/5\.0 \(Windows NT 10\.0; Win64; x64\) AppleWebKit/537\.36 \(KHTML, like Gecko\) Chrome/(\d+)\.0\.0\.0 Safari/537\.36$",
        )
        .unwrap();

        for _ in 0..50 {
            let ua = random_chrome_user_agent();
            let caps = re.captures(&ua).unwrap_or_else(|| panic!("UA 格式错误: {ua}"));
            let version: u32 = caps[1].parse().unwrap();
            assert!(
                (100..=120).contains(&version),
                "Chrome 版本超出范围: {version}"
            );
        }
    }

    #[test]
    fn cached_encrypt_key_expiry_follows_expire_at() {
        let cached = CachedEncryptKey {
            key: make_key(1000),
            user_agent: random_chrome_user_agent(),
        };

        assert!(cached.is_valid(999));
        // 对应 Python is_key_valid 的 `expire_at > int(time.time())`：等于当前时间视为过期
        assert!(!cached.is_valid(1000));
        assert!(!cached.is_valid(1001));

        let no_expiry = CachedEncryptKey {
            key: make_key(0),
            user_agent: random_chrome_user_agent(),
        };
        assert!(!no_expiry.is_valid(0));
    }

    #[test]
    fn white_encrypt_key_missing_expire_at_defaults_to_zero() {        let key: WhiteEncryptKey = serde_json::from_str(
            r#"{"rand_str":"r","enc_time":2,"is_special":0,"key":"k","enc_data":"e"}"#,
        )
        .unwrap();
        assert_eq!(key.expire_at, 0);

        let key: WhiteEncryptKey = serde_json::from_str(
            r#"{"rand_str":"r","enc_time":2,"is_special":0,"key":"k","enc_data":"e","expire_at":1234567890}"#,
        )
        .unwrap();
        assert_eq!(key.expire_at, 1234567890);
    }

    #[tokio::test]
    async fn update_key_reuses_cached_key_and_user_agent_pair() {
        let user_agent = random_chrome_user_agent();
        let encrypt_key_cache = Mutex::new(Some(CachedEncryptKey {
            key: make_key(unix_now().unwrap() + 3600),
            user_agent: user_agent.clone(),
        }));
        let real_room_id_cache = RwLock::new(HashMap::new());
        let live = make_live(
            "https://www.douyu.com/10568722",
            &encrypt_key_cache,
            &real_room_id_cache,
        );

        // 缓存未过期时直接复用，不发起网络请求，且密钥与 UA 成对返回
        let (key, ua) = live.update_key().await.unwrap();
        assert_eq!(key.rand_str, "rand");
        assert_eq!(key.enc_data, "enc");
        assert_eq!(ua, user_agent);
    }

    #[tokio::test]
    async fn resolve_room_id_uses_cached_real_room_id() {
        let url = "https://www.douyu.com/somename";
        let encrypt_key_cache = Mutex::new(None);
        let real_room_id_cache = RwLock::new(HashMap::from([(
            url.to_string(),
            "10568722".to_string(),
        )]));
        let live = make_live(url, &encrypt_key_cache, &real_room_id_cache);

        // 命中缓存时不请求移动端页面
        assert_eq!(live.resolve_room_id().await.unwrap(), "10568722");
    }

    #[tokio::test]
    async fn resolve_room_id_prefers_rid_query_param() {
        let encrypt_key_cache = Mutex::new(None);
        let real_room_id_cache = RwLock::new(HashMap::new());
        let live = make_live(
            "https://www.douyu.com/topic/xyz?rid=123456",
            &encrypt_key_cache,
            &real_room_id_cache,
        );

        assert_eq!(live.resolve_room_id().await.unwrap(), "123456");
    }

    #[test]
    fn is_douyu_auth_failed_detects_403_and_message() {
        // HTTP 403 一律视为鉴权失败
        assert!(is_douyu_auth_failed(403, ""));
        // 响应体含「鉴权失败」（含空白/引号包裹）也算
        assert!(is_douyu_auth_failed(200, r#"  "鉴 权 失 败"  "#));
        assert!(is_douyu_auth_failed(200, "鉴权失败"));
        // 正常响应不算
        assert!(!is_douyu_auth_failed(200, r#"{"error":0,"data":{}}"#));
        assert!(!is_douyu_auth_failed(200, ""));
    }
}
