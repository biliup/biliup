use super::{
    DanmakuSource, DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus,
    LiveStream, huya_wup, media_ext_from_url,
};
use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::Utc;
use md5::{Digest, Md5};
use rand::Rng;
use regex::Regex;
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

const HUYA_WEB_BASE_URL: &str = "https://www.huya.com";
const HUYA_MP_BASE_URL: &str = "https://mp.huya.com";
const HUYA_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct Huya {
    re: Regex,
    /// 别名房间号 -> 真实数字房间号，进程内缓存避免每轮轮询重复解析
    real_rid_cache: RwLock<HashMap<String, String>>,
}

impl Default for Huya {
    fn default() -> Self {
        Self::new()
    }
}

impl Huya {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"https?://(?:(?:www|m)\.)?huya\.com").unwrap(),
            real_rid_cache: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl LivePlugin for Huya {
    fn name(&self) -> &'static str {
        "Huya"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        HuyaLive::new(request, &self.real_rid_cache)
            .check_stream()
            .await
    }
}

struct HuyaLive<'a> {
    client: Client,
    url: String,
    name: String,
    huya_cdn: String,
    huya_max_ratio: u32,
    huya_protocol: HuyaProtocol,
    huya_imgplus: bool,
    huya_cdn_fallback: bool,
    huya_mobile_api: bool,
    huya_codec: String,
    huya_danmaku: bool,
    real_rid_cache: &'a RwLock<HashMap<String, String>>,
}

impl<'a> HuyaLive<'a> {
    fn new(request: LiveRequest, real_rid_cache: &'a RwLock<HashMap<String, String>>) -> Self {
        let options = request.options.huya;
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
            huya_cdn: options.cdn.to_uppercase(),
            huya_max_ratio: options.max_ratio,
            huya_protocol: HuyaProtocol::from_config(&options.protocol),
            huya_imgplus: options.imgplus,
            huya_cdn_fallback: options.cdn_fallback,
            huya_mobile_api: options.mobile_api,
            huya_codec: options.codec,
            huya_danmaku: options.danmaku,
            real_rid_cache,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        let room_id = self.resolve_room_id().await?;
        let Some(mut profile) = self.get_room_profile(&room_id).await? else {
            return Ok(LiveStatus::Offline);
        };

        if is_replay_title(&profile.title) {
            return Ok(LiveStatus::Offline);
        }

        let stream_urls = self.build_stream_urls(&profile.stream_info).await?;
        let (mut selected_cdn, mut raw_stream_url) =
            self.select_stream_url(&stream_urls, &profile)?;

        // HTTPS 的直播流只允许连接一次：健康检查会消耗掉这次连接，
        // 因此 cdn_fallback 无论是否回退，最后都要重新取一次流地址
        if self.huya_cdn_fallback {
            if !self.check_url_healthy(&raw_stream_url).await {
                let cdn_list: Vec<&str> =
                    stream_urls.iter().map(|(cdn, _)| cdn.as_str()).collect();
                info!(name = %self.name, "cdn_fallback 顺序尝试 {cdn_list:?}");
                let mut fallback = None;
                for (cdn, url) in &stream_urls {
                    if cdn == &selected_cdn {
                        continue;
                    }
                    info!(name = %self.name, "cdn_fallback-{cdn}");
                    if self.check_url_healthy(url).await {
                        fallback = Some(cdn.clone());
                        break;
                    }
                }
                let Some(fallback) = fallback else {
                    error!(name = %self.name, "cdn_fallback 所有链接无法使用");
                    return Ok(LiveStatus::Offline);
                };
                info!(name = %self.name, "cdn_fallback 回退到 {fallback}");
                selected_cdn = fallback;
            }

            let Some(next_profile) = self.get_room_profile(&room_id).await? else {
                return Ok(LiveStatus::Offline);
            };
            profile = next_profile;
            let stream_urls = self.build_stream_urls(&profile.stream_info).await?;
            let url = stream_urls
                .iter()
                .find(|(cdn, _)| cdn == &selected_cdn)
                .or_else(|| stream_urls.first())
                .map(|(_, url)| url)
                .ok_or_else(|| LiveError::custom("虎牙可用 CDN 为空"))?;
            raw_stream_url = self.add_ratio(url, &profile.bitrate_info, profile.max_bitrate);
        }

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: profile.title,
                date: Utc::now(),
                live_cover_url: profile.cover,
                suffix: media_ext_from_url(&raw_stream_url)
                    .unwrap_or_else(|| self.huya_protocol.extension().to_string()),
                raw_stream_url,
                platform: "huya".to_string(),
                stream_headers: HashMap::new(),
                danmaku: self.danmaku_source(),
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }

