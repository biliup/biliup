use super::{
    DanmakuSource, DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus,
    LiveStream, media_ext_from_url,
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

const HUYA_WEB_BASE_URL: &str = "https://www.huya.com";
const HUYA_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct Huya {
    re: Regex,
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
        HuyaLive::new(request).check_stream().await
    }
}

struct HuyaLive {
    client: Client,
    url: String,
    name: String,
    huya_cdn: String,
    huya_max_ratio: u32,
    huya_protocol: HuyaProtocol,
    huya_imgplus: bool,
    huya_codec: String,
    huya_danmaku: bool,
}

impl HuyaLive {
    fn new(request: LiveRequest) -> Self {
        let options = request.options.huya;
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
            huya_cdn: options.cdn.to_uppercase(),
            huya_max_ratio: options.max_ratio,
            huya_protocol: HuyaProtocol::from_config(&options.protocol),
            huya_imgplus: options.imgplus,
            huya_codec: options.codec,
            huya_danmaku: options.danmaku,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        let page = self.get_room_page().await?;
        let Some(profile) = self.extract_room_profile(&page)? else {
            return Ok(LiveStatus::Offline);
        };

        if profile.title.starts_with("回放")
            || profile.title.starts_with("重播")
            || profile.title.ends_with("回放")
            || profile.title.ends_with("重播")
        {
            return Ok(LiveStatus::Offline);
        }

        let stream_urls = self.build_stream_urls(&profile.stream_info)?;
        let raw_stream_url = self.select_stream_url(&stream_urls, &profile)?;

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

    async fn get_room_page(&self) -> LiveResult<String> {
        let room_id = self
            .url
            .split("huya.com/")
            .nth(1)
            .and_then(|part| part.split('?').next())
            .filter(|part| !part.is_empty())
            .ok_or_else(|| LiveError::custom("虎牙直播间地址错误"))?;

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

    fn build_stream_urls(&self, streams_info: &[Value]) -> LiveResult<Vec<(String, String)>> {
        let mut streams = Vec::new();

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
            let anti_code = json_str(stream, self.huya_protocol.anticode_key())?;
            let base_url =
                json_str(stream, self.huya_protocol.url_key())?.replace("http://", "https://");
            let anti_code = build_anticode(&stream_name, anti_code)?;
            let url = format!(
                "{base_url}/{stream_name}.{suffix}?{anti_code}&codec={}",
                self.huya_codec
            );
            streams.push((cdn, priority, url));
        }

        streams.sort_by(|a, b| b.1.cmp(&a.1));
        Ok(streams
            .into_iter()
            .filter(|(cdn, _, _)| !matches!(cdn.as_str(), "HY" | "HUYA" | "HYZJ"))
            .map(|(cdn, _, url)| (cdn, url))
            .collect())
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
    ) -> LiveResult<String> {
        let selected_url = stream_urls
            .iter()
            .find(|(cdn, _)| !self.huya_cdn.is_empty() && cdn == &self.huya_cdn)
            .or_else(|| stream_urls.first())
            .map(|(_, url)| url)
            .ok_or_else(|| LiveError::custom("虎牙可用 CDN 为空"))?;

        Ok(self.add_ratio(selected_url, &profile.bitrate_info, profile.max_bitrate))
    }

    fn add_ratio(&self, url: &str, bitrate_info: &[Value], max_bitrate: u32) -> String {
        if self.huya_max_ratio == 0 || url.contains("&ratio") {
            return url.to_string();
        }

        let selected_ratio = bitrate_info
            .iter()
            .filter_map(|info| {
                let bitrate = info
                    .get("iBitRate")
                    .and_then(|bitrate| bitrate.as_u64())
                    .unwrap_or(max_bitrate as u64) as u32;
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

fn build_anticode(stream_name: &str, anti_code: &str) -> LiveResult<String> {
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
    let uid = generate_random_uid();
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

    let secret_str = format!("{secret_prefix}_{convert_uid}_{stream_name}_{secret_hash}_{ws_time}");
    let ws_secret = md5_hex(secret_str);
    let fs = query
        .get("fs")
        .cloned()
        .unwrap_or_else(|| "bgct".to_string());
    let fm = urlencoding::encode(query.get("fm").map(String::as_str).unwrap_or_default());

    Ok(format!(
        "wsSecret={ws_secret}&wsTime={ws_time}&seqid={seq_id}&ctype={ctype}&ver=1&fs={fs}&fm={fm}&t={platform_id}&u={convert_uid}"
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

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
