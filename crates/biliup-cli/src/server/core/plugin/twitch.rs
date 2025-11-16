use crate::server::common::util::media_ext_from_url;
use crate::server::core::downloader::ffmpeg_downloader::FfmpegDownloader;
use crate::server::core::downloader::stream_gears::StreamGears;
use crate::server::core::downloader::streamlink::{Platform, Streamlink, StreamlinkDownloader};
use crate::server::core::downloader::{DanmakuClient, DownloaderRuntime, DownloaderType};
use crate::server::core::plugin::{DownloadBase, DownloadPlugin, StreamInfoExt, StreamStatus};
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::Context;
use crate::server::infrastructure::models::StreamerInfo;
use async_trait::async_trait;
use chrono::Utc;
use error_stack::{IntoReport, Report, ResultExt};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use tokio::time::Duration;
use tracing::{error, warn};

// 常量定义
const CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";

pub struct Twitch {
    re: Regex,
}

impl Twitch {
    fn new() -> Twitch {
        Twitch {
            re: Regex::new(r"https?://(?:(?:www|go|m)\.)?twitch\.tv/(?P<id>[0-9_a-zA-Z]+)")
                .unwrap(),
        }
    }
}

#[async_trait]
impl DownloadBase for TwitchDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        self.acheck_stream().await
    }

    fn downloader(&self, downloader_type: DownloaderType) -> DownloaderRuntime {
        match downloader_type {
            DownloaderType::Ffmpeg => DownloaderRuntime::Ffmpeg(FfmpegDownloader::new(
                Vec::new(),
                DownloaderType::FfmpegExternal,
            )),
            DownloaderType::Streamlink => {
                let result = StreamlinkDownloader::new(
                    self.url.clone(),
                    Platform::Twitch {
                        disable_ads: self.twitch_disable_ads,
                    },
                );
                DownloaderRuntime::StreamLink(Streamlink::new(result))
            }
            _ => DownloaderRuntime::StreamGears(StreamGears::new(None)),
            // ...
        }
    }
}

impl DownloadPlugin for Twitch {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    fn create_downloader(&self, ctx: &mut Context) -> Box<dyn DownloadBase> {
        Box::new(TwitchDownloader::new(
            &ctx.worker.live_streamer.remark,
            ctx.worker.live_streamer.url.clone(),
            self.re.clone(),
        ))
    }

    fn name(&self) -> &str {
        "Twitch"
    }
}
// GraphQL 响应结构
#[derive(Debug, Deserialize)]
struct GqlResponse {
    data: Option<GqlData>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GqlData {
    user: Option<UserData>,
}

#[derive(Debug, Deserialize)]
struct UserData {
    stream: Option<StreamData>,
}

#[derive(Debug, Deserialize)]
struct StreamData {
    #[serde(rename = "type")]
    stream_type: Option<String>,
    id: Option<String>,
    title: Option<String>,
    #[serde(rename = "previewImageURL")]
    preview_image_url: Option<String>,
    #[serde(rename = "playbackAccessToken")]
    playback_access_token: Option<PlaybackAccessToken>,
}

#[derive(Debug, Deserialize)]
struct PlaybackAccessToken {
    signature: String,
    value: String,
}

// Twitch 下载器
pub struct TwitchDownloader {
    url: String,
    re: Regex,
    twitch_danmaku: bool,
    twitch_disable_ads: bool,
    danmaku: Option<Arc<dyn DanmakuClient + Send + Sync>>,
    name: String,
}

impl TwitchDownloader {
    pub fn new(name: &str, url: String, re: Regex) -> Self {
        Self {
            url,
            re,
            twitch_danmaku: false,
            twitch_disable_ads: true,
            danmaku: None,
            name: name.to_string(),
        }
    }

    pub async fn acheck_stream(&self) -> AppResult<StreamStatus> {
        let channel_name = self
            .re
            .captures(&self.url)
            .and_then(|caps| caps.name("id"))
            .map(|m| m.as_str().to_lowercase())
            .ok_or_else(|| AppError::Custom("Invalid URL".to_string()))?;

        let query = r#"
            query query($channel_name:String!) {
                user(login: $channel_name){
                    stream {
                        id
                        title
                        type
                        previewImageURL(width: 0,height: 0)
                        playbackAccessToken(
                            params: {
                                platform: "web",
                                playerBackend: "mediaplayer",
                                playerType: "site"
                            }
                        ) {
                            signature
                            value
                        }
                    }
                }
            }
        "#;

        let ops = json!({
            "query": query,
            "variables": { "channel_name": channel_name }
        });

        let gql: GqlResponse = TwitchUtils::post_gql(ops).await?;

        let user = gql
            .data
            .and_then(|d| d.user)
            .ok_or_else(|| AppError::Custom("获取错误".to_string()))?;

        let stream = match user.stream {
            Some(s) if s.stream_type.as_deref() == Some("live") => s,
            _ => return Ok(StreamStatus::Offline),
        };

        let room_title = stream.title.unwrap_or_default();
        let live_cover_url = stream.preview_image_url.unwrap_or_default();

        let token = stream
            .playback_access_token
            .ok_or_else(|| AppError::Custom("No playback access token".to_string()))?;

        let query_params = [
            ("player", "twitchweb".to_string()),
            ("p", rand::random::<u32>().to_string()),
            ("allow_source", "true".to_string()),
            ("allow_audio_only", "true".to_string()),
            ("allow_spectre", "false".to_string()),
            ("fast_bread", "true".to_string()),
            ("sig", token.signature),
            ("token", token.value),
        ];

        let query_string =
            serde_urlencoded::to_string(&query_params).change_context(AppError::Unknown)?;
        let raw_stream_url = format!(
            "https://usher.ttvnw.net/api/channel/hls/{}.m3u8?{}",
            channel_name, query_string
        );

        Ok(StreamStatus::Live {
            stream_info: Box::new(StreamInfoExt {
                streamer_info: StreamerInfo {
                    id: -1,
                    name: self.name.clone(),
                    url: self.url.clone(),
                    title: room_title,
                    date: Utc::now(),
                    live_cover_path: live_cover_url,
                },
                suffix: media_ext_from_url(&raw_stream_url).unwrap(),
                raw_stream_url,
                platform: "twitch".to_string(),
                stream_headers: HashMap::new(),
            }),
        })
    }