    /// 解析房间号：URL 路径本身是数字则直接使用，
    /// 否则请求房间页取 TT_ROOM_DATA.profileRoom（结果进程内缓存）
    async fn resolve_room_id(&self) -> LiveResult<String> {
        let path = self
            .url
            .split("huya.com/")
            .nth(1)
            .and_then(|part| part.split(['?', '#']).next())
            .filter(|part| !part.is_empty())
            .ok_or_else(|| LiveError::custom("虎牙直播间地址错误"))?;

        if path.chars().all(|c| c.is_ascii_digit()) {
            return Ok(path.to_string());
        }

        if let Some(rid) = self.real_rid_cache.read().await.get(path) {
            return Ok(rid.clone());
        }

        let page = self.fetch_room_page(path).await?;
        let room_data = extract_json_after(&page, r"var\s+TT_ROOM_DATA\s*=\s*", ';')?;
        let rid = room_data
            .get("profileRoom")
            .and_then(|rid| {
                rid.as_u64()
                    .map(|rid| rid.to_string())
                    .or_else(|| rid.as_str().map(str::to_string))
            })
            .filter(|rid| !rid.is_empty() && rid != "0")
            .ok_or_else(|| LiveError::custom("找不到这个主播"))?;
        self.real_rid_cache
            .write()
            .await
            .insert(path.to_string(), rid.clone());
        Ok(rid)
    }

    async fn get_room_profile(&self, room_id: &str) -> LiveResult<Option<HuyaRoomProfile>> {
        if self.huya_mobile_api {
            self.get_room_profile_mobile(room_id).await
        } else {
            let page = self.fetch_room_page(room_id).await?;
            self.extract_room_profile(&page)
        }
    }

