use super::{
    DanmakuSource, DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus,
    LiveStream, RuntimeOptions, StreamlinkOptions, StreamlinkPlatform, YtDlpOptions,
    media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;
use tokio::time::Duration;

const CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";
const GQL_URL: &str = "https://gql.twitch.tv/gql";

pub struct TwitchVideos {
    re: Regex,
}

impl Default for TwitchVideos {
    fn default() -> Self {
        Self::new()
    }
}

impl TwitchVideos {
    pub fn new() -> Self {
        Self {
            re: Regex::new(
                r"https?://(?:(?:www|go|m)\.)?twitch\.tv/(?P<id>[^/]+)/(?:videos|profile|clips)",
            )
            .unwrap(),
        }
    }
}

#[async_trait]
impl LivePlugin for TwitchVideos {
    fn name(&self) -> &'static str {
        "TwitchVideos"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        TwitchVideosLive::new(request).check_stream().await
    }
}

pub struct Twitch {
    re: Regex,
}

impl Default for Twitch {
    fn default() -> Self {
        Self::new()
    }
}

impl Twitch {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"https?://(?:(?:www|go|m)\.)?twitch\.tv/(?P<id>[0-9_a-zA-Z]+)")
                .unwrap(),
        }
    }
}

#[async_trait]
impl LivePlugin for Twitch {
    fn name(&self) -> &'static str {
        "Twitch"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        TwitchLive::new(request, self.re.clone())
            .check_stream()
            .await
    }
}

struct TwitchLive {
    client: Client,
    url: String,
    name: String,
    re: Regex,
    twitch_danmaku: bool,
    twitch_disable_ads: bool,
    twitch_auth_token: Option<String>,
}

