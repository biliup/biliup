use super::{
    BatchCheckRequest, DanmakuSource, DownloaderHint, LiveError, LivePlugin, LiveRequest,
    LiveResult, LiveStatus, LiveStream, RuntimeOptions, StreamlinkOptions, StreamlinkPlatform,
    YtDlpOptions, media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::RwLock;
use tokio::fs;
use tokio::process::Command;
use tokio::time::Duration;
use tracing::warn;

const CLIENT_ID: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";
const GQL_URL: &str = "https://gql.twitch.tv/gql";
/// 单次 GQL 批量请求的最大操作数（对齐 twitch.py TwitchUtils.post_gql limit=30）
const GQL_BATCH_LIMIT: usize = 30;

/// Twitch 已失效的 auth_token（Twitch 与 TwitchVideos 共享，进程内长期记忆）。
/// 对应 Python 版 TwitchUtils._invalid_auth_token 类级属性。
static INVALID_AUTH_TOKEN: RwLock<Option<String>> = RwLock::new(None);

/// 返回可用的 auth_token：未配置、为空或已被记为失效时返回 None。
fn effective_auth_token(auth_token: Option<&str>) -> Option<String> {
    let auth_token = auth_token.filter(|token| !token.is_empty())?;
    let invalid = INVALID_AUTH_TOKEN
        .read()
        .unwrap_or_else(|err| err.into_inner());
    (invalid.as_deref() != Some(auth_token)).then(|| auth_token.to_string())
}

/// 将 auth_token 记为失效，后续请求跳过；仅首次记录时告警。
fn mark_auth_token_invalid(auth_token: &str) {
    let mut invalid = INVALID_AUTH_TOKEN
        .write()
        .unwrap_or_else(|err| err.into_inner());
    if invalid.as_deref() != Some(auth_token) {
        *invalid = Some(auth_token.to_string());
        warn!("Twitch Cookie已失效请及时更换，后续操作将忽略Twitch Cookie");
    }
}

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

    fn supports_batch_check(&self) -> bool {
        true
    }

    /// 批量检测：一次 GQL 请求最多判定 30 个直播间（对齐 twitch.py:159-184）。
    async fn batch_check(&self, request: BatchCheckRequest) -> LiveResult<Vec<String>> {
        let auth_token = request.credentials.twitch_cookie.clone();
        let mut live_urls = Vec::new();

        for chunk in request.urls.chunks(GQL_BATCH_LIMIT) {
            // 无法解析出频道名的 URL 保留占位，保证响应下标与 chunk 对齐
            let ops: Vec<Value> = chunk
                .iter()
                .map(|url| match self.batch_channel_name(url) {
                    Some(channel_name) => json!({
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
                    }),
                    None => Value::Null,
                })
                .collect();

            let responses =
                post_gql_batch(&request.client, auth_token.as_deref(), &ops).await?;

            for (index, url) in chunk.iter().enumerate() {
                let is_live = responses
                    .get(index)
                    .and_then(|resp| resp.data.as_ref())
                    .and_then(|data| data.user.as_ref())
                    .and_then(|user| user.stream.as_ref())
                    .is_some_and(|stream| stream.stream_type.as_deref() == Some("live"));
                if is_live {
                    live_urls.push(url.clone());
                }
            }
        }

        Ok(live_urls)
    }
}

