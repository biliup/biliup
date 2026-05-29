use crate::server::common::util::{danmaku_filename_template, media_ext_from_url};
use crate::server::core::downloader::{DanmakuClient, RustDanmakuClient};
use crate::server::core::plugin::{DownloadBase, DownloadPlugin, StreamInfoExt, StreamStatus};
use crate::server::errors::AppError;
use crate::server::infrastructure::context::PluginContext;
use crate::server::infrastructure::models::StreamerInfo;
use async_trait::async_trait;
use chrono::Utc;
use danmaku_client::{PlatformContext, RecorderConfig};
use error_stack::{Report, ResultExt, bail};
use md5::{Digest, Md5};
use regex::Regex;
use reqwest::Client;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

const TWITCASTING_REFERER: &str = "https://twitcasting.tv/";
const TWITCASTING_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct Twitcasting {
    re: Regex,
}

impl Default for Twitcasting {
    fn default() -> Self {
        Self::new()
    }
}

impl Twitcasting {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"https?://twitcasting\.tv/([^/]+)").unwrap(),
        }
    }
}

impl DownloadPlugin for Twitcasting {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    fn create_downloader(&self, ctx: &mut PluginContext) -> Box<dyn DownloadBase> {
        let config = ctx.config();
        Box::new(TwitcastingDownloader::new(
            ctx.client(),
            ctx.live_streamer().url.clone(),
            ctx.live_streamer().remark.clone(),
            config.twitcasting_password.clone(),
            config.twitcasting_quality.clone(),
            config
                .user
                .as_ref()
                .and_then(|user| user.twitcasting_cookie.clone()),
            config.twitcasting_danmaku.unwrap_or(false),
            ctx.live_streamer()
                .filename_prefix
                .clone()
                .or(config.filename_prefix.clone()),
        ))
    }

    fn name(&self) -> &str {
        "TwitCasting"
    }
}

struct TwitcastingDownloader {
    client: Client,
    url: String,
    name: String,
    password: Option<String>,
    quality: Option<String>,
    cookie: Option<String>,
    twitcasting_danmaku: bool,
    filename_prefix: Option<String>,
    movie_id: Option<String>,
}

impl TwitcastingDownloader {
    fn new(
        client: Client,
        url: String,
        name: String,
        password: Option<String>,
        quality: Option<String>,
        cookie: Option<String>,
        twitcasting_danmaku: bool,
        filename_prefix: Option<String>,
    ) -> Self {
        Self {
            client,
            url,
            name,
            password,
            quality,
            cookie,
            twitcasting_danmaku,
            filename_prefix,
            movie_id: None,
        }
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(REFERER, HeaderValue::from_static(TWITCASTING_REFERER));
        headers.insert(USER_AGENT, HeaderValue::from_static(TWITCASTING_USER_AGENT));
        let mut cookies = Vec::new();
        if let Some(cookie) = &self.cookie {
            cookies.extend(
                cookie
                    .split(';')
                    .map(str::trim)
                    .filter(|cookie| !cookie.is_empty())
                    .map(ToString::to_string),
            );
        }
        if let Some(password) = &self.password {
            cookies.push(format!("wpass={}", md5_hex(password)));
        }
        if !cookies.is_empty()
            && let Ok(cookie) = HeaderValue::from_str(&cookies.join("; "))
        {
            headers.insert(COOKIE, cookie);
        }
        headers
    }