impl TwitchLive {
    fn new(request: LiveRequest, re: Regex) -> Self {
        let options = request.options.twitch;
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
            re,
            twitch_danmaku: options.danmaku,
            twitch_disable_ads: options.disable_ads,
            twitch_auth_token: request.credentials.twitch_cookie,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        let channel_name = self.channel_name(&self.url)?;
        let gql: GqlResponse = self
            .post_gql(
                json!({
                    "query": r#"
                        query query($channel_name:String!) {
                            user(login: $channel_name){
                                stream {
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
                    "#,
                    "variables": { "channel_name": channel_name }
                }),
                self.twitch_auth_token.as_deref(),
            )
            .await?;

        let user = gql
            .data
            .and_then(|data| data.user)
            .ok_or_else(|| LiveError::custom("获取 Twitch 用户信息失败"))?;
        let Some(stream) = user
            .stream
            .filter(|stream| stream.stream_type.as_deref() == Some("live"))
        else {
            return Ok(LiveStatus::Offline);
        };
        let token = stream
            .playback_access_token
            .ok_or_else(|| LiveError::custom("Twitch playback access token 为空"))?;
        let raw_stream_url = format!(
            "https://usher.ttvnw.net/api/channel/hls/{}.m3u8?{}",
            channel_name,
            serde_urlencoded::to_string([
                ("player", "twitchweb".to_string()),
                ("p", rand::random::<u32>().to_string()),
                ("allow_source", "true".to_string()),
                ("allow_audio_only", "true".to_string()),
                ("allow_spectre", "false".to_string()),
                ("fast_bread", "true".to_string()),
                ("sig", token.signature),
                ("token", token.value),
            ])
            .map_err(|err| LiveError::custom(format!("构造 Twitch HLS 参数失败: {err}")))?
        );

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: stream.title.unwrap_or_default(),
                date: Utc::now(),
                live_cover_url: stream.preview_image_url.unwrap_or_default(),
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "m3u8".to_string()),
                raw_stream_url,
                platform: "twitch".to_string(),
                stream_headers: HashMap::new(),
                danmaku: self.danmaku_source(),
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: Some(RuntimeOptions::Streamlink(StreamlinkOptions {
                    url: Some(self.url.clone()),
                    platform: StreamlinkPlatform::Twitch {
                        disable_ads: self.twitch_disable_ads,
                        auth_token: self.twitch_auth_token.clone(),
                    },
                })),
            }),
        })
    }

    fn channel_name(&self, url: &str) -> LiveResult<String> {
        self.re
            .captures(url)
            .and_then(|caps| caps.name("id"))
            .map(|m| m.as_str().to_lowercase())
            .ok_or_else(|| LiveError::custom("Twitch 直播间地址错误"))
    }

    async fn post_gql<T: Serialize>(
        &self,
        ops: T,
        auth_token: Option<&str>,
    ) -> LiveResult<GqlResponse> {
        for retry in 0..2 {
            let resp = self
                .client
                .post(GQL_URL)
                .headers(gql_headers((retry == 0).then_some(auth_token).flatten())?)
                .json(&ops)
                .timeout(Duration::from_secs(15))
                .send()
                .await
                .map_err(|err| LiveError::custom(format!("请求 Twitch GQL 失败: {err}")))?;
            if !resp.status().is_success() {
                return Err(LiveError::custom(format!(
                    "请求 Twitch GQL 错误: {}",
                    resp.status()
                )));
            }
            let gql: GqlResponse = resp
                .json()
                .await
                .map_err(|err| LiveError::custom(format!("解析 Twitch GQL 失败: {err}")))?;
            if gql.error.as_deref() == Some("Unauthorized") {
                continue;
            }
            return Ok(gql);
        }
        Err(LiveError::custom("Twitch Cookie 已失效"))
    }

    fn danmaku_source(&self) -> Option<DanmakuSource> {
        if !self.twitch_danmaku {
            return None;
        }
        Some(DanmakuSource {
            platform: "twitch".to_string(),
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

struct TwitchVideosLive {
    url: String,
    name: String,
    twitch_auth_token: Option<String>,
}

impl TwitchVideosLive {
    fn new(request: LiveRequest) -> Self {
        Self {
            url: request.url,
            name: request.name,
            twitch_auth_token: request.credentials.twitch_cookie,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        let Some(info) = self.extract_info(&self.url, true).await? else {
            return Ok(LiveStatus::Offline);
        };
        let archive_ids = self.archive_ids().await;
        let Some(entry) = self.select_entry(&info, &archive_ids) else {
            return Ok(LiveStatus::Offline);
        };
        let entry = self.resolve_entry(entry).await?;
        let selection = self.selection(&entry);
        let raw_stream_url = selection
            .download_url
            .clone()
            .unwrap_or_else(|| selection.webpage_url.clone());

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: selection.title,
                date: Utc::now(),
                live_cover_url: selection.thumbnail.clone(),
                suffix: "mp4".to_string(),
                raw_stream_url,
                platform: "twitch".to_string(),
                stream_headers: HashMap::new(),
                danmaku: None,
                downloader_hint: DownloaderHint::YtDlp,
                runtime_options: Some(RuntimeOptions::YtDlp(YtDlpOptions {
                    webpage_url: selection.webpage_url,
                    download_url: selection.download_url,
                    backend: super::YtDlpBackend::YtDlp,
                    is_live: false,
                    use_live_cover: false,
                    cover_url: (!selection.thumbnail.is_empty()).then_some(selection.thumbnail),
                    cookies_file: None,
                    prefer_vcodec: None,
                    prefer_acodec: None,
                    max_filesize: None,
                    max_height: None,
                    download_archive: Some(PathBuf::from("archive.txt")),
                    extra_ytdlp_args: self.auth_cookie_args(),
                })),
            }),
        })
    }

    async fn extract_info(&self, url: &str, process: bool) -> LiveResult<Option<Value>> {
        let mut command = Command::new("yt-dlp");
        command
            .stdin(Stdio::null())
            .arg("--dump-single-json")
            .arg("--skip-download")
            .arg("--ignore-errors")
            .arg("--extractor-retries")
            .arg("0")
            .arg("--no-warnings");
        if !process {
            command.arg("--no-playlist");
        }
        for arg in self.auth_cookie_args() {
            command.arg(arg);
        }
        command.arg(url).kill_on_drop(true);

        let output = command.output().await.map_err(|err| {
            LiveError::custom(format!("运行 yt-dlp 获取 Twitch 视频信息失败: {err}"))
        })?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            if stderr.contains("Unauthorized") && self.twitch_auth_token.is_some() {
                return self.extract_info_without_auth(url, process).await;
            }
            return Ok(None);
        }

        parse_ytdlp_stdout(&output.stdout)
    }

    async fn extract_info_without_auth(
        &self,
        url: &str,
        process: bool,
    ) -> LiveResult<Option<Value>> {
        let mut command = Command::new("yt-dlp");
        command
            .stdin(Stdio::null())
            .arg("--dump-single-json")
            .arg("--skip-download")
            .arg("--ignore-errors")
            .arg("--extractor-retries")
            .arg("0")
            .arg("--no-warnings");
        if !process {
            command.arg("--no-playlist");
        }
        command.arg(url).kill_on_drop(true);

        let output = command.output().await.map_err(|err| {
            LiveError::custom(format!("运行 yt-dlp 获取 Twitch 视频信息失败: {err}"))
        })?;
        if !output.status.success() {
            return Ok(None);
        }
        parse_ytdlp_stdout(&output.stdout)
    }

    fn auth_cookie_args(&self) -> Vec<String> {
        let Some(auth_token) = self
            .twitch_auth_token
            .as_deref()
            .filter(|value| !value.is_empty())
        else {
            return Vec::new();
        };
        vec![
            "--add-header".to_string(),
            format!("Cookie: auth-token={auth_token}"),
        ]
    }

    async fn archive_ids(&self) -> HashSet<String> {
        fs::read_to_string("archive.txt")
            .await
            .unwrap_or_default()
            .lines()
            .filter_map(|line| line.split_whitespace().last())
            .map(str::to_string)
            .collect()
    }

    fn select_entry<'a>(
        &self,
        value: &'a Value,
        archive_ids: &HashSet<String>,
    ) -> Option<&'a Value> {
        if let Some(entries) = value.get("entries").and_then(Value::as_array) {
            entries.iter().find(|entry| {
                entry
                    .get("id")
                    .and_then(Value::as_str)
                    .is_none_or(|id| !archive_ids.contains(id))
            })
        } else {
            Some(value)
        }
    }

    async fn resolve_entry(&self, entry: &Value) -> LiveResult<Value> {
        let url = string_field(entry, &["url", "webpage_url"]).unwrap_or_else(|| self.url.clone());
        Ok(self
            .extract_info(&url, false)
            .await?
            .unwrap_or_else(|| entry.clone()))
    }

    fn selection(&self, entry: &Value) -> TwitchVideosSelection {
        let webpage_url = string_field(entry, &["webpage_url", "original_url", "url"])
            .unwrap_or_else(|| self.url.clone());
        let download_url = string_field(entry, &["webpage_url", "original_url", "url"])
            .or_else(|| Some(webpage_url.clone()));
        let title =
            string_field(entry, &["fulltitle", "title"]).unwrap_or_else(|| self.name.clone());
        let thumbnail = entry
            .get("thumbnails")
            .and_then(Value::as_array)
            .and_then(|thumbnails| thumbnails.last())
            .and_then(|thumbnail| thumbnail.get("url"))
            .and_then(Value::as_str)
            .filter(|url| !url.is_empty())
            .map(str::to_string)
            .or_else(|| string_field(entry, &["thumbnail"]))
            .unwrap_or_default();

        TwitchVideosSelection {
            webpage_url,
            download_url,
            title,
            thumbnail,
        }
    }
}

#[derive(Clone)]
struct TwitchVideosSelection {
    webpage_url: String,
    download_url: Option<String>,
    title: String,
    thumbnail: String,
}

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

fn gql_headers(auth_token: Option<&str>) -> LiveResult<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("text/plain;charset=UTF-8"),
    );
    headers.insert("Client-ID", HeaderValue::from_static(CLIENT_ID));
    if let Some(auth_token) = auth_token {
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&format!("OAuth {auth_token}"))?,
        );
    }
    Ok(headers)
}

fn parse_ytdlp_stdout(stdout: &[u8]) -> LiveResult<Option<Value>> {
    let stdout = String::from_utf8_lossy(stdout);
    let stdout = stdout.trim();
    if stdout.is_empty() || stdout == "null" {
        return Ok(None);
    }
    serde_json::from_str(stdout)
        .map(Some)
        .map_err(|err| LiveError::custom(format!("解析 Twitch 视频信息失败: {err}")))
}

fn string_field(entry: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        entry
            .get(*key)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}