    async fn get_room_profile_mobile(&self, room_id: &str) -> LiveResult<Option<HuyaRoomProfile>> {
        let text = self
            .client
            .get(format!("{HUYA_MP_BASE_URL}/cache.php"))
            .query(&[
                ("m", "Live"),
                ("do", "profileRoom"),
                ("roomid", room_id),
                ("showSecret", "1"),
            ])
            .header("referer", &self.url)
            .header("user-agent", HUYA_USER_AGENT)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("请求虎牙小程序接口失败: {err}")))?
            .text()
            .await
            .map_err(|err| LiveError::custom(format!("读取虎牙小程序接口失败: {err}")))?;

        let value: Value = serde_json::from_str(&decode_html_entities(&text))
            .map_err(|err| LiveError::custom(format!("解析虎牙小程序接口失败: {err}")))?;
        if value.get("status").and_then(Value::as_i64) != Some(200) {
            let message = value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("未知错误");
            return Err(LiveError::custom(format!("虎牙小程序接口错误: {message}")));
        }

        let data = value
            .get("data")
            .ok_or_else(|| LiveError::custom("虎牙小程序接口数据为空"))?;
        if data.get("liveStatus").and_then(Value::as_str) != Some("ON") {
            return Ok(None);
        }
        let live_data = data
            .get("liveData")
            .ok_or_else(|| LiveError::custom("虎牙直播信息为空"))?;
        // bitRateInfo 是内嵌 JSON 字符串；缺失视为未推流
        let Some(bitrate_json) = live_data
            .get("bitRateInfo")
            .and_then(Value::as_str)
            .filter(|info| !info.is_empty())
        else {
            return Ok(None);
        };
        let bitrate_info: Vec<Value> = serde_json::from_str(bitrate_json)
            .map_err(|err| LiveError::custom(format!("解析虎牙码率信息失败: {err}")))?;

        let stream_info = data
            .get("stream")
            .and_then(|stream| stream.get("baseSteamInfoList"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if stream_info.is_empty() {
            return Ok(None);
        }

        Ok(Some(HuyaRoomProfile {
            title: live_data
                .get("introduction")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            cover: live_data
                .get("screenshot")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .replace("http://", "https://"),
            max_bitrate: live_data
                .get("bitRate")
                .and_then(Value::as_u64)
                .unwrap_or_default() as u32,
            bitrate_info,
            stream_info,
        }))
    }

    async fn fetch_room_page(&self, room_id: &str) -> LiveResult<String> {
        let text = self
            .client
            .get(format!("{HUYA_WEB_BASE_URL}/{room_id}"))
            .header("referer", &self.url)
            .header("user-agent", HUYA_USER_AGENT)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取虎牙直播间页面失败: {err}")))?
            .text()
            .await
            .map_err(|err| LiveError::custom(format!("读取虎牙直播间页面失败: {err}")))?;

        if text.contains("找不到这个主播") || text.contains("该主播涉嫌违规，正在整改中")
        {
            return Err(LiveError::custom("虎牙直播间不可用"));
        }
        Ok(decode_html_entities(&text))
    }

    fn extract_room_profile(&self, page: &str) -> LiveResult<Option<HuyaRoomProfile>> {
        let room_data = extract_json_after(page, r"var\s+TT_ROOM_DATA\s*=\s*", ';')?;
        let room_state = room_data
            .get("state")
            .and_then(|state| state.as_str())
            .unwrap_or_default();

        let stream = extract_stream_json(page)?;
        let bitrate_info = stream
            .get("vMultiStreamInfo")
            .and_then(|info| info.as_array())
            .cloned()
            .unwrap_or_default();

        if room_state != "ON" || bitrate_info.is_empty() {
            return Ok(None);
        }

        let data = stream
            .get("data")
            .and_then(|data| data.as_array())
            .and_then(|data| data.first())
            .ok_or_else(|| LiveError::custom("虎牙流数据为空"))?;
        let live_info = data
            .get("gameLiveInfo")
            .ok_or_else(|| LiveError::custom("虎牙直播信息为空"))?;
        let stream_info = data
            .get("gameStreamInfoList")
            .and_then(|info| info.as_array())
            .cloned()
            .unwrap_or_default();
        if stream_info.is_empty() {
            return Ok(None);
        }

        Ok(Some(HuyaRoomProfile {
            title: live_info
                .get("introduction")
                .and_then(|title| title.as_str())
                .unwrap_or_default()
                .to_string(),
            cover: live_info
                .get("screenshot")
                .and_then(|cover| cover.as_str())
                .unwrap_or_default()
                .replace("http://", "https://"),
            max_bitrate: live_info
                .get("bitRate")
                .and_then(|bitrate| bitrate.as_u64())
                .unwrap_or_default() as u32,
            bitrate_info,
            stream_info,
        }))
    }

    async fn build_stream_urls(&self, streams_info: &[Value]) -> LiveResult<Vec<(String, String)>> {
        let mut streams = Vec::new();
        // 同一房间所有 CDN 的 stream_name 相同，防盗链参数只需计算一次
        let mut cached_anticode: Option<String> = None;

        for stream in streams_info {
            let priority = stream
                .get("iWebPriorityRate")
                .and_then(|priority| priority.as_i64())
                .unwrap_or_default();
            if priority < 0 {
                continue;
            }

            let stream_name = self.get_stream_name(json_str(stream, "sStreamName")?);
            let cdn = json_str(stream, "sCdnType")?.to_string();
            let suffix = json_str(stream, self.huya_protocol.suffix_key())?;
            let base_url =
                json_str(stream, self.huya_protocol.url_key())?.replace("http://", "https://");
            let presenter_uid = stream
                .get("lPresenterUid")
                .and_then(|uid| {
                    uid.as_u64()
                        .or_else(|| uid.as_str().and_then(|uid| uid.parse().ok()))
                })
                .unwrap_or_default();

            if cached_anticode.is_none() {
                // 小程序 API 且保留 imgplus 时直接使用接口返回的防盗链参数；
                // 其余情况用 WUP getCdnTokenInfoEx 取新 token 重建（imgplus=false 改写了
                // stream_name，页面自带的防盗链参数对改写后的流名无效）
                let anticode = if self.huya_mobile_api && self.huya_imgplus {
                    json_str(stream, self.huya_protocol.anticode_key())?.to_string()
                } else {
                    match self.get_cdn_token_info_ex(&stream_name).await {
                        Ok(token) => build_anticode(&stream_name, &token, presenter_uid)?,
                        Err(err) => {
                            warn!(
                                name = %self.name,
                                err = %err,
                                "虎牙 getCdnTokenInfoEx 失败，回退页面防盗链参数"
                            );
                            build_anticode(
                                &stream_name,
                                json_str(stream, self.huya_protocol.anticode_key())?,
                                presenter_uid,
                            )?
                        }
                    }
                };
                cached_anticode = Some(format!("{anticode}&codec={}", self.huya_codec));
            }
            let anti_code = cached_anticode.as_deref().unwrap_or_default();
            let url = format!("{base_url}/{stream_name}.{suffix}?{anti_code}");
            streams.push((cdn, priority, url));
        }

        streams.sort_by_key(|(_, priority, _)| std::cmp::Reverse(*priority));
        Ok(streams
            .into_iter()
            .filter(|(cdn, _, _)| !matches!(cdn.as_str(), "HY" | "HUYA" | "HYZJ"))
            .map(|(cdn, _, url)| (cdn, url))
            .collect())    }

    async fn get_cdn_token_info_ex(&self, stream_name: &str) -> LiveResult<String> {
        let ua = huya_wup::random_hyapp_ua();
        let payload = huya_wup::encode_get_cdn_token_ex(stream_name, &ua);
        let url = if rand::thread_rng().gen_bool(0.5) {
            huya_wup::WUP_YST_URL
        } else {
            huya_wup::WUP_MAIN_URL
        };
        let body = self
            .client
            .post(url)
            .body(payload)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("请求虎牙 getCdnTokenInfoEx 失败: {err}")))?
            .bytes()
            .await
            .map_err(|err| LiveError::custom(format!("读取虎牙 getCdnTokenInfoEx 失败: {err}")))?;
        huya_wup::decode_get_cdn_token_ex(&body)
            .filter(|token| !token.is_empty())
            .ok_or_else(|| LiveError::custom("解析虎牙 getCdnTokenInfoEx 响应失败"))
    }

    /// 流地址健康检查：能成功建立连接（m3u8 还要求首个变体列表可取）即视为可用。
    /// 注意这次连接会被服务端计数，调用方需要在检查后重新取流地址。
    async fn check_url_healthy(&self, url: &str) -> bool {
        let response = match self
            .client
            .get(url)
            .header("referer", HUYA_WEB_BASE_URL)
            .header("user-agent", HUYA_USER_AGENT)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => response,
            _ => return false,
        };

        if !url.contains(".m3u8") {
            return true;
        }
        let Ok(playlist) = response.text().await else {
            return false;
        };
        let Some(variant) = first_m3u8_variant(&playlist) else {
            // 没有变体说明本身就是媒体列表，能取到即健康
            return true;
        };
        let variant_url = match resolve_relative_url(url, &variant) {
            Some(resolved) => resolved,
            None => return false,
        };
        matches!(
            self.client
                .get(variant_url)
                .header("referer", HUYA_WEB_BASE_URL)
                .header("user-agent", HUYA_USER_AGENT)
                .send()
                .await,
            Ok(response) if response.status().is_success()
        )
    }

    fn get_stream_name(&self, stream_name: &str) -> String {
        if self.huya_imgplus {
            stream_name.to_string()
        } else {
            stream_name.replace("-imgplus", "")
        }
    }

    fn select_stream_url(
        &self,
        stream_urls: &[(String, String)],
        profile: &HuyaRoomProfile,
    ) -> LiveResult<(String, String)> {
        let (cdn, url) = stream_urls
            .iter()
            .find(|(cdn, _)| !self.huya_cdn.is_empty() && cdn == &self.huya_cdn)
            .or_else(|| stream_urls.first())
            .ok_or_else(|| LiveError::custom("虎牙可用 CDN 为空"))?;

        Ok((
            cdn.clone(),
            self.add_ratio(url, &profile.bitrate_info, profile.max_bitrate),
        ))
    }

    fn add_ratio(&self, url: &str, bitrate_info: &[Value], max_bitrate: u32) -> String {
        if self.huya_max_ratio == 0 || url.contains("&ratio") {
            return url.to_string();
        }

        let selected_ratio = bitrate_info
            .iter()
            .filter_map(|info| {
                // 与 Python 语义一致：iBitRate 缺失或为 0 都视为原画码率 max_bitrate
                let bitrate = match info.get("iBitRate").and_then(|bitrate| bitrate.as_u64()) {
                    Some(bitrate) if bitrate > 0 => bitrate as u32,
                    _ => max_bitrate,
                };
                (bitrate <= self.huya_max_ratio).then_some(bitrate)
            })
            .max();

        match selected_ratio {
            Some(ratio) if ratio > 0 => format!("{url}&ratio={ratio}"),
            _ => url.to_string(),
        }
    }

    fn danmaku_source(&self) -> Option<DanmakuSource> {
        if !self.huya_danmaku {
            return None;
        }
        Some(DanmakuSource {
            platform: "huya".to_string(),
            url: self.url.clone(),
            room_id: None,
            cookie: None,
            raw: false,
            detail: false,
            extra: HashMap::new(),
            movie_id: None,
            password: None,
        })
    }
}

