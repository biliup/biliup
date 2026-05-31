use super::wbi::WbiSigner;
use super::{
    DanmakuSource, DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus,
    LiveStream, media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use reqwest::Client;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use url::Url;

const BILIBILI_API_BASE: &str = "https://api.live.bilibili.com";
const BILIBILI_REFERER: &str = "https://live.bilibili.com";
const BILIBILI_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const WBI_WEB_LOCATION: &str = "444.8";

pub struct Bilibili {
    re: Regex,
}

impl Default for Bilibili {
    fn default() -> Self {
        Self::new()
    }
}

impl Bilibili {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"(?:https?://)?(?:b23\.tv|(?:(?:www|m|live)\.)?bilibili\.com)")
                .unwrap(),
        }
    }
}

#[async_trait]
impl LivePlugin for Bilibili {
    fn name(&self) -> &'static str {
        "Bilibili"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        BilibiliLive::new(request).check_stream().await
    }
}

struct BilibiliLive {
    client: Client,
    url: String,
    name: String,
    qn: u32,
    protocol: String,
    cdn: Vec<String>,
    cdn_fallback: bool,
    hls_transcode_timeout: u64,
    anonymous_origin: bool,
    api_list: Vec<String>,
    cookie: Option<String>,
    cookie_file: Option<PathBuf>,
    danmaku: bool,
    danmaku_raw: bool,
    danmaku_detail: bool,
    room_id: Option<u64>,
    wbi_signer: WbiSigner,
}

impl BilibiliLive {
    fn new(request: LiveRequest) -> Self {
        let options = request.options.bilibili;
        let credentials = request.credentials;
        Self {
            wbi_signer: WbiSigner::new(request.client.clone()),
            client: request.client,
            url: request.url,
            name: request.name,
            qn: options.qn,
            protocol: options.protocol,
            cdn: options.cdn,
            cdn_fallback: options.cdn_fallback,
            hls_transcode_timeout: options.hls_transcode_timeout,
            anonymous_origin: options.anonymous_origin,
            api_list: vec![
                normalize_api(options.live_api.as_deref()),
                normalize_api(options.fallback_api.as_deref()),
            ],
            cookie: credentials.bilibili_cookie,
            cookie_file: credentials.bilibili_cookie_file,
            danmaku: options.danmaku,
            danmaku_raw: options.danmaku_raw,
            danmaku_detail: options.danmaku_detail,
            room_id: None,
        }
    }

