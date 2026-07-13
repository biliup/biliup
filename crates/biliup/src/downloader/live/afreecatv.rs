use super::{
    DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus, LiveStream,
    media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const AFREECATV_CHANNEL_API_URL: &str = "https://live.afreecatv.com/afreeca/player_live_api.php";
const AFREECATV_LOGIN_URL: &str = "https://login.afreecatv.com/app/LoginAction.php";
const AFREECATV_REFERER: &str = "https://play.afreecatv.com/";
const AFREECATV_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const AFREECATV_QUALITY: &str = "original";
const AFREECATV_LOGIN_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

struct CachedLogin {
    cookie: String,
    fetched_at: Instant,
    username: String,
    password: String,
}

pub struct AfreecaTV {
    re: Regex,
    login_cache: Mutex<Option<CachedLogin>>,
}

impl Default for AfreecaTV {
    fn default() -> Self {
        Self::new()
    }
}

impl AfreecaTV {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"https?://(.*?)\.afreecatv\.com/(\w+)(?:/\d+)?").unwrap(),
            login_cache: Mutex::new(None),
        }
    }

    /// 复用 7 天内的登录 cookie，避免每次 check_stream 都请求 LoginAction.php。
    /// 凭据与缓存不一致时视为缓存失效；登录失败不缓存。
    async fn login_cookie(&self, live: &AfreecaTVLive) -> LiveResult<Option<String>> {
        let (Some(username), Some(password)) = (&live.username, &live.password) else {
            return Ok(None);
        };
        if username.is_empty() || password.is_empty() {
            return Ok(None);
        }

        let mut cache = self.login_cache.lock().await;
        if let Some(cached) = cache.as_ref()
            && cached.fetched_at.elapsed() < AFREECATV_LOGIN_TTL
            && cached.username == *username
            && cached.password == *password
        {
            return Ok(Some(cached.cookie.clone()));
        }

        let cookie = live.login().await?;
        *cache = cookie.as_ref().map(|cookie| CachedLogin {
            cookie: cookie.clone(),
            fetched_at: Instant::now(),
            username: username.clone(),
            password: password.clone(),
        });
        Ok(cookie)
    }
}

#[async_trait]
impl LivePlugin for AfreecaTV {
    fn name(&self) -> &'static str {
        "AfreecaTV"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        let live = AfreecaTVLive::new(request);
        let cookie = self.login_cookie(&live).await?;
        live.check_stream(cookie.as_deref()).await
    }
}

struct AfreecaTVLive {
    client: reqwest::Client,
    url: String,
    name: String,
    username: Option<String>,
    password: Option<String>,
}