struct HuyaRoomProfile {
    title: String,
    cover: String,
    max_bitrate: u32,
    bitrate_info: Vec<Value>,
    stream_info: Vec<Value>,
}

enum HuyaProtocol {
    Flv,
    Hls,
}

impl HuyaProtocol {
    fn from_config(value: &str) -> Self {
        if value == "Hls" { Self::Hls } else { Self::Flv }
    }

    fn url_key(&self) -> &'static str {
        match self {
            Self::Flv => "sFlvUrl",
            Self::Hls => "sHlsUrl",
        }
    }

    fn suffix_key(&self) -> &'static str {
        match self {
            Self::Flv => "sFlvUrlSuffix",
            Self::Hls => "sHlsUrlSuffix",
        }
    }

    fn anticode_key(&self) -> &'static str {
        match self {
            Self::Flv => "sFlvAntiCode",
            Self::Hls => "sHlsAntiCode",
        }
    }

    fn extension(&self) -> &'static str {
        match self {
            Self::Flv => "flv",
            Self::Hls => "m3u8",
        }
    }
}

fn is_replay_title(title: &str) -> bool {
    let head: String = title.chars().take(3).collect();
    let tail: String = title
        .chars()
        .skip(title.chars().count().saturating_sub(3))
        .collect();
    ["回放", "重播"]
        .iter()
        .any(|key| head.contains(key) || tail.contains(key))
}