impl Twitch {
    fn batch_channel_name(&self, url: &str) -> Option<String> {
        self.re
            .captures(url)
            .and_then(|caps| caps.name("id"))
            .map(|m| m.as_str().to_lowercase())
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
                }))
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
                        auth_token: effective_auth_token(self.twitch_auth_token.as_deref()),
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

    async fn post_gql<T: Serialize>(&self, ops: T) -> LiveResult<GqlResponse> {
        let mut auth_token = effective_auth_token(self.twitch_auth_token.as_deref());
        loop {
            let resp = self
                .client
                .post(GQL_URL)
                .headers(gql_headers(auth_token.as_deref())?)
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
                // 带 token 被拒时记为失效并改用匿名请求重试一次
                if let Some(token) = auth_token.take() {
                    mark_auth_token_invalid(&token);
                    continue;
                }
                return Err(LiveError::custom("请求 Twitch GQL 错误: Unauthorized"));
            }
            return Ok(gql);
        }
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
                    cookies_file: self.auth_cookie_file().await,
                    prefer_vcodec: None,
                    prefer_acodec: None,
                    max_filesize: None,
                    max_height: None,
                    download_archive: Some(PathBuf::from("archive.txt")),
                    extra_ytdlp_args: Vec::new(),
                })),
            }),
        })
    }

    /// 运行 yt-dlp 提取信息。`flat` 为 true 时用于 /videos 列表页，
    /// 以 `--flat-playlist` 惰性枚举条目（对应 Python 版 process=False）；
    /// 为 false 时对单个条目做完整提取。
    async fn extract_info(&self, url: &str, flat: bool) -> LiveResult<Option<Value>> {
        let mut auth_token = effective_auth_token(self.twitch_auth_token.as_deref());
        loop {
            let cookie_file = match auth_token.as_deref() {
                Some(token) => write_cookie_file(token).await,
                None => None,
            };
            let mut command = Command::new("yt-dlp");
            command
                .stdin(Stdio::null())
                .arg("--dump-single-json")
                .arg("--skip-download")
                .arg("--ignore-errors")
                .arg("--extractor-retries")
                .arg("0")
                .arg("--no-warnings");
            if flat {
                command.arg("--flat-playlist");
            } else {
                command.arg("--no-playlist");
            }
            if let Some(cookie_file) = &cookie_file {
                command.arg("--cookies").arg(cookie_file);
            }
            command.arg(url).kill_on_drop(true);

            let output = command.output().await.map_err(|err| {
                LiveError::custom(format!("运行 yt-dlp 获取 Twitch 视频信息失败: {err}"))
            })?;
            if output.status.success() {
                return parse_ytdlp_stdout(&output.stdout);
            }
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("Unauthorized")
                && let Some(token) = auth_token.take()
            {
                // 带 token 被拒时记为失效并改用匿名请求重试一次
                mark_auth_token_invalid(&token);
                continue;
            }
            return Ok(None);
        }
    }

    /// auth_token 可用时生成 Netscape cookie 文件，返回其路径传给 yt-dlp --cookies。
    async fn auth_cookie_file(&self) -> Option<PathBuf> {
        let auth_token = effective_auth_token(self.twitch_auth_token.as_deref())?;
        write_cookie_file(&auth_token).await
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
                    .is_none_or(|id| !archive_contains(archive_ids, id))
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

/// 批量 POST GQL 操作数组。带 token 收到 Unauthorized 时记为失效并匿名重试一次
/// （对齐 twitch.py TwitchUtils.__post_gql 的失效处理）。
async fn post_gql_batch(
    client: &Client,
    auth_token: Option<&str>,
    ops: &[Value],
) -> LiveResult<Vec<GqlResponse>> {
    let mut auth_token = effective_auth_token(auth_token);
    loop {
        let resp = client
            .post(GQL_URL)
            .headers(gql_headers(auth_token.as_deref())?)
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
        let body: Value = resp
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析 Twitch GQL 失败: {err}")))?;
        // 鉴权失败时返回的是 {"error": "Unauthorized"} 对象而非数组
        if body.get("error").and_then(Value::as_str) == Some("Unauthorized") {
            if let Some(token) = auth_token.take() {
                mark_auth_token_invalid(&token);
                continue;
            }
            return Err(LiveError::custom("请求 Twitch GQL 错误: Unauthorized"));
        }
        return serde_json::from_value(body)
            .map_err(|err| LiveError::custom(format!("解析 Twitch GQL 批量响应失败: {err}")));
    }
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

/// 判断条目 id 是否已归档。yt-dlp 新版对 Twitch VOD 记录的归档 id 为 `v<数字>`
/// （flat 条目 id 同为 `v<数字>`），旧版本归档为纯数字，两种形式都视为已归档。
fn archive_contains(archive_ids: &HashSet<String>, id: &str) -> bool {
    if archive_ids.contains(id) {
        return true;
    }
    if let Some(stripped) = id.strip_prefix('v') {
        !stripped.is_empty()
            && stripped.bytes().all(|byte| byte.is_ascii_digit())
            && archive_ids.contains(stripped)
    } else {
        !id.is_empty()
            && id.bytes().all(|byte| byte.is_ascii_digit())
            && archive_ids.contains(&format!("v{id}"))
    }
}

/// 生成 Netscape 格式 cookie 文件内容（域 .twitch.tv，键 auth-token），
/// 与 Python 版传给 yt-dlp cookiefile 的内容一致。
fn netscape_cookie_file_content(auth_token: &str) -> String {
    format!(
        "# Netscape HTTP Cookie File\n.twitch.tv\tTRUE\t/\tFALSE\t0\tauth-token\t{auth_token}\n"
    )
}

/// cookie 文件放系统临时目录，路径由 token 哈希决定：
/// 同一 token 复用同一路径，token 变化自然写入新文件。
fn cookie_file_path(auth_token: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    auth_token.hash(&mut hasher);
    std::env::temp_dir().join(format!("biliup-twitch-cookies-{:016x}.txt", hasher.finish()))
}

/// 将 auth_token 写入 Netscape cookie 文件并返回路径；内容未变化时不重写。
/// 写入失败仅告警并返回 None（降级为匿名请求）。
async fn write_cookie_file(auth_token: &str) -> Option<PathBuf> {
    let path = cookie_file_path(auth_token);
    let content = netscape_cookie_file_content(auth_token);
    if fs::read_to_string(&path).await.ok().as_deref() != Some(content.as_str()) {
        if let Err(err) = fs::write(&path, &content).await {
            warn!("写入 Twitch cookie 文件失败: {}: {err}", path.display());
            return None;
        }
    }
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn netscape_cookie_file_content_matches_python_format() {
        assert_eq!(
            netscape_cookie_file_content("abc123"),
            "# Netscape HTTP Cookie File\n.twitch.tv\tTRUE\t/\tFALSE\t0\tauth-token\tabc123\n"
        );
    }

    #[test]
    fn cookie_file_path_is_stable_per_token() {
        assert_eq!(cookie_file_path("token-a"), cookie_file_path("token-a"));
        assert_ne!(cookie_file_path("token-a"), cookie_file_path("token-b"));
        assert!(cookie_file_path("token-a").starts_with(std::env::temp_dir()));
    }

    #[test]
    fn archive_contains_matches_both_id_forms() {
        let archive_ids: HashSet<String> = ["v111", "222", "SomeClipSlug"]
            .into_iter()
            .map(str::to_string)
            .collect();
        // 完全一致
        assert!(archive_contains(&archive_ids, "v111"));
        assert!(archive_contains(&archive_ids, "222"));
        assert!(archive_contains(&archive_ids, "SomeClipSlug"));
        // v 前缀与纯数字互认
        assert!(archive_contains(&archive_ids, "111"));
        assert!(archive_contains(&archive_ids, "v222"));
        // 未归档
        assert!(!archive_contains(&archive_ids, "v333"));
        assert!(!archive_contains(&archive_ids, "333"));
        // 非数字 id 不做前缀换算
        assert!(!archive_contains(&archive_ids, "omeClipSlug"));
        assert!(!archive_contains(&archive_ids, "v"));
        assert!(!archive_contains(&archive_ids, ""));
    }

    #[test]
    fn write_cookie_file_writes_and_reuses_path() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let token = "unit-test-token";
            let path = write_cookie_file(token).await.expect("写入 cookie 文件失败");
            assert_eq!(
                fs::read_to_string(&path).await.unwrap(),
                netscape_cookie_file_content(token)
            );
            // 复用同一路径
            assert_eq!(write_cookie_file(token).await.as_deref(), Some(&*path));
            let _ = fs::remove_file(&path).await;
        });
    }

    #[test]
    fn invalid_auth_token_is_remembered_and_skipped() {
        // 共用全局状态，串行放在同一个测试内
        assert_eq!(effective_auth_token(None), None);
        assert_eq!(effective_auth_token(Some("")), None);
        assert_eq!(
            effective_auth_token(Some("token-x")),
            Some("token-x".to_string())
        );
        mark_auth_token_invalid("token-x");
        assert_eq!(effective_auth_token(Some("token-x")), None);
        // 更换 token 后恢复可用
        assert_eq!(
            effective_auth_token(Some("token-y")),
            Some("token-y".to_string())
        );
    }
}
