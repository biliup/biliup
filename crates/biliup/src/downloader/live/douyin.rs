use super::{
    DanmakuSource, DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus,
    LiveStream, media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use libsm::sm3::hash::Sm3Hash;
use rand::Rng;
use rand::seq::SliceRandom;
use regex::Regex;
use reqwest::Client;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tracing::{debug, warn};

const DOUYIN_LIVE_URL: &str = "https://live.douyin.com/";
const DOUYIN_WEBCAST_ENTER_URL: &str = "https://live.douyin.com/webcast/room/web/enter/";
const DOUYIN_APP_REFLOW_URL: &str = "https://webcast.amemv.com/webcast/room/reflow/info/";
const DOUYIN_UNION_REGISTER_URL: &str = "https://ttwid.bytedance.com/ttwid/union/register/";
const DOUYIN_BASE_URL: &str = "https://www.douyin.com";
const DOUYIN_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36";
const DEFAULT_TTWID: &str = "1%7Cu7ogdHsSmHtxbt4hjDCNvcLfVJz78CTM0TTWU8Hio8w%7C1751545220%7C18aac967e501e9d6c13384335ced3523c46a0b1cc4535c7213bc2506a7f462c8";

pub struct Douyin {
    re: Regex,
    /// 进程级 ttwid 缓存，首次成功获取后复用（对应 Python DouyinUtils._douyin_ttwid 类级缓存）
    ttwid: Mutex<Option<String>>,
}

impl Default for Douyin {
    fn default() -> Self {
        Self::new()
    }
}

impl Douyin {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"https?://(?:(?:www|m|live|v)\.)?douyin\.com").unwrap(),
            ttwid: Mutex::new(None),
        }
    }
}

#[async_trait]
impl LivePlugin for Douyin {
    fn name(&self) -> &'static str {
        "Douyin"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        DouyinLive::new(request, &self.ttwid).check_stream().await
    }
}

struct DouyinLive<'a> {
    client: Client,
    url: String,
    name: String,
    douyin_quality: String,
    douyin_protocol: String,
    douyin_double_screen: bool,
    douyin_true_origin: bool,
    cookie: String,
    douyin_danmaku: bool,
    web_rid: Option<String>,
    room_id: Option<String>,
    sec_uid: Option<String>,
    ttwid_cache: &'a Mutex<Option<String>>,
}