fn extract_json_after(page: &str, pattern: &str, end: char) -> LiveResult<Value> {
    let re = Regex::new(pattern).unwrap();
    let Some(mat) = re.find(page) else {
        return Err(LiveError::custom("虎牙房间数据不存在"));
    };
    let start = mat.end();
    let end = page[start..]
        .find(end)
        .map(|idx| start + idx)
        .ok_or_else(|| LiveError::custom("虎牙房间数据不完整"))?;
    serde_json::from_str(page[start..end].trim())
        .map_err(|err| LiveError::custom(format!("解析虎牙房间数据失败: {err}")))
}

fn extract_stream_json(page: &str) -> LiveResult<Value> {
    let Some(start) = page.find("stream: ").map(|idx| idx + "stream: ".len()) else {
        return Err(LiveError::custom("虎牙流数据不存在"));
    };
    let end =
        find_json_value_end(page, start).ok_or_else(|| LiveError::custom("虎牙流数据不完整"))?;
    serde_json::from_str(page[start..end].trim())
        .map_err(|err| LiveError::custom(format!("解析虎牙流数据失败: {err}")))
}

fn find_json_value_end(input: &str, start: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut idx = start;
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }

    let opening = *bytes.get(idx)?;
    let closing = match opening {
        b'{' => b'}',
        b'[' => b']',
        _ => return None,
    };

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, byte) in bytes[idx..].iter().copied().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => depth += 1,
            b'}' | b']' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if depth == 0 && byte == closing {
                    return Some(idx + offset + 1);
                }
            }
            _ => {}
        }
    }
    None
}