    async fn check_stream(&mut self) -> LiveResult<LiveStatus> {
        let url = self.resolve_url().await?;
        let room_id = self.room_id(&url)?;
        self.url = url;

        let headers = self.headers(&self.url)?;
        let Some(profile) = self.get_room_info(&room_id, headers.clone()).await? else {
            return Ok(LiveStatus::Offline);
        };
        self.room_id = Some(profile.room_id);
        let candidates = self.get_stream_candidates(&profile, headers).await?;
        let raw_stream_url = self.select_stream_url(&candidates).await?;
        let danmaku = self.danmaku_source();

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: profile.title,
                date: Utc::now(),
                live_cover_url: profile.cover,
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| {
                    if raw_stream_url.contains(".m3u8") {
                        "m3u8".to_string()
                    } else {
                        "flv".to_string()
                    }
                }),
                raw_stream_url,
                platform: "bilibili".to_string(),
                stream_headers: self.stream_headers(),
                danmaku,
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }

    async fn resolve_url(&self) -> LiveResult<String> {
        if !self.url.contains("b23.tv") {
            return Ok(self.url.clone());
        }

        let resp = self
            .client
            .get(&self.url)
            .header(USER_AGENT, BILIBILI_USER_AGENT)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("解析 B 站短链接失败: {err}")))?;
        let url = resp.url().to_string();
        if !url.contains("live.bilibili.com") {
            return Err(LiveError::custom("B 站短链接不是直播间地址"));
        }
        Ok(url)
    }

    fn room_id(&self, url: &str) -> LiveResult<String> {
        Regex::new(r"/(\d+)")
            .unwrap()
            .captures(url)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| LiveError::custom("B 站直播间地址错误"))
    }

    fn headers(&self, referer: &str) -> LiveResult<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(BILIBILI_USER_AGENT));
        if let Ok(referer) = HeaderValue::from_str(referer) {
            headers.insert(REFERER, referer);
        }
        if let Some(cookie) = self.cookie()
            && let Ok(cookie) = HeaderValue::from_str(&cookie)
        {
            headers.insert(COOKIE, cookie);
        }
        Ok(headers)
    }

    fn cookie(&self) -> Option<String> {
        if self.cookie.is_some() {
            return self.cookie.clone();
        }

        let path = self.cookie_file.as_ref()?;
        let text = std::fs::read_to_string(path).ok()?;
        let value: Value = serde_json::from_str(&text).ok()?;
        let cookies = value
            .get("cookie_info")
            .and_then(|info| info.get("cookies"))
            .and_then(|cookies| cookies.as_array())?;
        let cookie = cookies
            .iter()
            .filter_map(|cookie| {
                Some(format!(
                    "{}={}",
                    cookie.get("name")?.as_str()?,
                    cookie.get("value")?.as_str()?
                ))
            })
            .collect::<Vec<_>>()
            .join(";");
        (!cookie.is_empty()).then_some(cookie)
    }

    async fn get_room_info(
        &self,
        room_id: &str,
        headers: HeaderMap,
    ) -> LiveResult<Option<BiliRoomProfile>> {
        let mut params = BTreeMap::new();
        params.insert("room_id".to_string(), room_id.to_string());
        params.insert("web_location".to_string(), WBI_WEB_LOCATION.to_string());
        self.wbi_signer.sign(&mut params, &headers).await?;

        let room_info: Value = self
            .client
            .get(format!(
                "{BILIBILI_API_BASE}/xlive/web-room/v1/index/getInfoByRoom"
            ))
            .query(&params)
            .headers(headers)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取 B 站直播间信息失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析 B 站直播间信息失败: {err}")))?;

        if room_info.get("code").and_then(|code| code.as_i64()) != Some(0) {
            return Err(LiveError::custom(format!(
                "获取 B 站直播间信息错误: {}",
                room_info
                    .get("message")
                    .or_else(|| room_info.get("msg"))
                    .and_then(|msg| msg.as_str())
                    .unwrap_or_default()
            )));
        }

        let room = room_info
            .get("data")
            .and_then(|data| data.get("room_info"))
            .ok_or_else(|| LiveError::custom("B 站直播间信息为空"))?;
        if room.get("live_status").and_then(|status| status.as_i64()) != Some(1) {
            return Ok(None);
        }

        Ok(Some(BiliRoomProfile {
            room_id: room
                .get("room_id")
                .and_then(|room_id| room_id.as_u64())
                .ok_or_else(|| LiveError::custom("B 站真实房间号为空"))?,
            uid: room
                .get("uid")
                .and_then(|uid| uid.as_u64())
                .unwrap_or_default(),
            live_start_time: room
                .get("live_start_time")
                .and_then(|time| time.as_u64())
                .unwrap_or_default(),
            special_type: room
                .get("special_type")
                .and_then(|special_type| special_type.as_u64())
                .unwrap_or_default(),
            title: room
                .get("title")
                .and_then(|title| title.as_str())
                .unwrap_or_default()
                .to_string(),
            cover: room
                .get("cover")
                .and_then(|cover| cover.as_str())
                .unwrap_or_default()
                .to_string(),
        }))
    }

    async fn get_stream_candidates(
        &self,
        profile: &BiliRoomProfile,
        headers: HeaderMap,
    ) -> LiveResult<Vec<BiliStreamCandidate>> {
        let mut last_error = None;
        for api in &self.api_list {
            match self
                .request_play_info(api, profile.room_id, headers.clone())
                .await
            {
                Ok(play_info) => {
                    if let Some(candidates) = self
                        .parse_play_info(api, profile, &play_info, &headers)
                        .await?
                    {
                        return Ok(candidates);
                    }
                }
                Err(err) => last_error = Some(err),
            }
        }

        Err(last_error.unwrap_or_else(|| LiveError::custom("获取 B 站直播流失败")))
    }

    async fn request_play_info(
        &self,
        api: &str,
        room_id: u64,
        headers: HeaderMap,
    ) -> LiveResult<Value> {
        let mut params = BTreeMap::new();
        params.insert("room_id".to_string(), room_id.to_string());
        params.insert("qn".to_string(), self.qn.to_string());
        params.insert("platform".to_string(), "html5".to_string());
        params.insert("protocol".to_string(), "0,1".to_string());
        params.insert("format".to_string(), "0,1,2".to_string());
        params.insert("codec".to_string(), "0".to_string());
        params.insert("dolby".to_string(), "5".to_string());
        params.insert("web_location".to_string(), WBI_WEB_LOCATION.to_string());
        self.wbi_signer.sign(&mut params, &headers).await?;

        let play_info: Value = self
            .client
            .get(format!("{api}/xlive/web-room/v2/index/getRoomPlayInfo"))
            .query(&params)
            .headers(headers)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("请求 B 站播放信息失败: {api}: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析 B 站播放信息失败: {api}: {err}")))?;

        if play_info.get("code").and_then(|code| code.as_i64()) != Some(0) {
            return Err(LiveError::custom(format!(
                "B 站播放信息错误: {}",
                play_info
                    .get("message")
                    .or_else(|| play_info.get("msg"))
                    .and_then(|msg| msg.as_str())
                    .unwrap_or_default()
            )));
        }
        Ok(play_info)
    }

    async fn parse_play_info(
        &self,
        api: &str,
        profile: &BiliRoomProfile,
        play_info: &Value,
        headers: &HeaderMap,
    ) -> LiveResult<Option<Vec<BiliStreamCandidate>>> {
        let streams = match play_info
            .get("data")
            .and_then(|data| data.get("playurl_info"))
            .and_then(|info| info.get("playurl"))
            .and_then(|playurl| playurl.get("stream"))
            .and_then(Value::as_array)
        {
            Some(streams) => streams,
            None => return Ok(None),
        };

        if self.protocol == "hls_fmp4" {
            if let Some(candidates) = self
                .master_m3u8_candidates(api, profile, play_info, headers)
                .await?
            {
                return Ok(Some(candidates));
            }

            let candidates = streams
                .iter()
                .flat_map(|stream| stream.get("format").and_then(|formats| formats.as_array()))
                .flatten()
                .find(|format| {
                    format.get("format_name").and_then(|name| name.as_str()) == Some("fmp4")
                })
                .and_then(|format| format.get("codec"))
                .and_then(Value::as_array)
                .and_then(|codecs| codecs.first())
                .and_then(parse_codec_urls);
            if let Some(candidates) = &candidates
                && self.qn >= 10000
                && !candidates.iter().any(|candidate| candidate.qn == self.qn)
            {
                return Ok(Some(Vec::new()));
            }
            if candidates.is_some() {
                return Ok(candidates);
            }
            if now_unix().saturating_sub(profile.live_start_time) <= self.hls_transcode_timeout {
                return Ok(Some(Vec::new()));
            }
        }

        Ok(streams
            .first()
            .and_then(|stream| stream.get("format"))
            .and_then(Value::as_array)
            .and_then(|formats| formats.first())
            .and_then(|format| format.get("codec"))
            .and_then(Value::as_array)
            .and_then(|codecs| codecs.first())
            .and_then(parse_codec_urls))
    }

    async fn master_m3u8_candidates(
        &self,
        api: &str,
        profile: &BiliRoomProfile,
        play_info: &Value,
        headers: &HeaderMap,
    ) -> LiveResult<Option<Vec<BiliStreamCandidate>>> {
        if !self.anonymous_origin {
            return Ok(None);
        }
        let special_types = play_info
            .get("data")
            .and_then(|data| data.get("all_special_types"))
            .and_then(Value::as_array);
        if special_types
            .map(|items| {
                items
                    .iter()
                    .any(|item| item.as_u64() == Some(profile.special_type))
            })
            .unwrap_or(false)
            && self.cookie().is_none()
        {
            return Ok(None);
        }

        let mid = self.login_mid(headers).await?.unwrap_or(profile.uid);
        let text = self
            .client
            .get(format!("{api}/xlive/play-gateway/master/url"))
            .query(&[
                ("cid", profile.room_id.to_string()),
                ("mid", mid.to_string()),
                ("pt", "web".to_string()),
                ("p2p_type", "-1".to_string()),
                ("net", "0".to_string()),
                ("free_type", "0".to_string()),
                ("build", "0".to_string()),
                ("feature", "2".to_string()),
                ("qn", self.qn.to_string()),
                ("drm_type", "0".to_string()),
                ("codec", "0,1".to_string()),
            ])
            .headers(headers.clone())
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取 B 站 master m3u8 失败: {err}")))?
            .text()
            .await
            .map_err(|err| LiveError::custom(format!("读取 B 站 master m3u8 失败: {err}")))?;

        if !text.starts_with("#EXTM3U") {
            return Ok(None);
        }
        Ok(parse_master_m3u8(&text))
    }

    async fn login_mid(&self, headers: &HeaderMap) -> LiveResult<Option<u64>> {
        let value: Value = self
            .client
            .get("https://api.bilibili.com/x/web-interface/nav")
            .headers(headers.clone())
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取 B 站登录状态失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析 B 站登录状态失败: {err}")))?;
        Ok(value
            .get("data")
            .and_then(|data| data.get("isLogin"))
            .and_then(Value::as_bool)
            .filter(|is_login| *is_login)
            .and_then(|_| value.get("data"))
            .and_then(|data| data.get("mid"))
            .and_then(Value::as_u64))
    }

    async fn check_url_healthy(&self, url: &str) -> Option<String> {
        if url.contains(".m3u8") {
            let response = self
                .client
                .get(url)
                .headers(self.headers(&self.url).ok()?)
                .send()
                .await
                .ok()?;
            if !response.status().is_success() {
                return None;
            }
            let text = response.text().await.ok()?;
            if let Some(uri) = first_variant_uri(&text) {
                let nested = resolve_url(url, &uri)?;
                let response = self
                    .client
                    .get(&nested)
                    .headers(self.headers(&self.url).ok()?)
                    .send()
                    .await
                    .ok()?;
                return response.status().is_success().then_some(nested);
            }
            return Some(url.to_string());
        }

        let response = self
            .client
            .get(url)
            .headers(self.headers(&self.url).ok()?)
            .send()
            .await
            .ok()?;
        if response.status().is_success() {
            return Some(url.to_string());
        }
        if response.status().is_redirection()
            && let Some(location) = response
                .headers()
                .get("location")
                .and_then(|value| value.to_str().ok())
        {
            let resolved = resolve_url(url, location)?;
            let response = self
                .client
                .get(&resolved)
                .headers(self.headers(&self.url).ok()?)
                .send()
                .await
                .ok()?;
            return response.status().is_success().then_some(resolved);
        }
        None
    }

    async fn select_stream_url(&self, candidates: &[BiliStreamCandidate]) -> LiveResult<String> {
        let selected = if !self.cdn.is_empty()
            && let Some(candidate) = self
                .cdn
                .iter()
                .find_map(|cdn| candidates.iter().find(|candidate| &candidate.cdn == cdn))
        {
            candidate
        } else {
            candidates
                .first()
                .ok_or_else(|| LiveError::custom("B 站可用直播流为空"))?
        };

        if !self.cdn_fallback {
            return Ok(selected.url.clone());
        }
        if let Some(url) = self.check_url_healthy(&selected.url).await {
            return Ok(url);
        }
        for candidate in candidates {
            if let Some(url) = self.check_url_healthy(&candidate.url).await {
                return Ok(url);
            }
        }
        Err(LiveError::custom("B 站所有 CDN 均不可用"))
    }

    fn stream_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::from([
            ("referer".to_string(), BILIBILI_REFERER.to_string()),
            ("user-agent".to_string(), BILIBILI_USER_AGENT.to_string()),
        ]);
        if let Some(cookie) = self.cookie() {
            headers.insert("cookie".to_string(), cookie);
        }
        headers
    }

    fn danmaku_source(&self) -> Option<DanmakuSource> {
        if !self.danmaku {
            return None;
        }
        Some(DanmakuSource {
            platform: "bilibili".to_string(),
            url: self.url.clone(),
            room_id: Some(self.room_id?.to_string()),
            cookie: self.cookie(),
            raw: self.danmaku_raw,
            detail: self.danmaku_detail,
            extra: HashMap::new(),
            movie_id: None,
            password: None,
        })
    }
}

