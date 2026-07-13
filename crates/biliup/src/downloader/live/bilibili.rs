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
use tracing::{error, warn};
use url::Url;

const BILIBILI_API_BASE: &str = "https://api.live.bilibili.com";
const BILIBILI_REFERER: &str = "https://live.bilibili.com";
const BILIBILI_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const WBI_WEB_LOCATION: &str = "444.8";

pub struct Bilibili {
    re: Regex,
    /// WBI key 状态跨 check_stream 共享，避免每轮重建导致 2 小时缓存失效
    wbi_signer: WbiSigner,
}

impl Default for Bilibili {
    fn default() -> Self {
        Self::new()
    }
}

impl Bilibili {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"(?:https?://)?(?:b23\.tv|live\.bilibili\.com)").unwrap(),
            wbi_signer: WbiSigner::new(),
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
        BilibiliLive::new(request, self.wbi_signer.clone())
            .check_stream()
            .await
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
    fn new(request: LiveRequest, wbi_signer: WbiSigner) -> Self {
        let options = request.options.bilibili;
        let credentials = request.credentials;
        Self {
            wbi_signer,
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
        let mut candidates = self
            .get_stream_candidates(&profile, headers.clone(), &self.protocol)
            .await?;
        if candidates.is_empty() && self.protocol == "hls_fmp4" {
            // fmp4 流可能尚未转码完成：等待窗口内按未开播处理，超时后回退 stream(flv) 协议
            if now_unix().saturating_sub(profile.live_start_time) <= self.hls_transcode_timeout {
                warn!("{}: 暂未提供 hls_fmp4 流，等待下一次检测", self.name);
                return Ok(LiveStatus::Offline);
            }
            candidates = self
                .get_stream_candidates(&profile, headers, "stream")
                .await?;
        }
        if candidates.is_empty() {
            return Err(LiveError::custom(format!("获取 {} 流失败", self.protocol)));
        }
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
        self.wbi_signer
            .sign(&self.client, &mut params, &headers)
            .await?;

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
        protocol: &str,
    ) -> LiveResult<Vec<BiliStreamCandidate>> {
        for api in &self.api_list {
            match self
                .request_play_info(api, profile.room_id, headers.clone())
                .await
            {
                Ok(play_info) => {
                    if let Some(candidates) = self
                        .parse_play_info(api, profile, &play_info, &headers, protocol)
                        .await
                        && !candidates.is_empty()
                    {
                        return Ok(candidates);
                    }
                }
                Err(err) => error!("{}: {api} 获取 play_info 失败: {err}", self.name),
            }
        }

        // 与 Python 一致：空结果照常返回，等待/协议回退交给上层处理
        Ok(Vec::new())
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
        self.wbi_signer
            .sign(&self.client, &mut params, &headers)
            .await?;

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
        protocol: &str,
    ) -> Option<Vec<BiliStreamCandidate>> {
        let streams = play_info
            .get("data")
            .and_then(|data| data.get("playurl_info"))
            .and_then(|info| info.get("playurl"))
            .and_then(|playurl| playurl.get("stream"))
            .and_then(Value::as_array)?;

        if protocol == "hls_fmp4" {
            match self
                .master_m3u8_candidates(api, profile, play_info, headers)
                .await
            {
                Ok(Some(candidates)) => return Some(candidates),
                Ok(None) => {}
                Err(err) => error!("{}: {api} 获取 m3u8 失败: {err}", self.name),
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
                .and_then(parse_codec_urls)?;
            // fmp4 可能没有原画
            if matches!(self.qn, 10000 | 25000)
                && !candidates.iter().any(|candidate| candidate.qn == self.qn)
            {
                return None;
            }
            return Some(candidates);
        }

        streams
            .first()
            .and_then(|stream| stream.get("format"))
            .and_then(Value::as_array)
            .and_then(|formats| formats.first())
            .and_then(|format| format.get("codec"))
            .and_then(Value::as_array)
            .and_then(|codecs| codecs.first())
            .and_then(parse_codec_urls)
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

        let mid = self.login_mid(headers).await.unwrap_or(profile.uid);
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

    /// 获取登录用户 mid。与 Python 一致：nav 请求失败或未登录时返回 None，
    /// 由调用方以主播 uid 兜底，不中断整次取流。
    async fn login_mid(&self, headers: &HeaderMap) -> Option<u64> {
        let response = match self
            .client
            .get("https://api.bilibili.com/x/web-interface/nav")
            .headers(headers.clone())
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => {
                error!("{}: 获取 B 站登录状态失败: {err}", self.name);
                return None;
            }
        };
        let value: Value = match response.json().await {
            Ok(value) => value,
            Err(err) => {
                error!("{}: 解析 B 站登录状态失败: {err}", self.name);
                return None;
            }
        };
        let mid = value
            .get("data")
            .and_then(|data| data.get("isLogin"))
            .and_then(Value::as_bool)
            .filter(|is_login| *is_login)
            .and_then(|_| value.get("data"))
            .and_then(|data| data.get("mid"))
            .and_then(Value::as_u64);
        if mid.is_none() {
            warn!("{}: 未登录，或将只能录制到最低画质。", self.name);
        }
        mid
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
        let first = candidates
            .first()
            .ok_or_else(|| LiveError::custom("B 站可用直播流为空"))?;
        // 按 bili_qn 精确匹配画质，选不中取第一组（master m3u8 已按 qn 降序，即最高画质）
        let target_qn = if candidates.iter().any(|candidate| candidate.qn == self.qn) {
            self.qn
        } else {
            first.qn
        };
        let group: Vec<&BiliStreamCandidate> = candidates
            .iter()
            .filter(|candidate| candidate.qn == target_qn)
            .collect();
        let selected = self
            .cdn
            .iter()
            .find_map(|cdn| group.iter().copied().find(|candidate| &candidate.cdn == cdn))
            .unwrap_or(group[0]);

        if !self.cdn_fallback {
            return Ok(selected.url.clone());
        }
        if let Some(url) = self.check_url_healthy(&selected.url).await {
            return Ok(url);
        }
        for candidate in &group {
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
    let mut groups: BTreeMap<u32, Vec<BiliStreamCandidate>> = BTreeMap::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with("#EXT-X-STREAM-INF:") {
            // 仅保留 CODECS 含 avc 的变体（跳过 hevc/av1）
            current_qn = parse_stream_inf_qn(line).filter(|_| {
                parse_stream_inf_codecs(line)
                    .is_some_and(|codecs| codecs.to_ascii_lowercase().contains("avc"))
            });
        } else if line.starts_with("http")
            && let Some(qn) = current_qn
        {
            let cdn = extract_cdn(line);
            if cdn.is_empty() {
                continue;
            }
            let group = groups.entry(qn).or_default();
            if !group.iter().any(|candidate| candidate.cdn == cdn) {
                group.push(BiliStreamCandidate {
                    qn,
                    cdn,
                    url: line.to_string(),
                });
            }
        }
    }
    // 按 qn 降序展开，供选流时缺省取最高画质
    let candidates: Vec<_> = groups
        .into_iter()
        .rev()
        .flat_map(|(_, group)| group)
        .collect();
    (!candidates.is_empty()).then_some(candidates)
}

fn parse_stream_inf_qn(line: &str) -> Option<u32> {
    Regex::new(r"BILI-QN=(\d+)")
        .unwrap()
        .captures(line)
        .and_then(|captures| captures[1].parse().ok())
}

fn parse_stream_inf_codecs(line: &str) -> Option<String> {
    Regex::new(r#"CODECS="([^"]+)""#)
        .unwrap()
        .captures(line)
        .map(|captures| captures[1].to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_only_live_and_short_urls() {
        let plugin = Bilibili::new();
        assert!(plugin.matches("https://live.bilibili.com/12345"));
        assert!(plugin.matches("http://live.bilibili.com/12345"));
        assert!(plugin.matches("live.bilibili.com/12345"));
        assert!(plugin.matches("https://b23.tv/abc123"));
        assert!(plugin.matches("b23.tv/abc123"));
        // 普通视频页/主站不应被认领
        assert!(!plugin.matches("https://www.bilibili.com/video/BV1xx411c7mD"));
        assert!(!plugin.matches("https://m.bilibili.com/video/BV1xx411c7mD"));
        assert!(!plugin.matches("https://bilibili.com/12345"));
        assert!(!plugin.matches("https://space.bilibili.com/1"));
    }

    const MASTER_M3U8: &str = concat!(
        "#EXTM3U\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=5000000,CODECS=\"hev1.1.6.L120.90,mp4a.40.2\",RESOLUTION=1920x1080,BILI-QN=20000\n",
        "https://cn-hevc.example.com/live-bvc/0/live_1_2/index.m3u8?cdn=cn-gotcha09&expires=1\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=2000000,CODECS=\"avc1.640028,mp4a.40.2\",RESOLUTION=1280x720,BILI-QN=400\n",
        "https://cn-sd.example.com/live-bvc/0/live_1_2/index.m3u8?cdn=cn-gotcha02&expires=1\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=4000000,CODECS=\"avc1.640032,mp4a.40.2\",RESOLUTION=1920x1080,BILI-QN=10000\n",
        "https://cn-hd.example.com/live-bvc/0/live_1_2/index.m3u8?cdn=cn-gotcha01&expires=1\n",
        "#EXT-X-STREAM-INF:BANDWIDTH=4000000,CODECS=\"avc1.640032,mp4a.40.2\",RESOLUTION=1920x1080,BILI-QN=10000\n",
        "https://ov-hd.example.com/live-bvc/0/live_1_2/index.m3u8?cdn=ov-gotcha05&expires=1\n",
    );

    #[test]
    fn parse_master_m3u8_filters_non_avc_and_sorts_by_qn_desc() {
        let candidates = parse_master_m3u8(MASTER_M3U8).unwrap();

        // hevc 变体被过滤
        assert!(candidates.iter().all(|candidate| candidate.qn != 20000));
        // 按 qn 降序：两个 10000 在前，400 在最后
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.qn)
                .collect::<Vec<_>>(),
            vec![10000, 10000, 400]
        );
        // cdn 从 URL 的 cdn= 参数提取
        assert_eq!(candidates[0].cdn, "cn-gotcha01");
        assert_eq!(candidates[1].cdn, "ov-gotcha05");
        assert_eq!(candidates[2].cdn, "cn-gotcha02");
        assert!(candidates[0].url.starts_with("https://cn-hd.example.com/"));
    }

    #[test]
    fn parse_master_m3u8_skips_variants_without_qn_or_codecs() {
        let text = concat!(
            "#EXTM3U\n",
            "#EXT-X-STREAM-INF:BANDWIDTH=4000000,CODECS=\"avc1.640032,mp4a.40.2\"\n",
            "https://a.example.com/index.m3u8?cdn=cn-gotcha01\n",
            "#EXT-X-STREAM-INF:BANDWIDTH=4000000,BILI-QN=10000\n",
            "https://b.example.com/index.m3u8?cdn=cn-gotcha02\n",
        );
        assert!(parse_master_m3u8(text).is_none());
    }

    #[test]
    fn parse_master_m3u8_dedups_same_qn_and_cdn() {
        let text = concat!(
            "#EXTM3U\n",
            "#EXT-X-STREAM-INF:CODECS=\"avc1.640032\",BILI-QN=10000\n",
            "https://a.example.com/index.m3u8?cdn=cn-gotcha01\n",
            "#EXT-X-STREAM-INF:CODECS=\"avc1.640032\",BILI-QN=10000\n",
            "https://b.example.com/index.m3u8?cdn=cn-gotcha01\n",
        );
        let candidates = parse_master_m3u8(text).unwrap();
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].url.starts_with("https://a.example.com/"));
    }
}
