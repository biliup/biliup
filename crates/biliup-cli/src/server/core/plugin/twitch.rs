use crate::server::errors::{AppError, AppResult};
use error_stack::{IntoReport, ResultExt, bail, report, Report};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use async_trait::async_trait;
use tokio::net::TcpListener;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};
use tracing::{error, warn};
use crate::server::core::downloader::Downloader;
use crate::server::core::plugin::{DownloadPlugin, StreamStatus};
use crate::server::infrastructure::context::Context;

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
impl DownloadPlugin for Twitch {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_status(&self, ctx: &mut Context) -> Result<StreamStatus, Report<AppError>> {
        todo!()
    }

    fn danmaku_init(&self) -> Option<Box<dyn Downloader>> {
        None
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
    fname: String,
    url: String,
    suffix: String,
    twitch_danmaku: bool,
    twitch_disable_ads: bool,
    downloader: String,
    pub room_title: Option<String>,
    pub live_cover_url: Option<String>,
    pub raw_stream_url: Option<String>,
    proc: Option<Child>,
    danmaku: Option<DanmakuClient>,
}

impl TwitchDownloader {
    pub fn new(fname: String, url: String, suffix: Option<String>) -> Self {
        Self {
            fname,
            url,
            suffix: suffix.unwrap_or_else(|| "flv".to_string()),
            twitch_danmaku: false,
            twitch_disable_ads: true,
            downloader: "".to_string(),
            room_title: None,
            live_cover_url: None,
            raw_stream_url: None,
            proc: None,
            danmaku: None,
        }
    }

    pub async fn acheck_stream(&mut self, is_check: bool) -> AppResult<bool> {
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
            _ => return Ok(false),
        };

        self.room_title = stream.title;
        self.live_cover_url = stream.preview_image_url;

        if is_check {
            return Ok(true);
        }

        if self.downloader == "streamlink" || self.downloader == "ffmpeg" {
            // 获取可用端口
            let listener = TcpListener::bind("127.0.0.1:0")
                .await
                .change_context(AppError::Unknown)?;
            let port = listener
                .local_addr()
                .change_context(AppError::Unknown)?
                .port()
                .to_string();
            drop(listener);

            let mut args = vec![
                "--player-external-http",
                "--player-external-http-port",
                &port,
                "--player-external-http-interface",
                "localhost",
            ];

            let mut extra_args = Vec::new();

            if self.twitch_disable_ads {
                extra_args.push("--twitch-disable-ads".to_string());
            }

            if let Some(auth_token) = TwitchUtils::get_auth_token() {
                extra_args.push(format!(
                    "--twitch-api-header=Authorization=OAuth {}",
                    auth_token
                ));
            }

            let port_str = port.to_string();
            let mut cmd = Command::new("streamlink");

            for arg in &extra_args {
                cmd.arg(arg);
            }

            cmd.args(&args).arg(&self.url).arg("best");

            let mut child = cmd.spawn().change_context(AppError::Unknown)?;

            self.raw_stream_url = Some(format!("http://localhost:{}", port));

            // 等待进程启动
            for _ in 0..5 {
                if child
                    .try_wait()
                    .change_context(AppError::Unknown)?
                    .is_some()
                {
                    return Ok(false);
                }
                sleep(Duration::from_secs(1)).await;
            }

            self.proc = Some(child);
            Ok(true)
        } else {
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
            self.raw_stream_url = Some(format!(
                "https://usher.ttvnw.net/api/channel/hls/{}.m3u8?{}",
                channel_name, query_string
            ));

            Ok(true)
        }
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
            if let Some(data) = &gql.data {
                if let Some(user) = &data.user {
                    if let Some(stream) = &user.stream {
                        if stream.stream_type.as_deref() == Some("live") {
                            live_urls.push(check_urls[index].clone());
                        }
                    }
                }
            }
        }

        Ok(live_urls)
    }

    pub fn danmaku_init(&mut self) {
        if self.twitch_danmaku {
            self.danmaku = Some(DanmakuClient::new(
                self.url.clone(),
                self.gen_download_filename(),
            ));
        }
    }

    pub async fn close(&mut self) -> AppResult<()> {
        if let Some(mut proc) = self.proc.take() {
            match tokio::time::timeout(Duration::from_secs(5), proc.kill()).await {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => error!("terminate {} failed: {}", self.fname, e),
                Err(_) => {
                    warn!("Timeout expired, force killing process");
                    let _ = proc.kill().await;
                }
            }
        }
        Ok(())
    }

    fn gen_download_filename(&self) -> String {
        format!("{}.{}", self.fname, self.suffix)
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
            if retry == 0 {
                if let Some(auth_token) = Self::get_auth_token() {
                    headers.insert(
                        "Authorization",
                        format!("OAuth {}", auth_token)
                            .parse()
                            .change_context(AppError::Unknown)?,
                    );
                }
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
        Ok(resp.json().await.change_context(AppError::Unknown)?)
    }
}

// 弹幕客户端 (需要根据实际情况实现)
struct DanmakuClient {
    url: String,
    filename: String,
}

impl DanmakuClient {
    fn new(url: String, filename: String) -> Self {
        Self { url, filename }
    }
}