impl<'a> DouyinLive<'a> {
    fn new(request: LiveRequest, ttwid_cache: &'a Mutex<Option<String>>) -> Self {
        let options = request.options.douyin;
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
            douyin_quality: options.quality,
            douyin_protocol: options.protocol,
            douyin_double_screen: options.double_screen,
            douyin_true_origin: options.true_origin,
            cookie: request.credentials.douyin_cookie.unwrap_or_default(),
            douyin_danmaku: options.danmaku,
            web_rid: None,
            room_id: None,
            sec_uid: None,
            ttwid_cache,
        }
    }

    async fn check_stream(&mut self) -> LiveResult<LiveStatus> {
        self.ensure_cookie().await;
        if !self.resolve_room_keys().await? {
            return Ok(LiveStatus::Offline);
        }
        let Some(room_info) = self.get_room_info().await? else {
            return Ok(LiveStatus::Offline);
        };
        let raw_stream_url = self.select_stream_url(&room_info)?;
        let title = room_info
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let cover = room_info
            .pointer("/cover/url_list")
            .and_then(Value::as_array)
            .and_then(|url_list| url_list.first())
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title,
                date: Utc::now(),
                live_cover_url: cover,
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "flv".to_string()),
                raw_stream_url,
                platform: "douyin".to_string(),
                stream_headers: self.stream_headers(),
                danmaku: self.danmaku_source(),
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }

    async fn ensure_cookie(&mut self) {
        if !self.cookie.is_empty() && !self.cookie.ends_with(';') {
            self.cookie.push(';');
        }
        if !self.cookie.contains("ttwid") {
            let ttwid = self.cached_ttwid().await;
            self.cookie.push_str(&format!("ttwid={ttwid};"));
        }
        if !self.cookie.contains("odin_ttid=") {
            self.cookie
                .push_str(&format!("odin_ttid={};", generate_odin_ttid()));
        }
        if !self.cookie.contains("__ac_nonce=") {
            self.cookie
                .push_str(&format!("__ac_nonce={};", generate_nonce()));
        }
    }

    /// 进程内首次成功获取 ttwid 后复用；获取失败回退 DEFAULT_TTWID 但不缓存失败结果
    async fn cached_ttwid(&self) -> String {
        let mut cached = self.ttwid_cache.lock().await;
        if let Some(ttwid) = cached.as_deref() {
            return ttwid.to_string();
        }
        match fetch_ttwid(&self.client).await {
            Some(ttwid) => {
                *cached = Some(ttwid.clone());
                ttwid
            }
            None => DEFAULT_TTWID.to_string(),
        }
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(DOUYIN_USER_AGENT));
        headers.insert(REFERER, HeaderValue::from_static(DOUYIN_LIVE_URL));
        if !self.cookie.is_empty()
            && let Ok(cookie) = HeaderValue::from_str(&self.cookie)
        {
            headers.insert(COOKIE, cookie);
        }
        headers
    }

    /// 解析直播间标识；返回 false 表示已确定未开播（如用户主页无 web_rid）
    async fn resolve_room_keys(&mut self) -> LiveResult<bool> {
        if self.url.contains("v.douyin") {
            let resp = self
                .client
                .get(&self.url)
                .headers(self.headers())
                .send()
                .await
                .map_err(|err| LiveError::custom(format!("解析抖音短链接失败: {err}")))?;
            let url = resp.url().to_string();
            if url.contains("webcast.amemv") {
                self.sec_uid = capture(&url, r"[?&]sec_user_id=([^&]+)");
                self.room_id =
                    capture(&url, r"/reflow/(\d+)").or_else(|| capture(&url, r"room_id=(\d+)"));
                return Ok(true);
            }
            if url.contains("isedouyin.com/share/user") {
                self.sec_uid = capture(&url, r"[?&]sec_uid=([^&]+)");
                return Ok(true);
            }
            self.url = url;
        }

        if self.url.contains("/user/") {
            let sec_uid = self
                .url
                .split("/user/")
                .nth(1)
                .and_then(|part| part.split('?').next())
                .unwrap_or_default();
            if matches!(sec_uid.len(), 55 | 76) {
                self.sec_uid = Some(sec_uid.to_string());
                return Ok(true);
            }

            let text = self
                .client
                .get(&self.url)
                .headers(self.headers())
                .send()
                .await
                .map_err(|err| LiveError::custom(format!("获取抖音用户页失败: {err}")))?
                .text()
                .await
                .map_err(|err| LiveError::custom(format!("读取抖音用户页失败: {err}")))?;
            let render_data = text
                .split(r#"<script id="RENDER_DATA" type="application/json">"#)
                .nth(1)
                .and_then(|part| part.split("</script>").next())
                .and_then(|part| urlencoding::decode(part).ok())
                .map(|cow| cow.to_string())
                .ok_or_else(|| LiveError::custom("抖音房间号获取失败"))?;
            self.web_rid = capture(&render_data, r#""web_rid":"([^"]+)""#);
            if self.web_rid.is_none() {
                debug!(name = %self.name, url = %self.url, "未开播");
                return Ok(false);
            }
            return Ok(true);
        }

        let web_rid = self
            .url
            .split("douyin.com/")
            .nth(1)
            .and_then(|part| part.split('/').next())
            .and_then(|part| part.split('?').next())
            .map(|part| part.trim_start_matches('+').to_string())
            .filter(|part| !part.is_empty())
            .ok_or_else(|| LiveError::custom("抖音直播间地址错误"))?;
        self.web_rid = Some(web_rid);
        Ok(true)
    }

    async fn get_web_room_info(&self, web_rid: &str) -> LiveResult<Value> {
        let mut params = common_params();
        params.push(("web_rid".to_string(), web_rid.to_string()));
        params.push(("is_need_double_stream".to_string(), "false".to_string()));
        params.push(("msToken".to_string(), generate_ms_token()));
        let query = sign_query(&params)?;
        self.client
            .get(format!("{DOUYIN_WEBCAST_ENTER_URL}?{query}"))
            .headers(self.headers())
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取抖音直播间信息失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析抖音直播间信息失败: {err}")))
    }

    async fn get_h5_room_info(&self) -> LiveResult<Value> {
        let sec_uid = self
            .sec_uid
            .as_deref()
            .ok_or_else(|| LiveError::custom("抖音 sec_user_id 为空"))?;
        let verify_fp = gen_verify_fp();
        let params = vec![
            (
                "room_id".to_string(),
                self.room_id.clone().unwrap_or_else(|| "2".to_string()),
            ),
            ("sec_user_id".to_string(), sec_uid.to_string()),
            ("type_id".to_string(), "0".to_string()),
            ("live_id".to_string(), "1".to_string()),
            ("version_code".to_string(), "99.99.99".to_string()),
            ("app_id".to_string(), "1128".to_string()),
            ("aid".to_string(), "6383".to_string()),
            ("verifyFp".to_string(), verify_fp),
            ("msToken".to_string(), generate_ms_token()),
        ];
        let query = sign_query(&params)?;
        self.client
            .get(format!("{DOUYIN_APP_REFLOW_URL}?{query}"))
            .headers(self.headers())
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取抖音 H5 直播间信息失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析抖音 H5 直播间信息失败: {err}")))
    }

    async fn get_room_info(&mut self) -> LiveResult<Option<Value>> {
        let mut response = Value::Null;
        if let Some(web_rid) = &self.web_rid {
            response = self.get_web_room_info(web_rid).await?;
            if let Some(sec_uid) = response
                .pointer("/data/user/sec_uid")
                .and_then(Value::as_str)
                .filter(|sec_uid| !sec_uid.is_empty())
            {
                self.sec_uid = Some(sec_uid.to_string());
            }
            if response.pointer("/data/user").is_none() {
                let prompts = response
                    .pointer("/data/prompts")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if prompts == "直播已结束" {
                    return Ok(None);
                }
                // 对应 Python 在此直接抛异常（可能是用户被封禁）；Rust 保留 H5 回退，仅提示
                warn!(
                    name = %self.name,
                    url = %self.url,
                    prompts,
                    response = %response,
                    "抖音 web 端接口未返回用户信息，可能被风控或封禁，尝试 H5 接口"
                );
            }
        }

        let mut room_info = response
            .pointer("/data/data")
            .and_then(Value::as_array)
            .and_then(|data| data.first())
            .cloned();

        if room_info.is_none() {
            response = self.get_h5_room_info().await?;
            if let Some(web_rid) = response
                .pointer("/data/room/owner/web_rid")
                .and_then(Value::as_str)
                .filter(|web_rid| !web_rid.is_empty())
            {
                self.web_rid = Some(web_rid.to_string());
            }
            room_info = response.pointer("/data/room").cloned();
        }

        let Some(room_info) = room_info else {
            return Ok(None);
        };
        if room_info.get("status").and_then(Value::as_i64) != Some(2) {
            return Ok(None);
        }
        self.room_id = room_info
            .get("id_str")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        Ok(Some(room_info))
    }

    fn select_stream_url(&self, room_info: &Value) -> LiveResult<String> {
        let mut pull_data = room_info.pointer("/stream_url/live_core_sdk_data/pull_data");
        if self.douyin_double_screen
            && let Some(double_screen) = room_info
                .pointer("/stream_url/pull_datas")
                .and_then(Value::as_object)
                .and_then(|pull_datas| pull_datas.values().next())
        {
            pull_data = Some(double_screen);
        }
        let stream_data_text = pull_data
            .and_then(|pull_data| pull_data.get("stream_data"))
            .and_then(Value::as_str)
            .ok_or_else(|| LiveError::custom("抖音直播流数据为空"))?;
        let stream_data: Value = serde_json::from_str(stream_data_text)
            .map_err(|err| LiveError::custom(format!("解析抖音直播流数据失败: {err}")))?;
        let stream_data = stream_data
            .get("data")
            .and_then(Value::as_object)
            .ok_or_else(|| LiveError::custom("抖音直播流清晰度为空"))?;

        if self.douyin_true_origin
            && self.douyin_quality == "origin"
            && self.douyin_protocol != "hls"
            && let Some(url) = stream_data
                .get("ao")
                .and_then(|quality| quality.pointer("/main/flv"))
                .and_then(Value::as_str)
                .filter(|url| !url.is_empty())
        {
            return Ok(url
                .replace("&only_audio=1", "")
                .replace("http://", "https://"));
        }

        let quality_items = ["origin", "uhd", "hd", "sd", "ld", "md"];
        let quality = if quality_items.contains(&self.douyin_quality.as_str()) {
            self.douyin_quality.as_str()
        } else {
            "origin"
        };
        let quality_index = quality_items
            .iter()
            .position(|item| item == &quality)
            .unwrap_or_default();
        let selected_quality = if stream_data.contains_key(quality) {
            quality
        } else {
            quality_items[quality_index + 1..]
                .iter()
                .copied()
                .find(|item| stream_data.contains_key(*item))
                .or_else(|| {
                    quality_items[..quality_index]
                        .iter()
                        .rev()
                        .copied()
                        .find(|item| stream_data.contains_key(*item))
                })
                .ok_or_else(|| LiveError::custom("抖音没有可用清晰度"))?
        };

        let protocol = if self.douyin_protocol == "hls" {
            "hls"
        } else {
            "flv"
        };

        stream_data
            .get(selected_quality)
            .and_then(|quality| quality.pointer(&format!("/main/{protocol}")))
            .and_then(Value::as_str)
            .filter(|url| !url.is_empty())
            .map(|url| url.replace("http://", "https://"))
            .ok_or_else(|| LiveError::custom("抖音可用直播流为空"))
    }

    fn stream_headers(&self) -> HashMap<String, String> {
        HashMap::from([
            ("referer".to_string(), DOUYIN_LIVE_URL.to_string()),
            ("user-agent".to_string(), DOUYIN_USER_AGENT.to_string()),
            ("cookie".to_string(), self.cookie.clone()),
        ])
    }

    fn danmaku_source(&self) -> Option<DanmakuSource> {
        if !self.douyin_danmaku {
            return None;
        }
        Some(DanmakuSource {
            platform: "douyin".to_string(),
            url: self.url.clone(),
            room_id: self.room_id.clone(),
            cookie: (!self.cookie.is_empty()).then_some(self.cookie.clone()),
            raw: false,
            detail: false,
            extra: HashMap::from([
                ("referer".to_string(), DOUYIN_LIVE_URL.to_string()),
                ("user-agent".to_string(), DOUYIN_USER_AGENT.to_string()),
            ]),
            movie_id: None,
            password: None,
        })
    }
}

fn common_params() -> Vec<(String, String)> {
    vec![
        ("app_name".to_string(), "douyin_web".to_string()),
        ("enter_from".to_string(), "web_live".to_string()),
        ("live_id".to_string(), "1".to_string()),
        ("aid".to_string(), "6383".to_string()),
        ("compress".to_string(), "gzip".to_string()),
        ("device_platform".to_string(), "web".to_string()),
        ("browser_language".to_string(), "zh-CN".to_string()),
        ("browser_platform".to_string(), "Win32".to_string()),
        ("browser_name".to_string(), "Mozilla".to_string()),
        (
            "browser_version".to_string(),
            DOUYIN_USER_AGENT
                .split("Chrome/")
                .nth(1)
                .and_then(|part| part.split(' ').next())
                .unwrap_or("0.0.0.0")
                .to_string(),
        ),
    ]
}

fn sign_query(params: &[(String, String)]) -> LiveResult<String> {
    let query = serde_urlencoded::to_string(params)
        .map_err(|err| LiveError::custom(format!("编码抖音请求参数失败: {err}")))?;
    let mut abogus = ABogus::new(Some(DOUYIN_USER_AGENT));
    Ok(abogus.generate_abogus(&query, ""))
}

fn capture(input: &str, pattern: &str) -> Option<String> {
    Regex::new(pattern)
        .ok()?
        .captures(input)
        .map(|captures| captures[1].to_string())
}

async fn fetch_ttwid(client: &Client) -> Option<String> {
    let resp = client
        .post(DOUYIN_UNION_REGISTER_URL)
        .header(USER_AGENT, DOUYIN_USER_AGENT)
        .json(&json!({
            "region": "cn",
            "aid": 6383,
            "needFid": false,
            "service": DOUYIN_BASE_URL,
            "union": true,
            "fid": ""
        }))
        .send()
        .await
        .ok()?;
    resp.headers()
        .get_all("set-cookie")
        .iter()
        .find_map(|value| {
            let cookie = value.to_str().ok()?;
            cookie
                .split(';')
                .next()?
                .strip_prefix("ttwid=")
                .map(ToString::to_string)
        })
}

fn generate_ms_token() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";
    random_from_charset(CHARSET, 184)
}