    async fn get_room_page(
        &self,
        headers: HeaderMap,
    ) -> Result<TwitcastingRoomPage, Report<AppError>> {
        let text = self
            .client
            .get(&self.url)
            .headers(headers)
            .send()
            .await
            .change_context(AppError::Custom(
                "获取 TwitCasting 直播间页面失败".to_string(),
            ))?
            .text()
            .await
            .change_context(AppError::Custom(
                "读取 TwitCasting 直播间页面失败".to_string(),
            ))?;

        if text.contains("Enter the secret word to access") {
            bail!(AppError::Custom("TwitCasting 直播间需要密码".to_string()))
        }

        Ok(TwitcastingRoomPage {
            title: capture(&text, r#"<meta name="twitter:title" content="([^"]*)""#)
                .unwrap_or_default(),
            uploader_id: capture(&text, r#"<meta name="twitter:creator" content="([^"]*)""#)
                .ok_or_else(|| {
                    Report::new(AppError::Custom("TwitCasting 主播 ID 为空".to_string()))
                })?,
        })
    }

    async fn get_stream_info(
        &self,
        uploader_id: &str,
        headers: HeaderMap,
    ) -> Result<Option<TwitcastingStreamInfo>, Report<AppError>> {
        let resp = self
            .client
            .get("https://twitcasting.tv/streamserver.php")
            .query(&[
                ("target", uploader_id),
                ("mode", "client"),
                ("player", "pc_web"),
            ])
            .headers(headers)
            .send()
            .await
            .change_context(AppError::Custom(
                "获取 TwitCasting 直播流信息失败".to_string(),
            ))?;

        if !resp.status().is_success() {
            bail!(AppError::Custom(format!(
                "获取 TwitCasting 直播流信息错误: {}",
                resp.status()
            )))
        }

        let stream_info: TwitcastingStreamInfo = resp.json().await.change_context(
            AppError::Custom("解析 TwitCasting 直播流信息失败".to_string()),
        )?;
        if !stream_info.movie.live {
            return Ok(None);
        }
        Ok(Some(stream_info))
    }

    fn select_stream_url(&self, streams: &HashMap<String, String>) -> Option<String> {
        let quality_levels = ["high", "medium", "low"];
        if let Some(quality) = self.quality.as_deref()
            && let Some(start) = quality_levels.iter().position(|level| level == &quality)
        {
            for quality in &quality_levels[start..] {
                if let Some(url) = streams.get(*quality) {
                    return Some(url.clone());
                }
            }
        }

        for quality in quality_levels {
            if let Some(url) = streams.get(quality) {
                return Some(url.clone());
            }
        }
        streams.values().next().cloned()
    }
}

#[async_trait]
impl DownloadBase for TwitcastingDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        let headers = self.headers();
        let room_page = self.get_room_page(headers.clone()).await?;
        let Some(stream_info) = self
            .get_stream_info(&room_page.uploader_id, headers.clone())
            .await?
        else {
            return Ok(StreamStatus::Offline);
        };

        let Some(raw_stream_url) = self.select_stream_url(&stream_info.tc_hls.streams) else {
            return Ok(StreamStatus::Offline);
        };
        self.movie_id = Some(stream_info.movie.id);
        Ok(StreamStatus::Live {
            stream_info: Box::new(StreamInfoExt {
                streamer_info: StreamerInfo {
                    id: -1,
                    name: self.name.clone(),
                    url: self.url.clone(),
                    title: room_page.title,
                    date: Utc::now(),
                    live_cover_path: String::new(),
                },
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "m3u8".to_string()),
                raw_stream_url,
                platform: "twitcasting".to_string(),
                stream_headers: HashMap::from([
                    ("referer".to_string(), TWITCASTING_REFERER.to_string()),
                    ("user-agent".to_string(), TWITCASTING_USER_AGENT.to_string()),
                ]),
            }),
        })
    }

    fn danmaku_init(&self) -> Option<Arc<dyn DanmakuClient + Send + Sync>> {
        if !self.twitcasting_danmaku {
            return None;
        }

        let mut context = PlatformContext::new();
        context.movie_id = Some(self.movie_id.clone()?);
        context.password = self.password.clone();

        let config = RecorderConfig::new(
            self.url.clone(),
            PathBuf::from(danmaku_filename_template(
                self.filename_prefix.as_deref(),
                &self.name,
            )),
        )
        .with_context(context);

        Some(Arc::new(RustDanmakuClient::new(config)) as Arc<dyn DanmakuClient + Send + Sync>)
    }
}

struct TwitcastingRoomPage {
    title: String,
    uploader_id: String,
}

#[derive(Deserialize)]
struct TwitcastingStreamInfo {
    movie: TwitcastingMovie,
    #[serde(rename = "tc-hls")]
    tc_hls: TwitcastingHls,
}

#[derive(Deserialize)]
struct TwitcastingMovie {
    live: bool,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    id: String,
}

#[derive(Deserialize)]
struct TwitcastingHls {
    streams: HashMap<String, String>,
}

fn capture(input: &str, pattern: &str) -> Option<String> {
    Regex::new(pattern)
        .ok()?
        .captures(input)
        .map(|captures| html_unescape(&captures[1]))
}

fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(match value {
        serde_json::Value::String(value) => value,
        serde_json::Value::Number(value) => value.to_string(),
        _ => String::new(),
    })
}

fn md5_hex(input: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn html_unescape(input: &str) -> String {
    input
        .replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&#x22;", "\"")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}