struct BiliRoomProfile {
    room_id: u64,
    uid: u64,
    live_start_time: u64,
    special_type: u64,
    title: String,
    cover: String,
}

struct BiliStreamCandidate {
    qn: u32,
    cdn: String,
    url: String,
}

fn parse_codec_urls(codec: &Value) -> Option<Vec<BiliStreamCandidate>> {
    let current_qn = codec.get("current_qn")?.as_u64()? as u32;
    let base_url = codec.get("base_url")?.as_str()?;
    let url_info = codec.get("url_info")?.as_array()?;
    let candidates = url_info
        .iter()
        .filter_map(|info| {
            let host = info.get("host")?.as_str()?;
            let extra = info.get("extra")?.as_str()?;
            Some(BiliStreamCandidate {
                qn: current_qn,
                cdn: extract_cdn(extra),
                url: format!("{host}{base_url}{extra}"),
            })
        })
        .collect::<Vec<_>>();
    (!candidates.is_empty()).then_some(candidates)
}

fn parse_master_m3u8(text: &str) -> Option<Vec<BiliStreamCandidate>> {
    let mut current_qn = None;
    let mut candidates = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with("#EXT-X-STREAM-INF:") {
            current_qn = parse_stream_inf_qn(line);
        } else if !line.is_empty() && !line.starts_with('#') {
            candidates.push(BiliStreamCandidate {
                qn: current_qn.unwrap_or_default(),
                cdn: String::new(),
                url: line.to_string(),
            });
            current_qn = None;
        }
    }
    (!candidates.is_empty()).then_some(candidates)
}