fn generate_nonce() -> String {
    random_from_charset(b"abcdef0123456789", 21)
}

fn generate_odin_ttid() -> String {
    random_from_charset(b"abcdef0123456789", 160)
}

fn random_from_charset(charset: &[u8], len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| charset[rng.gen_range(0..charset.len())] as char)
        .collect()
}

const BASE_CHARS: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const BASE36_CHARS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
const FIXED_UNDERSCORE_POSITIONS: [usize; 4] = [8, 13, 18, 23];
const FIXED_4_POSITION: usize = 14;
const VARIANT_POSITION: usize = 19;
const UUID_PART_LEN: usize = 36;
const MAX_BASE36_LEN: usize = 13;

fn gen_verify_fp() -> String {
    let mut rng = rand::thread_rng();
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut base36_buf = [0u8; MAX_BASE36_LEN];
    let base36_len = write_base36(milliseconds, &mut base36_buf);
    let mut uuid_buf = [0u8; UUID_PART_LEN];
    let base_len = BASE_CHARS.len();

    for (i, byte) in uuid_buf.iter_mut().enumerate() {
        *byte = if FIXED_UNDERSCORE_POSITIONS.contains(&i) {
            b'_'
        } else if i == FIXED_4_POSITION {
            b'4'
        } else {
            let n = rng.gen_range(0..base_len);
            let char_idx = if i == VARIANT_POSITION {
                (3 & n) | 8
            } else {
                n
            };
            BASE_CHARS[char_idx]
        };
    }

    let mut result = String::with_capacity(7 + base36_len + 1 + UUID_PART_LEN);
    result.push_str("verify_");
    result.push_str(std::str::from_utf8(&base36_buf[MAX_BASE36_LEN - base36_len..]).unwrap_or(""));
    result.push('_');
    result.push_str(std::str::from_utf8(&uuid_buf).unwrap_or(""));
    result
}