fn build_anticode(
    stream_name: &str,
    anti_code: &str,
    presenter_uid: u64,
) -> LiveResult<String> {
    let query = serde_urlencoded::from_str::<HashMap<String, String>>(anti_code)
        .map_err(|err| LiveError::custom(format!("解析虎牙防盗链参数失败: {err}")))?;
    let Some(fm) = query.get("fm") else {
        return Ok(anti_code.to_string());
    };

    let ctype = query
        .get("ctype")
        .cloned()
        .unwrap_or_else(|| "huya_live".to_string());
    let platform_id = query.get("t").cloned().unwrap_or_else(|| "100".to_string());
    let is_wap = matches!(platform_id.parse::<u64>(), Ok(103));
    let uid = if presenter_uid == 0 {
        generate_random_uid()
    } else {
        presenter_uid
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| LiveError::custom(format!("获取系统时间失败: {err}")))?;
    let now_secs = now.as_secs();
    let seq_id = uid + now.as_millis() as u64;
    let secret_hash = md5_hex(format!("{seq_id}|{ctype}|{platform_id}"));
    let convert_uid = rotl64(uid);
    let fm = urlencoding::decode(fm)
        .map_err(|err| LiveError::custom(format!("解码虎牙 fm 参数失败: {err}")))?
        .to_string();
    let secret_prefix = String::from_utf8(
        STANDARD
            .decode(fm.as_bytes())
            .map_err(|err| LiveError::custom(format!("解码虎牙 fm base64 失败: {err}")))?,
    )
    .map_err(|err| LiveError::custom(format!("虎牙 fm 参数不是 UTF-8: {err}")))?
    .split('_')
    .next()
    .unwrap_or_default()
    .to_string();

    let mut ws_time = query
        .get("wsTime")
        .cloned()
        .ok_or_else(|| LiveError::custom("虎牙 wsTime 为空"))?;
    if u64::from_str_radix(&ws_time, 16).unwrap_or_default() < now_secs + 20 * 60 {
        ws_time = format!("{:x}", now_secs + 24 * 60 * 60);
    }

    // wap 平台(t=103)用原始 uid 参与 wsSecret 计算，其余平台用 convert_uid
    let calc_uid = if is_wap { uid } else { convert_uid };
    let secret_str = format!("{secret_prefix}_{calc_uid}_{stream_name}_{secret_hash}_{ws_time}");
    let ws_secret = md5_hex(secret_str);
    let fs = query
        .get("fs")
        .cloned()
        .unwrap_or_else(|| "bgct".to_string());
    let fm = urlencoding::encode(query.get("fm").map(String::as_str).unwrap_or_default());

    let base = format!(
        "wsSecret={ws_secret}&wsTime={ws_time}&seqid={seq_id}&ctype={ctype}&ver=1&fs={fs}&fm={fm}&t={platform_id}"
    );
    Ok(if is_wap {
        let mut rng = rand::thread_rng();
        let ws_time_secs = u64::from_str_radix(&ws_time, 16).unwrap_or_default();
        let ct = ((ws_time_secs as f64 + rng.gen_range(0.0..1.0)) * 1000.0) as u64;
        let uuid = (((ct % 10_000_000_000) as f64 + rng.gen_range(0.0..1.0)) * 1e3
            % f64::from(u32::MAX)) as u64;
        format!("{base}&uid={uid}&uuid={uuid}")
    } else {
        format!("{base}&u={convert_uid}")
    })
}