fn parse_stream_inf_qn(line: &str) -> Option<u32> {
    line.split(',').find_map(|part| {
        let (key, value) = part.split_once('=')?;
        matches!(key.trim(), "QN" | "QUALITY" | "BILI-QN")
            .then(|| value.trim().trim_matches('"').parse().ok())?
    })
}

fn first_variant_uri(text: &str) -> Option<String> {
    let mut variant = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with("#EXT-X-STREAM-INF:") {
            variant = true;
        } else if variant && !line.is_empty() && !line.starts_with('#') {
            return Some(line.to_string());
        }
    }
    None
}

fn resolve_url(base: &str, value: &str) -> Option<String> {
    Url::parse(base)
        .ok()?
        .join(value)
        .ok()
        .map(|url| url.to_string())
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn extract_cdn(extra: &str) -> String {
    Regex::new(r"(?:^|[?&])cdn=([^&]+)")
        .unwrap()
        .captures(extra)
        .map(|captures| captures[1].to_string())
        .unwrap_or_default()
}

fn normalize_api(value: Option<&str>) -> String {
    let value = value.unwrap_or(BILIBILI_API_BASE);
    let value = if value.starts_with("http://") || value.starts_with("https://") {
        value.to_string()
    } else {
        format!("http://{value}")
    };
    value.trim_end_matches('/').to_string()
}