fn write_base36(mut num: u64, buf: &mut [u8; MAX_BASE36_LEN]) -> usize {
    if num == 0 {
        buf[MAX_BASE36_LEN - 1] = b'0';
        return 1;
    }
    let mut pos = MAX_BASE36_LEN;
    while num > 0 {
        pos -= 1;
        buf[pos] = BASE36_CHARS[(num % 36) as usize];
        num /= 36;
    }
    MAX_BASE36_LEN - pos
}

struct StringProcessor;

impl StringProcessor {
    fn to_char_str(bytes: &[u8]) -> String {
        bytes.iter().map(|&byte| byte as char).collect()
    }

    fn to_char_array(input: &str) -> Vec<u8> {
        input.bytes().collect()
    }

    fn generate_random_bytes(length: usize) -> String {
        let mut rng = rand::thread_rng();
        let mut result = Vec::new();
        for _ in 0..length {
            let rd = rng.gen_range(0..10000) as u64;
            result.extend([
                (((rd & 255) & 170) | 1) as u8,
                (((rd & 255) & 85) | 2) as u8,
                (((rd >> 8) & 170) | 5) as u8,
                (((rd >> 8) & 85) | 40) as u8,
            ]);
        }
        Self::to_char_str(&result)
    }
}

struct CryptoUtility {
    salt: String,
    base64_alphabet: Vec<Vec<char>>,
    big_array: Vec<u8>,
}