fn json_str<'a>(value: &'a Value, key: &str) -> LiveResult<&'a str> {
    value
        .get(key)
        .and_then(|value| value.as_str())
        .ok_or_else(|| LiveError::custom(format!("虎牙字段 {key} 为空")))
}

fn md5_hex(input: String) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn rotl64(value: u64) -> u64 {
    (((value & 0xFFFF_FFFF) << 8) | ((value & 0xFFFF_FFFF) >> 24)) & 0xFFFF_FFFF
        | (value & !0xFFFF_FFFF)
}

fn generate_random_uid() -> u64 {
    let mut rng = rand::thread_rng();
    if rng.gen_bool(0.5) {
        format!("1234{:04}", rng.gen_range(0..10000))
            .parse()
            .unwrap_or(12340000)
    } else {
        format!("140000{:07}", rng.gen_range(0..10000000))
            .parse()
            .unwrap_or(1400000000000)
    }
}

fn decode_html_entities(input: &str) -> String {
    input
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&#x22;", "\"")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

/// 返回 master m3u8 中第一个 #EXT-X-STREAM-INF 变体的 URI；媒体列表返回 None
fn first_m3u8_variant(playlist: &str) -> Option<String> {
    let mut lines = playlist.lines().map(str::trim);
    while let Some(line) = lines.next() {
        if line.starts_with("#EXT-X-STREAM-INF") {
            for candidate in lines.by_ref() {
                if !candidate.is_empty() && !candidate.starts_with('#') {
                    return Some(candidate.to_string());
                }
            }
        }
    }
    None
}

fn resolve_relative_url(base: &str, target: &str) -> Option<String> {
    if target.starts_with("http://") || target.starts_with("https://") {
        return Some(target.to_string());
    }
    url::Url::parse(base)
        .ok()?
        .join(target)
        .ok()
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_title_matches_keyword_within_first_or_last_three_chars() {
        // 关键词整体出现在前三个字符内
        assert!(is_replay_title("【回放】昨天的比赛"));
        assert!(is_replay_title("回放：昨天的比赛"));
        assert!(is_replay_title("重播比赛"));
        // 关键词整体出现在后三个字符内
        assert!(is_replay_title("昨天的比赛回放"));
        assert!(is_replay_title("昨天的比赛重播】"));
        // 标题不足三个字符时取整个标题
        assert!(is_replay_title("回放"));
        // 关键词跨越前三/后三边界或出现在中间时不过滤
        assert!(!is_replay_title("精彩回放合集视频节目"));
        assert!(!is_replay_title("每日【回放】剪辑合集"));
        assert!(!is_replay_title("正在直播"));
        assert!(!is_replay_title(""));
    }

    #[test]
    fn extract_stream_json_stops_before_player_config_closing_brace() {
        let page = r#"
            var hyPlayerConfig = {
                stream: {"data":[],"vMultiStreamInfo":[]}
            };
        "#;

        let stream = extract_stream_json(page).unwrap();

        assert_eq!(stream.get("data").unwrap().as_array().unwrap().len(), 0);
        assert!(stream.get("vMultiStreamInfo").unwrap().is_array());
    }

    #[test]
    fn extract_stream_json_stops_before_following_player_fields() {
        let page = r#"
            var hyPlayerConfig = {
                stream: {"data":[],"vMultiStreamInfo":[]},
                liveLineUrl: "https://example.invalid"
            };
        "#;

        let stream = extract_stream_json(page).unwrap();

        assert_eq!(stream.get("data").unwrap().as_array().unwrap().len(), 0);
        assert!(stream.get("vMultiStreamInfo").unwrap().is_array());
    }

    #[test]
    fn find_json_value_end_ignores_braces_in_strings() {
        let input = r#"{"text":"}; { ]","items":[{"value":1}]}
            };
        "#;

        let end = find_json_value_end(input, 0).unwrap();
        let value: Value = serde_json::from_str(&input[..end]).unwrap();

        assert_eq!(value["text"], "}; { ]");
        assert_eq!(value["items"][0]["value"], 1);
    }
}
