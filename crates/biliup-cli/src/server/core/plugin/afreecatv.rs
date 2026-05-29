use crate::server::common::util::media_ext_from_url;
use crate::server::core::plugin::{DownloadBase, DownloadPlugin, StreamInfoExt, StreamStatus};
use crate::server::errors::AppError;
use crate::server::infrastructure::context::PluginContext;
use crate::server::infrastructure::models::StreamerInfo;
use async_trait::async_trait;
use chrono::Utc;
use error_stack::{Report, ResultExt, bail};
use regex::Regex;
use reqwest::Client;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use serde::Deserialize;
use std::collections::HashMap;

const AFREECATV_CHANNEL_API_URL: &str = "https://live.afreecatv.com/afreeca/player_live_api.php";
const AFREECATV_LOGIN_URL: &str = "https://login.afreecatv.com/app/LoginAction.php";
const AFREECATV_REFERER: &str = "https://play.afreecatv.com/";
const AFREECATV_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const AFREECATV_QUALITY: &str = "original";

pub struct AfreecaTV {
    re: Regex,
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
        }
    }
}

impl DownloadPlugin for AfreecaTV {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    fn create_downloader(&self, ctx: &mut PluginContext) -> Box<dyn DownloadBase> {
        let user = ctx.config().user.clone().unwrap_or_default();
        Box::new(AfreecaTVDownloader::new(
            ctx.client(),
            ctx.live_streamer().url.clone(),
            ctx.live_streamer().remark.clone(),
            user.afreecatv_username,
            user.afreecatv_password,
        ))
    }

    fn name(&self) -> &str {
        "AfreecaTV"
    }
}

struct AfreecaTVDownloader {
    client: Client,
    url: String,
    name: String,
    username: Option<String>,
    password: Option<String>,
    cookie: Option<String>,
}

impl AfreecaTVDownloader {
    fn new(
        client: Client,
        url: String,
        name: String,
        username: Option<String>,
        password: Option<String>,
    ) -> Self {
        Self {
            client,
            url,
            name,
            username,
            password,
            cookie: None,
        }
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(AFREECATV_USER_AGENT));
        headers.insert(REFERER, HeaderValue::from_static(AFREECATV_REFERER));
        if let Some(cookie) = &self.cookie
            && let Ok(cookie) = HeaderValue::from_str(cookie)
        {
            headers.insert(COOKIE, cookie);
        }
        headers
    }

    fn room_username(&self) -> Result<String, Report<AppError>> {
        Regex::new(r"https?://play\.afreecatv\.com/(\w+)(?:/\d+)?")
            .unwrap()
            .captures(&self.url)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| Report::new(AppError::Custom("AfreecaTV 直播间地址错误".to_string())))
    }

    async fn login(&mut self) -> Result<(), Report<AppError>> {
        let (Some(username), Some(password)) = (&self.username, &self.password) else {
            return Ok(());
        };
        if username.is_empty() || password.is_empty() {
            return Ok(());
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
            .change_context(AppError::Custom("AfreecaTV 登录失败".to_string()))?;

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
            .change_context(AppError::Custom("解析 AfreecaTV 登录响应失败".to_string()))?;

        if body.result == 1 && !cookie.is_empty() {
            self.cookie = Some(cookie);
        }
        Ok(())
    }

    async fn channel_info(&self, username: &str) -> Result<Option<Channel>, Report<AppError>> {
        let response: ChannelResponse = self
            .client
            .post(AFREECATV_CHANNEL_API_URL)
            .headers(self.headers())
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
            .change_context(AppError::Custom(
                "获取 AfreecaTV 直播间信息失败".to_string(),
            ))?
            .json()
            .await
            .change_context(AppError::Custom(
                "解析 AfreecaTV 直播间信息失败".to_string(),
            ))?;

        match response.channel.result {
            1 => Ok(Some(response.channel)),
            -6 => Ok(None),
            _ => Ok(None),
        }
    }

    async fn aid(&self, username: &str, bno: &str) -> Result<String, Report<AppError>> {
        let response: ChannelResponse = self
            .client
            .post(AFREECATV_CHANNEL_API_URL)
            .headers(self.headers())
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
            .change_context(AppError::Custom("获取 AfreecaTV AID 失败".to_string()))?
            .json()
            .await
            .change_context(AppError::Custom("解析 AfreecaTV AID 失败".to_string()))?;

        response
            .channel
            .aid
            .filter(|aid| !aid.is_empty())
            .ok_or_else(|| Report::new(AppError::Custom("AfreecaTV AID 为空".to_string())))
    }

    async fn stream_url(&self, channel: &Channel, aid: &str) -> Result<String, Report<AppError>> {
        let response: ViewResponse = self
            .client
            .get(format!("{}/broad_stream_assign.html", channel.rmd))
            .headers(self.headers())
            .query(&[
                ("return_type", channel.cdn.as_str()),
                (
                    "broad_key",
                    format!("{}-common-{AFREECATV_QUALITY}-hls", channel.bno).as_str(),
                ),
            ])
            .send()
            .await
            .change_context(AppError::Custom("获取 AfreecaTV 播放地址失败".to_string()))?
            .json()
            .await
            .change_context(AppError::Custom("解析 AfreecaTV 播放地址失败".to_string()))?;

        if response.view_url.is_empty() {
            bail!(AppError::Custom("AfreecaTV 播放地址为空".to_string()))
        }
        Ok(format!("{}?aid={aid}", response.view_url))
    }
}

#[async_trait]
impl DownloadBase for AfreecaTVDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        self.login().await?;
        let username = self.room_username()?;
        let Some(channel) = self.channel_info(&username).await? else {
            return Ok(StreamStatus::Offline);
        };
        let aid = self.aid(&username, &channel.bno).await?;
        let raw_stream_url = self.stream_url(&channel, &aid).await?;

        Ok(StreamStatus::Live {
            stream_info: Box::new(StreamInfoExt {
                streamer_info: StreamerInfo {
                    id: -1,
                    name: self.name.clone(),
                    url: self.url.clone(),
                    title: channel.title,
                    date: Utc::now(),
                    live_cover_path: String::new(),
                },
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "m3u8".to_string()),
                raw_stream_url,
                platform: "afreecatv".to_string(),
                stream_headers: HashMap::from([
                    ("referer".to_string(), AFREECATV_REFERER.to_string()),
                    ("user-agent".to_string(), AFREECATV_USER_AGENT.to_string()),
                ]),
            }),
        })
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