impl CryptoUtility {
    fn new(salt: &str, custom_base64_alphabet: Vec<&str>) -> Self {
        let base64_alphabet = custom_base64_alphabet
            .into_iter()
            .map(|value| value.chars().collect())
            .collect();
        let big_array = vec![
            121, 243, 55, 234, 103, 36, 47, 228, 30, 231, 106, 6, 115, 95, 78, 101, 250, 207, 198,
            50, 139, 227, 220, 105, 97, 143, 34, 28, 194, 215, 18, 100, 159, 160, 43, 8, 169, 217,
            180, 120, 247, 45, 90, 11, 27, 197, 46, 3, 84, 72, 5, 68, 62, 56, 221, 75, 144, 79, 73,
            161, 178, 81, 64, 187, 134, 117, 186, 118, 16, 241, 130, 71, 89, 147, 122, 129, 65, 40,
            88, 150, 110, 219, 199, 255, 181, 254, 48, 4, 195, 248, 208, 32, 116, 167, 69, 201, 17,
            124, 125, 104, 96, 83, 80, 127, 236, 108, 154, 126, 204, 15, 20, 135, 112, 158, 13, 1,
            188, 164, 210, 237, 222, 98, 212, 77, 253, 42, 170, 202, 26, 22, 29, 182, 251, 10, 173,
            152, 58, 138, 54, 141, 185, 33, 157, 31, 252, 132, 233, 235, 102, 196, 191, 223, 240,
            148, 39, 123, 92, 82, 128, 109, 57, 24, 38, 113, 209, 245, 2, 119, 153, 229, 189, 214,
            230, 174, 232, 63, 52, 205, 86, 140, 66, 175, 111, 171, 246, 133, 238, 193, 99, 60, 74,
            91, 225, 51, 76, 37, 145, 211, 166, 151, 213, 206, 0, 200, 244, 176, 218, 44, 184, 172,
            49, 216, 93, 168, 53, 21, 183, 41, 67, 85, 224, 155, 226, 242, 87, 177, 146, 70, 190,
            12, 162, 19, 137, 114, 25, 165, 163, 192, 23, 59, 9, 94, 179, 107, 35, 7, 142, 131,
            239, 203, 149, 136, 61, 249, 14, 156,
        ];
        Self {
            salt: salt.to_string(),
            base64_alphabet,
            big_array,
        }
    }