    pub async fn abatch_check(&self, check_urls: Vec<String>) -> AppResult<Vec<String>> {
        let mut ops = Vec::new();

        for url in &check_urls {
            let channel_name = self
                .re
                .captures(url)
                .and_then(|caps| caps.name("id"))
                .map(|m| m.as_str().to_lowercase())
                .ok_or_else(|| AppError::Custom("Invalid URL".to_string()))?;

            ops.push(json!({
                "query": r#"
                    query query($login:String!) {
                        user(login: $login){
                            stream {
                              type
                            }
                        }
                    }
                "#,
                "variables": { "login": channel_name }
            }));
        }

        let gql_list: Vec<GqlResponse> = TwitchUtils::post_gql_batch(ops).await?;
        let mut live_urls = Vec::new();

        for (index, gql) in gql_list.iter().enumerate() {
            if let Some(data) = &gql.data
                && let Some(user) = &data.user
                && let Some(stream) = &user.stream
                && stream.stream_type.as_deref() == Some("live")
            {
                live_urls.push(check_urls[index].clone());
            }
        }

        Ok(live_urls)
    }

    pub fn danmaku_init(&mut self) {
        if self.twitch_danmaku {
            todo!()
        }
    }

    fn gen_download_filename(&self) -> String {
        todo!()
    }
}

// TwitchUtils 工具类
pub struct TwitchUtils {
    invalid_auth_token: Arc<RwLock<Option<String>>>,
}

impl TwitchUtils {
    pub fn get_auth_token() -> Option<String> {
        // Config::get_nested("user", "twitch_cookie")
        None
    }

    pub async fn invalid_auth_token() {
        warn!("Twitch Cookie已失效请及时更换，后续操作将忽略Twitch Cookie");
    }

    pub async fn post_gql<T: Serialize>(ops: T) -> AppResult<GqlResponse> {
        // 最多重试一次（在 token 失效的情况下）
        for retry in 0..2 {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "Content-Type",
                "text/plain;charset=UTF-8"
                    .parse()
                    .change_context(AppError::Unknown)?,
            );
            headers.insert(
                "Client-ID",
                CLIENT_ID.parse().change_context(AppError::Unknown)?,
            );

            // 第二次重试时不使用 auth_token（因为已经失效）
            if retry == 0
                && let Some(auth_token) = Self::get_auth_token()
            {
                headers.insert(
                    "Authorization",
                    format!("OAuth {}", auth_token)
                        .parse()
                        .change_context(AppError::Unknown)?,
                );
            }

            let client = Client::new();
            let resp = client
                .post("https://gql.twitch.tv/gql")
                .headers(headers)
                .json(&ops)
                .timeout(Duration::from_secs(15))
                .send()
                .await
                .change_context(AppError::Unknown)?;

            resp.error_for_status_ref()
                .change_context(AppError::Unknown)?;
            let gql: GqlResponse = resp.json().await.change_context(AppError::Unknown)?;

            if gql.error.as_deref() == Some("Unauthorized") {
                Self::invalid_auth_token().await;
                continue; // 重试
            }
            return Ok(gql);
        }

        Err(AppError::Custom("Failed to authenticate".to_string()).into_report())
    }

    pub async fn post_gql_batch(ops: Vec<Value>) -> AppResult<Vec<GqlResponse>> {
        const LIMIT: usize = 30;
        let mut all_data = Vec::new();

        for chunk in ops.chunks(LIMIT) {
            match Self::post_gql_chunk(chunk).await {
                Ok(data) => all_data.extend(data),
                Err(e) => error!("Twitch - post_gql: {:?}", e),
            }
        }

        Ok(all_data)
    }

    async fn post_gql_chunk(ops: &[Value]) -> AppResult<Vec<GqlResponse>> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Content-Type",
            "text/plain;charset=UTF-8"
                .parse()
                .change_context(AppError::Unknown)?,
        );
        headers.insert(
            "Client-ID",
            CLIENT_ID.parse().change_context(AppError::Unknown)?,
        );

        if let Some(auth_token) = Self::get_auth_token() {
            headers.insert(
                "Authorization",
                format!("OAuth {}", auth_token)
                    .parse()
                    .change_context(AppError::Unknown)?,
            );
        }

        let client = Client::new();
        let resp = client
            .post("https://gql.twitch.tv/gql")
            .headers(headers)
            .json(&ops)
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .change_context(AppError::Unknown)?;

        resp.error_for_status_ref()
            .change_context(AppError::Unknown)?;
        resp.json().await.change_context(AppError::Unknown)
    }
}