impl AfreecaTVLive {
    fn new(request: LiveRequest) -> Self {
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
            username: request.credentials.afreecatv_username,
            password: request.credentials.afreecatv_password,
        }
    }

    async fn check_stream(&self, cookie: Option<&str>) -> LiveResult<LiveStatus> {
        let username = self.room_username()?;
        let Some(channel) = self.channel_info(&username, cookie).await? else {
            return Ok(LiveStatus::Offline);
        };
        let aid = self.aid(&username, &channel.bno, cookie).await?;
        let raw_stream_url = self.stream_url(&channel, &aid, cookie).await?;

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: channel.title,
                date: Utc::now(),
                live_cover_url: String::new(),
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "m3u8".to_string()),
                raw_stream_url,
                platform: "afreecatv".to_string(),
                stream_headers: HashMap::from([
                    ("referer".to_string(), AFREECATV_REFERER.to_string()),
                    ("user-agent".to_string(), AFREECATV_USER_AGENT.to_string()),
                ]),
                danmaku: None,
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }

    fn headers(&self, cookie: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(AFREECATV_USER_AGENT));
        headers.insert(REFERER, HeaderValue::from_static(AFREECATV_REFERER));
        if let Some(cookie) = cookie
            && let Ok(cookie) = HeaderValue::from_str(cookie)
        {
            headers.insert(COOKIE, cookie);
        }
        headers
    }

    fn room_username(&self) -> LiveResult<String> {
        Regex::new(r"https?://play\.afreecatv\.com/(\w+)(?:/\d+)?")
            .unwrap()
            .captures(&self.url)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| LiveError::custom("AfreecaTV 直播间地址错误"))
    }

    async fn login(&self) -> LiveResult<Option<String>> {
        let (Some(username), Some(password)) = (&self.username, &self.password) else {
            return Ok(None);
        };
        if username.is_empty() || password.is_empty() {
            return Ok(None);
        }

        let response = self
            .client
            .post(AFREECATV_LOGIN_URL)
            .header(USER_AGENT, AFREECATV_USER_AGENT)
            .form(&[
                ("szUid", username.as_str()),
                ("szPassword", password.as_str()),
                ("szWork", "login"),
                ("szType", "json"),
                ("isSaveId", "true"),
                ("isSavePw", "true"),
                ("isSaveJoin", "true"),
                ("isLoginRetain", "Y"),
            ])
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("AfreecaTV 登录失败: {err}")))?;

        let cookie = response
            .headers()
            .get_all("set-cookie")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .filter_map(|cookie| cookie.split(';').next())
            .filter(|cookie| {
                ["RDB=", "PdboxBbs=", "PdboxTicket=", "PdboxSaveTicket="]
                    .iter()
                    .any(|prefix| cookie.starts_with(prefix))
            })
            .collect::<Vec<_>>()
            .join(";");
        let body: LoginResponse = response
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析 AfreecaTV 登录响应失败: {err}")))?;

        Ok((body.result == 1 && !cookie.is_empty()).then_some(cookie))
    }

    async fn channel_info(
        &self,
        username: &str,
        cookie: Option<&str>,
    ) -> LiveResult<Option<Channel>> {
        let response: ChannelResponse = self
            .client
            .post(AFREECATV_CHANNEL_API_URL)
            .headers(self.headers(cookie))
            .form(&[
                ("bid", username),
                ("bno", ""),
                ("type", "live"),
                ("pwd", ""),
                ("player_type", "html5"),
                ("stream_type", "common"),
                ("quality", AFREECATV_QUALITY),
                ("mode", "landing"),
                ("from_api", "0"),
            ])
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取 AfreecaTV 直播间信息失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析 AfreecaTV 直播间信息失败: {err}")))?;

        match response.channel.result {
            1 => Ok(Some(response.channel)),
            -6 => Ok(None),
            _ => Ok(None),
        }
    }

    async fn aid(&self, username: &str, bno: &str, cookie: Option<&str>) -> LiveResult<String> {
        let response: ChannelResponse = self
            .client
            .post(AFREECATV_CHANNEL_API_URL)
            .headers(self.headers(cookie))
            .form(&[
                ("bid", username),
                ("bno", bno),
                ("type", "aid"),
                ("pwd", ""),
                ("player_type", "html5"),
                ("stream_type", "common"),
                ("quality", AFREECATV_QUALITY),
                ("mode", "landing"),
                ("from_api", "0"),
            ])
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取 AfreecaTV AID 失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析 AfreecaTV AID 失败: {err}")))?;

        response
            .channel
            .aid
            .filter(|aid| !aid.is_empty())
            .ok_or_else(|| LiveError::custom("AfreecaTV AID 为空"))
    }

    async fn stream_url(
        &self,
        channel: &Channel,
        aid: &str,
        cookie: Option<&str>,
    ) -> LiveResult<String> {
        let response: ViewResponse = self
            .client
            .get(format!("{}/broad_stream_assign.html", channel.rmd))
            .headers(self.headers(cookie))
            .query(&[
                ("return_type", channel.cdn.as_str()),
                (
                    "broad_key",
                    format!("{}-common-{AFREECATV_QUALITY}-hls", channel.bno).as_str(),
                ),
            ])
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取 AfreecaTV 播放地址失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析 AfreecaTV 播放地址失败: {err}")))?;

        if response.view_url.is_empty() {
            return Err(LiveError::custom("AfreecaTV 播放地址为空"));
        }
        Ok(format!("{}?aid={aid}", response.view_url))
    }
}

#[derive(Deserialize)]
struct LoginResponse {
    #[serde(rename = "RESULT")]
    result: i32,
}

#[derive(Deserialize)]
struct ChannelResponse {
    #[serde(rename = "CHANNEL")]
    channel: Channel,
}

#[derive(Deserialize)]
struct Channel {
    #[serde(rename = "RESULT")]
    result: i32,
    #[serde(default, rename = "TITLE")]
    title: String,
    #[serde(default, rename = "BNO")]
    bno: String,
    #[serde(default, rename = "RMD")]
    rmd: String,
    #[serde(default, rename = "CDN")]
    cdn: String,
    #[serde(default, rename = "AID")]
    aid: Option<String>,
}

#[derive(Deserialize)]
struct ViewResponse {
    view_url: String,
}