    fn sm3_to_array(input_data: &[u8]) -> Vec<u8> {
        Sm3Hash::new(input_data).get_hash().to_vec()
    }

    fn params_to_array(&self, param: &str, add_salt: bool) -> Vec<u8> {
        let processed_param = if add_salt {
            format!("{param}{}", self.salt)
        } else {
            param.to_string()
        };
        Self::sm3_to_array(processed_param.as_bytes())
    }

    fn transform_bytes(&mut self, values_list: &[u32]) -> Vec<u32> {
        let mut result_vec = Vec::with_capacity(values_list.len());
        let mut index_b = self.big_array[1] as usize;
        let mut initial_value: u8 = 0;
        let mut value_e: u8 = 0;
        let array_len = self.big_array.len();

        for (index, &char_code) in values_list.iter().enumerate() {
            let sum_initial = if index == 0 {
                initial_value = self.big_array[index_b];
                let sum_val = (index_b as u8).wrapping_add(initial_value);
                self.big_array[1] = initial_value;
                self.big_array[index_b] = index_b as u8;
                sum_val
            } else {
                initial_value.wrapping_add(value_e)
            };

            let sum_initial_idx = (sum_initial as usize) % array_len;
            let value_f = self.big_array[sum_initial_idx];
            result_vec.push(char_code ^ (value_f as u32));

            let next_idx = (index + 2) % array_len;
            value_e = self.big_array[next_idx];
            let new_sum_initial_idx = ((index_b as u8).wrapping_add(value_e) as usize) % array_len;
            initial_value = self.big_array[new_sum_initial_idx];
            self.big_array.swap(new_sum_initial_idx, next_idx);
            index_b = new_sum_initial_idx;
        }
        result_vec
    }

    fn base64_encode(&self, bytes: &[u8], selected_alphabet: usize) -> String {
        let alphabet = &self.base64_alphabet[selected_alphabet];
        let mut output_string = String::with_capacity((bytes.len() * 4).div_ceil(3));
        for chunk in bytes.chunks(3) {
            let b1 = chunk[0];
            let b2 = chunk.get(1).copied().unwrap_or(0);
            let b3 = chunk.get(2).copied().unwrap_or(0);
            let combined = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);
            output_string.push(alphabet[((combined >> 18) & 63) as usize]);
            output_string.push(alphabet[((combined >> 12) & 63) as usize]);
            if chunk.len() > 1 {
                output_string.push(alphabet[((combined >> 6) & 63) as usize]);
            }
            if chunk.len() > 2 {
                output_string.push(alphabet[(combined & 63) as usize]);
            }
        }
        let padding_needed = (4 - output_string.len() % 4) % 4;
        if padding_needed > 0 {
            output_string.push_str(&"=".repeat(padding_needed));
        }
        output_string
    }

    fn abogus_encode(&self, values: &[u32], selected_alphabet: usize) -> String {
        let alphabet = &self.base64_alphabet[selected_alphabet];
        let mut abogus = String::with_capacity((values.len() * 4).div_ceil(3));
        for chunk in values.chunks(3) {
            let v1 = chunk[0];
            let v2 = chunk.get(1).copied().unwrap_or(0);
            let v3 = chunk.get(2).copied().unwrap_or(0);
            let n = (v1 << 16) | (v2 << 8) | v3;
            abogus.push(alphabet[((n & 0xFC0000) >> 18) as usize]);
            abogus.push(alphabet[((n & 0x03F000) >> 12) as usize]);
            if chunk.len() > 1 {
                abogus.push(alphabet[((n & 0x0FC0) >> 6) as usize]);
            }
            if chunk.len() > 2 {
                abogus.push(alphabet[(n & 0x3F) as usize]);
            }
        }
        let padding = (4 - abogus.len() % 4) % 4;
        if padding > 0 {
            abogus.push_str(&"=".repeat(padding));
        }
        abogus
    }

    fn rc4_encrypt(key: &[u8], plaintext: &str) -> Vec<u8> {
        let mut state: [u8; 256] = [0; 256];
        for (i, elem) in state.iter_mut().enumerate() {
            *elem = i as u8;
        }
        let mut j: u8 = 0;
        for i in 0..256 {
            j = j.wrapping_add(state[i]).wrapping_add(key[i % key.len()]);
            state.swap(i, j as usize);
        }
        let mut i: u8 = 0;
        let mut j: u8 = 0;
        let mut ciphertext = Vec::with_capacity(plaintext.len());
        for &char_val in plaintext.as_bytes() {
            i = i.wrapping_add(1);
            j = j.wrapping_add(state[i as usize]);
            state.swap(i as usize, j as usize);
            let k = state[state[i as usize].wrapping_add(state[j as usize]) as usize];
            ciphertext.push(char_val ^ k);
        }
        ciphertext
    }
}

struct BrowserFingerprintGenerator;

impl BrowserFingerprintGenerator {
    fn generate_fingerprint() -> String {
        let mut rng = rand::thread_rng();
        let inner_width = rng.gen_range(1024..=1920);
        let inner_height = rng.gen_range(768..=1080);
        let outer_width = inner_width + rng.gen_range(24..=32);
        let outer_height = inner_height + rng.gen_range(75..=90);
        let screen_y = [0, 30].choose(&mut rng).copied().unwrap_or(0);
        let size_width = rng.gen_range(1024..=1920);
        let size_height = rng.gen_range(768..=1080);
        let avail_width = rng.gen_range(1280..=1920);
        let avail_height = rng.gen_range(800..=1080);
        format!(
            "{inner_width}|{inner_height}|{outer_width}|{outer_height}|0|{screen_y}|0|0|{size_width}|{size_height}|{avail_width}|{avail_height}|{inner_width}|{inner_height}|24|24|Win32",
        )
    }
}

struct ABogus {
    crypto_utility: CryptoUtility,
    user_agent: String,
    browser_fp: String,
    options: Vec<u64>,
    page_id: u64,
    aid: u64,
    ua_key: Vec<u8>,
    sort_index: Vec<u8>,
    sort_index_2: Vec<u8>,
}

impl ABogus {
    fn new(user_agent: Option<&str>) -> Self {
        Self {
            crypto_utility: CryptoUtility::new(
                "cus",
                vec![
                    "Dkdpgh2ZmsQB80/MfvV36XI1R45-WUAlEixNLwoqYTOPuzKFjJnry79HbGcaStCe",
                    "ckdp1h4ZKsUB80/Mfvw36XIgR25+WQAlEi7NLboqYTOPuzmFjJnryx9HVGDaStCe",
                ],
            ),
            user_agent: user_agent.unwrap_or(DOUYIN_USER_AGENT).to_string(),
            browser_fp: BrowserFingerprintGenerator::generate_fingerprint(),
            options: vec![0, 1, 14],
            page_id: 0,
            aid: 6383,
            ua_key: vec![0x00, 0x01, 0x0E],
            sort_index: vec![
                18, 20, 52, 26, 30, 34, 58, 38, 40, 53, 42, 21, 27, 54, 55, 31, 35, 57, 39, 41, 43,
                22, 28, 32, 60, 36, 23, 29, 33, 37, 44, 45, 59, 46, 47, 48, 49, 50, 24, 25, 65, 66,
                70, 71,
            ],
            sort_index_2: vec![
                18, 20, 26, 30, 34, 38, 40, 42, 21, 27, 31, 35, 39, 41, 43, 22, 28, 32, 36, 23, 29,
                33, 37, 44, 45, 46, 47, 48, 49, 50, 24, 25, 52, 53, 54, 55, 57, 58, 59, 60, 65, 66,
                70, 71,
            ],
        }
    }

    fn generate_abogus(&mut self, params: &str, body: &str) -> String {
        let mut ab_dir: HashMap<u8, u64> = HashMap::new();
        ab_dir.insert(8, 3);
        ab_dir.insert(18, 44);
        ab_dir.insert(66, 0);
        ab_dir.insert(69, 0);
        ab_dir.insert(70, 0);
        ab_dir.insert(71, 0);

        let start_encryption = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let params_hash_1 = self.crypto_utility.params_to_array(params, true);
        let array1 = CryptoUtility::sm3_to_array(&params_hash_1);
        let body_hash_1 = self.crypto_utility.params_to_array(body, true);
        let array2 = CryptoUtility::sm3_to_array(&body_hash_1);
        let rc4_ua = CryptoUtility::rc4_encrypt(&self.ua_key, &self.user_agent);
        let ua_b64 = self.crypto_utility.base64_encode(&rc4_ua, 1);
        let array3 = self.crypto_utility.params_to_array(&ua_b64, false);
        let end_encryption = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(start_encryption);

        ab_dir.insert(20, (start_encryption >> 24) & 255);
        ab_dir.insert(21, (start_encryption >> 16) & 255);
        ab_dir.insert(22, (start_encryption >> 8) & 255);
        ab_dir.insert(23, start_encryption & 255);
        ab_dir.insert(24, start_encryption / 0x100000000);
        ab_dir.insert(25, start_encryption / 0x10000000000);
        ab_dir.insert(26, (self.options[0] >> 24) & 255);
        ab_dir.insert(27, (self.options[0] >> 16) & 255);
        ab_dir.insert(28, (self.options[0] >> 8) & 255);
        ab_dir.insert(29, self.options[0] & 255);
        ab_dir.insert(30, (self.options[1] / 256) & 255);
        ab_dir.insert(31, self.options[1] % 256);
        ab_dir.insert(32, (self.options[1] >> 24) & 255);
        ab_dir.insert(33, (self.options[1] >> 16) & 255);
        ab_dir.insert(34, (self.options[2] >> 24) & 255);
        ab_dir.insert(35, (self.options[2] >> 16) & 255);
        ab_dir.insert(36, (self.options[2] >> 8) & 255);
        ab_dir.insert(37, self.options[2] & 255);
        ab_dir.insert(38, array1[21] as u64);
        ab_dir.insert(39, array1[22] as u64);
        ab_dir.insert(40, array2[21] as u64);
        ab_dir.insert(41, array2[22] as u64);
        ab_dir.insert(42, array3[23] as u64);
        ab_dir.insert(43, array3[24] as u64);
        ab_dir.insert(44, (end_encryption >> 24) & 255);
        ab_dir.insert(45, (end_encryption >> 16) & 255);
        ab_dir.insert(46, (end_encryption >> 8) & 255);
        ab_dir.insert(47, end_encryption & 255);
        ab_dir.insert(48, *ab_dir.get(&8).unwrap_or(&0));
        ab_dir.insert(49, end_encryption / 0x100000000);
        ab_dir.insert(50, end_encryption / 0x10000000000);
        ab_dir.insert(51, (self.page_id >> 24) & 255);
        ab_dir.insert(52, (self.page_id >> 16) & 255);
        ab_dir.insert(53, (self.page_id >> 8) & 255);
        ab_dir.insert(54, self.page_id & 255);
        ab_dir.insert(55, self.page_id);
        ab_dir.insert(56, self.aid);
        ab_dir.insert(57, self.aid & 255);
        ab_dir.insert(58, (self.aid >> 8) & 255);
        ab_dir.insert(59, (self.aid >> 16) & 255);
        ab_dir.insert(60, (self.aid >> 24) & 255);
        ab_dir.insert(64, self.browser_fp.len() as u64);
        ab_dir.insert(65, self.browser_fp.len() as u64);

        let mut sorted_values: Vec<u32> = self
            .sort_index
            .iter()
            .map(|&i| *ab_dir.get(&i).unwrap_or(&0) as u32)
            .collect();
        let fp_array = StringProcessor::to_char_array(&self.browser_fp);
        let mut ab_xor: u32 = 0;
        for (index, &key) in self.sort_index_2.iter().enumerate() {
            let val = *ab_dir.get(&key).unwrap_or(&0) as u32;
            if index == 0 {
                ab_xor = val;
            } else {
                ab_xor ^= val;
            }
        }
        sorted_values.extend(fp_array.iter().map(|&b| b as u32));
        sorted_values.push(ab_xor);

        let transformed_values = self.crypto_utility.transform_bytes(&sorted_values);
        let random_prefix: Vec<u32> = StringProcessor::generate_random_bytes(3)
            .chars()
            .map(|c| c as u32)
            .collect();
        let final_values: Vec<u32> = random_prefix
            .into_iter()
            .chain(transformed_values)
            .collect();
        let abogus = self.crypto_utility.abogus_encode(&final_values, 0);
        format!("{params}&a_bogus={abogus}")
    }
}
