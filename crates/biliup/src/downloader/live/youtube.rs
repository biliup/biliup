use super::{
    DanmakuSource, DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus,
    LiveStream, RuntimeOptions, YtDlpBackend, YtDlpOptions, media_ext_from_url,
};
use crate::tools;
use async_trait::async_trait;
use chrono::Utc;
use quick_xml::Reader;
use quick_xml::events::Event;
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::process::Stdio;
use tokio::fs;

pub struct Youtube {
    re: Regex,
}

impl Default for Youtube {
    fn default() -> Self {
        Self::new()
    }
}

impl Youtube {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"^https?://(?:(?:(?:www|m)\.)?youtube\.com|youtu\.be)/").unwrap(),
        }
    }
}

#[async_trait]
impl LivePlugin for Youtube {
    fn name(&self) -> &'static str {
        "Youtube"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        YoutubeLive::new(request).check_stream().await
    }
}

struct YoutubeLive {
    url: String,
    name: String,
    client: reqwest::Client,
    cookie_file: Option<std::path::PathBuf>,
    live_url: bool,
    enable_download_live: bool,
    enable_download_playback: bool,
    after_date: Option<String>,
    before_date: Option<String>,
    prefer_vcodec: Option<String>,
    prefer_acodec: Option<String>,
    max_resolution: Option<u32>,
    max_videosize: Option<String>,
    youtube_danmaku: bool,
}

#[derive(Clone)]
struct YoutubeSelection {
    webpage_url: String,
    download_url: Option<String>,
    title: String,
    thumbnail: String,
    stream_url: Option<String>,
    is_live: bool,
}

/// 从 RSS XML 解析出的单条视频条目。
#[derive(Default, Debug, Clone)]
struct RssEntry {
    id: String,
    title: String,
    webpage_url: String,
    thumbnail: String,
    /// `YYYYMMDD` 格式，解析失败时为空字符串。
    upload_date: String,
}

/// 条目筛选结果，对齐 Python `loop_entries` 的 dict / None / 'stop' 三态。
#[derive(Debug, PartialEq)]
enum EntryDecision {
    /// 命中待下载条目
    Select(Value),
    /// 跳过当前条目，继续扫描
    Skip,
    /// 条目早于 after_date，列表按时间倒序，无需继续扫描当前列表
    Stop,
}

impl YoutubeLive {
    fn new(request: LiveRequest) -> Self {
        let options = request.options.youtube;
        let live_url = is_youtube_live_url(&request.url);
        Self {
            url: request.url,
            name: request.name,
            client: request.client,
            cookie_file: request.credentials.youtube_cookie,
            live_url,
            enable_download_live: options.enable_download_live,
            enable_download_playback: options.enable_download_playback,
            after_date: options.after_date,
            before_date: options.before_date,
            prefer_vcodec: options.prefer_vcodec,
            prefer_acodec: options.prefer_acodec,
            max_resolution: options.max_resolution,
            max_videosize: options.max_videosize,
            youtube_danmaku: options.danmaku,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        // RSS 订阅链接走原生 XML 解析（对齐 Python youtube.py 的 _fetch_rss_video_ids），
        // 其余 URL（普通视频 / 频道 / 播放列表 / 搜索结果）走 yt-dlp --flat-playlist 轻量枚举。
        let info = if is_rss_url(&self.url) {
            self.fetch_rss_entries().await?
        } else {
            self.extract_info(&self.url, true).await?
        };

        let Some(info) = info else {
            return Ok(LiveStatus::Offline);
        };
        let archive_ids = self.archive_ids().await;
        let entry = match self
            .select_entry(&info, &archive_ids, &|entry| self.resolve_entry(entry))
            .await?
        {
            EntryDecision::Select(entry) => entry,
            EntryDecision::Skip | EntryDecision::Stop => return Ok(LiveStatus::Offline),
        };
        let selection = self.selection(&entry);
        if selection.is_live && selection.stream_url.is_none() {
            return Ok(LiveStatus::Offline);
        }

        let raw_stream_url = selection
            .stream_url
            .clone()
            .unwrap_or_else(|| selection.webpage_url.clone());
        let suffix = if selection.is_live {
            media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "flv".to_string())
        } else {
            "mp4".to_string()
        };

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: selection.title.clone(),
                date: Utc::now(),
                live_cover_url: selection.thumbnail.clone(),
                avatar_url: None,
                raw_stream_url,
                platform: "youtube".to_string(),
                stream_headers: HashMap::new(),
                suffix,
                danmaku: self.danmaku_source(&selection),
                downloader_hint: if selection.is_live {
                    DownloaderHint::StreamGears
                } else {
                    DownloaderHint::YtDlp
                },
                runtime_options: Some(RuntimeOptions::YtDlp(YtDlpOptions {
                    webpage_url: selection.webpage_url,
                    download_url: selection.download_url,
                    backend: YtDlpBackend::YtDlp,
                    is_live: selection.is_live,
                    use_live_cover: false,
                    cover_url: (!selection.thumbnail.is_empty()).then_some(selection.thumbnail),
                    cookies_file: self.cookie_file.clone(),
                    prefer_vcodec: self.prefer_vcodec.clone(),
                    prefer_acodec: self.prefer_acodec.clone(),
                    max_filesize: self.max_videosize.clone(),
                    max_height: self.max_resolution,
                    // 直播完成后 yt-dlp 也会写入 archive（对齐 youtube.py:312），
                    // 避免直播结束后同一视频被当作回放重复下载
                    download_archive: Some(std::path::PathBuf::from("archive.txt")),
                    extra_ytdlp_args: Vec::new(),
                })),
            }),
        })
    }

    // ---------- yt-dlp 提取 ----------

    async fn extract_info(&self, url: &str, flat: bool) -> LiveResult<Option<Value>> {
        let mut command = tools::command("yt-dlp");
        command
            .stdin(Stdio::null())
            .arg("--dump-single-json")
            .arg("--skip-download")
            .arg("--ignore-errors")
            .arg("--extractor-retries")
            .arg("0")
            .arg("--no-warnings");

        if flat {
            // 列表页只做轻量枚举，不逐条完整解析；对单视频 URL 无影响
            command.arg("--flat-playlist");
        }

        if let Some(cookie_file) = &self.cookie_file {
            command.arg("--cookies").arg(cookie_file);
        }

        let output = command.arg(url).output().await.map_err(|err| {
            LiveError::custom(format!("运行 yt-dlp 失败，请确认已安装并在 PATH 中: {err}"))
        })?;

        if !output.status.success() {
            return Ok(None);
        }

        parse_ytdlp_stdout(&output.stdout)
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

    // ---------- RSS 原生解析 ----------

    /// 请求 YouTube RSS XML 并解析为 yt-dlp 兼容的播放列表 Value。
    /// 兼容四类 RSS：
    /// - `channel_id=UCxxx`        全内容频道
    /// - `playlist_id=UULFxxx`     纯长视频
    /// - `playlist_id=UUSHxxx`     仅 Shorts
    /// - `playlist_id=UULVxxx`     仅直播回放
    async fn fetch_rss_entries(&self) -> LiveResult<Option<Value>> {
        // RSS 是公开接口；cookie 文件仅交给 yt-dlp 按域名规则使用。
        let req = self
            .client
            .get(&self.url)
            .header("User-Agent", YOUTUBE_UA)
            .timeout(std::time::Duration::from_secs(15));

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(%e, url = %self.url, "YouTube RSS 请求失败");
                return Ok(None);
            }
        };
        if !resp.status().is_success() {
            tracing::warn!(status = %resp.status(), url = %self.url, "YouTube RSS HTTP 错误");
            return Ok(None);
        }

        let xml = match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(%e, "YouTube RSS 响应读取失败");
                return Ok(None);
            }
        };

        let entries = match parse_rss_xml(&xml) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(%e, "YouTube RSS XML 解析失败");
                return Ok(None);
            }
        };

        if entries.is_empty() {
            tracing::debug!(url = %self.url, "YouTube RSS 无视频条目");
            return Ok(None);
        }

        let live_status = infer_live_status_from_url(&self.url);
        let json_entries: Vec<Value> = entries
            .iter()
            .map(|e| rss_entry_to_value(e, live_status))
            .collect();

        Ok(Some(serde_json::json!({
            "_type": "playlist",
            "entries": json_entries,
        })))
    }

    // ---------- 条目筛选 ----------

    /// 递归扫描播放列表，返回首个命中的条目（对齐 Python loop_entries）。
    /// Stop 只终止当前列表的扫描，向上层表现为未命中。
    fn select_entry<'a, F, Fut>(
        &'a self,
        value: &'a Value,
        archive_ids: &'a HashSet<String>,
        resolve: &'a F,
    ) -> Pin<Box<dyn Future<Output = LiveResult<EntryDecision>> + Send + 'a>>
    where
        F: Fn(Value) -> Fut + Sync,
        Fut: Future<Output = LiveResult<Option<Value>>> + Send + 'a,
    {
        Box::pin(async move {
            let Some(object) = value.as_object() else {
                return Ok(EntryDecision::Skip);
            };

            if object.get("_type").and_then(Value::as_str) == Some("playlist") {
                let entries = object.get("entries").and_then(Value::as_array);
                for entry in entries.into_iter().flatten() {
                    match self.select_entry(entry, archive_ids, resolve).await? {
                        selected @ EntryDecision::Select(_) => return Ok(selected),
                        EntryDecision::Stop => return Ok(EntryDecision::Skip),
                        EntryDecision::Skip => {}
                    }
                }
                return Ok(EntryDecision::Skip);
            }

            match self.evaluate_entry(value, object, archive_ids).await? {
                EntryDecision::Select(entry) => {
                    let Some(entry) = resolve(entry).await? else {
                        return Ok(EntryDecision::Skip);
                    };
                    // RSS / flat 条目补全后可能变成预告、直播或回放，需重新筛选。
                    if entry
                        .as_object()
                        .is_some_and(|object| self.entry_allowed(object, archive_ids))
                    {
                        Ok(EntryDecision::Select(entry))
                    } else {
                        Ok(EntryDecision::Skip)
                    }
                }
                decision => Ok(decision),
            }
        })
    }

    async fn evaluate_entry(
        &self,
        value: &Value,
        object: &Map<String, Value>,
        archive_ids: &HashSet<String>,
    ) -> LiveResult<EntryDecision> {
        if !self.entry_allowed(object, archive_ids) {
            return Ok(EntryDecision::Skip);
        }

        // 未配置日期范围时无需 upload_date，直接命中
        if self.after_date.is_none() && self.before_date.is_none() {
            return Ok(EntryDecision::Select(value.clone()));
        }

        // flat 条目缺少 upload_date 时单条完整提取后再判定（对齐 youtube.py:259-272）
        let mut candidate = value.clone();
        if candidate
            .get("upload_date")
            .and_then(Value::as_str)
            .is_none()
            && let Some(url) = string_field(&candidate, &["webpage_url", "url"])
            && let Some(resolved) = self.extract_info(&url, true).await?
        {
            candidate = resolved;
        }

        let Some(upload_date) = candidate
            .get("upload_date")
            .and_then(Value::as_str)
            .map(str::to_string)
        else {
            // 时间必然存在，补全后仍缺失说明条目异常，暂时跳过（对齐 youtube.py:268-270）
            return Ok(EntryDecision::Skip);
        };

        Ok(self.date_decision(candidate, &upload_date))
    }

    fn entry_allowed(&self, object: &Map<String, Value>, archive_ids: &HashSet<String>) -> bool {
        let live_status = object.get("live_status").and_then(Value::as_str);
        if !self.live_status_allowed(live_status) {
            return false;
        }

        // 已在 archive 中的条目视为已下载；直播中的条目例外（对齐 youtube.py:253-257）
        if live_status != Some("is_live")
            && let Some(id) = object.get("id").and_then(Value::as_str)
            && archive_ids.contains(id)
        {
            return false;
        }

        true
    }

    /// live_status 准入判定：
    /// - is_upcoming 一律跳过；
    /// - is_live 由直播下载开关控制；
    /// - /live URL 只处理直播中的条目，其余状态一律跳过；
    /// - was_live 由回放下载开关控制；
    /// - 其他状态（含普通视频）放行。
    fn live_status_allowed(&self, live_status: Option<&str>) -> bool {
        match live_status {
            Some("is_upcoming") => false,
            Some("is_live") => self.enable_download_live,
            _ if self.live_url => false,
            Some("was_live") => self.enable_download_playback,
            _ => true,
        }
    }

    /// 日期范围判定。早于 after_date 时返回 Stop：
    /// 列表按时间倒序，后续条目只会更旧（对齐 youtube.py:274-275）。
    fn date_decision(&self, candidate: Value, upload_date: &str) -> EntryDecision {
        if let Some(after_date) = &self.after_date
            && upload_date < after_date.as_str()
        {
            return EntryDecision::Stop;
        }
        if let Some(before_date) = &self.before_date
            && upload_date > before_date.as_str()
        {
            return EntryDecision::Skip;
        }
        EntryDecision::Select(candidate)
    }

    async fn resolve_entry(&self, entry: Value) -> LiveResult<Option<Value>> {
        if entry.get("_type").and_then(Value::as_str) != Some("url") {
            return Ok(Some(entry));
        }

        let Some(url) = string_field(&entry, &["webpage_url", "url"]) else {
            return Ok(None);
        };

        // 命中的 flat 条目单条完整提取，获得下载所需字段（对齐 youtube.py:292-293）
        self.extract_info(&url, false).await
    }

    fn selection(&self, entry: &Value) -> YoutubeSelection {
        let webpage_url = string_field(entry, &["webpage_url", "original_url", "url"])
            .unwrap_or_else(|| self.url.clone());
        let title =
            string_field(entry, &["fulltitle", "title"]).unwrap_or_else(|| self.name.clone());
        let thumbnail = string_field(entry, &["thumbnail"]).unwrap_or_default();
        let is_live = entry.get("live_status").and_then(Value::as_str) == Some("is_live");
        let stream_url = is_live.then(|| self.stream_url(entry)).flatten();

        YoutubeSelection {
            webpage_url: webpage_url.clone(),
            download_url: Some(webpage_url),
            title,
            thumbnail,
            stream_url,
            is_live,
        }
    }

    fn stream_url(&self, entry: &Value) -> Option<String> {
        if let Some(manifest_url) = entry.get("manifest_url").and_then(Value::as_str)
            && !manifest_url.is_empty()
        {
            return Some(manifest_url.to_string());
        }

        entry
            .get("formats")
            .and_then(Value::as_array)
            .and_then(|formats| {
                formats.iter().rev().find_map(|format| {
                    let protocol = format.get("protocol").and_then(Value::as_str).unwrap_or("");
                    if !protocol.contains("m3u8") {
                        return None;
                    }
                    format
                        .get("url")
                        .and_then(Value::as_str)
                        .filter(|url| !url.is_empty())
                        .map(str::to_string)
                })
            })
    }

    fn danmaku_source(&self, selection: &YoutubeSelection) -> Option<DanmakuSource> {
        if !self.youtube_danmaku || !selection.is_live {
            return None;
        }
        Some(DanmakuSource {
            platform: "youtube".to_string(),
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

// ========== 纯函数工具 ==========

const YOUTUBE_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
    AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

fn parse_ytdlp_stdout(stdout: &[u8]) -> LiveResult<Option<Value>> {
    let stdout = String::from_utf8_lossy(stdout);
    let stdout = stdout.trim();
    if stdout.is_empty() || stdout == "null" {
        return Ok(None);
    }
    serde_json::from_str(stdout)
        .map(Some)
        .map_err(|err| LiveError::custom(format!("解析 YouTube 信息失败: {err}")))
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

fn is_youtube_live_url(url: &str) -> bool {
    url.trim_end_matches('/').ends_with("/live")
}

/// 判断是否为 YouTube RSS 订阅链接。
fn is_rss_url(url: &str) -> bool {
    let Ok(url) = url::Url::parse(url) else {
        return false;
    };
    matches!(url.scheme(), "http" | "https")
        && url
            .host_str()
            .is_some_and(|host| host == "youtube.com" || host.ends_with(".youtube.com"))
        && url.path() == "/feeds/videos.xml"
}

/// 根据 RSS URL 中的 playlist_id 前缀推断视频类型：
/// - `UULV` → 直播回放（`was_live`）
/// - `UULF` / `UUSH` → 普通视频（`None`）
/// - `channel_id` 或其他 → `None`（交由后续 yt-dlp 补全 live_status）
fn infer_live_status_from_url(url: &str) -> Option<&'static str> {
    let parsed = url::Url::parse(url).ok()?;
    let playlist_id = parsed
        .query_pairs()
        .find_map(|(k, v)| (k == "playlist_id").then_some(v.into_owned()))?;
    if playlist_id.starts_with("UULV") {
        Some("was_live")
    } else {
        None
    }
}

/// 将 RSS 条目转为 yt-dlp 兼容的 JSON Value。
/// `_type: "url"` 使 resolve_entry 对命中条目做完整 yt-dlp 提取以获取流地址。
fn rss_entry_to_value(entry: &RssEntry, live_status: Option<&str>) -> Value {
    let mut obj = Map::new();
    obj.insert("id".to_string(), Value::String(entry.id.clone()));
    obj.insert("title".to_string(), Value::String(entry.title.clone()));
    obj.insert(
        "webpage_url".to_string(),
        Value::String(entry.webpage_url.clone()),
    );
    obj.insert("url".to_string(), Value::String(entry.webpage_url.clone()));
    obj.insert(
        "thumbnail".to_string(),
        Value::String(entry.thumbnail.clone()),
    );
    if !entry.upload_date.is_empty() {
        obj.insert(
            "upload_date".to_string(),
            Value::String(entry.upload_date.clone()),
        );
    }
    match live_status {
        Some(s) => obj.insert("live_status".to_string(), Value::String(s.to_string())),
        None => obj.insert("live_status".to_string(), Value::Null),
    };
    obj.insert("_type".to_string(), Value::String("url".to_string()));
    Value::Object(obj)
}

/// 用 quick-xml 解析 YouTube RSS XML，提取视频条目列表。
fn parse_rss_xml(xml: &str) -> Result<Vec<RssEntry>, LiveError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    reader.config_mut().expand_empty_elements = true;

    let mut entries: Vec<RssEntry> = Vec::new();
    let mut current: Option<RssEntry> = None;
    let mut text_buf = String::new();
    let mut in_media_group = false;

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local = e.name().local_name();
                match local.as_ref() {
                    b"entry" => {
                        current = Some(RssEntry::default());
                    }
                    b"videoId" | b"title" | b"published" => {
                        text_buf.clear();
                    }
                    b"link" => {
                        if let Some(entry) = current.as_mut() {
                            for attr in e.attributes().flatten() {
                                if attr.key.local_name().as_ref() == b"href" {
                                    entry.webpage_url = attr
                                        .unescape_value()
                                        .map_err(|e| {
                                            LiveError::custom(format!("RSS XML 解析错误: {e}"))
                                        })?
                                        .into_owned();
                                }
                            }
                        }
                    }
                    b"group" => {
                        in_media_group = true;
                    }
                    b"thumbnail" => {
                        if in_media_group {
                            if let Some(entry) = current.as_mut() {
                                for attr in e.attributes().flatten() {
                                    if attr.key.local_name().as_ref() == b"url" {
                                        entry.thumbnail = attr
                                            .unescape_value()
                                            .map_err(|e| {
                                                LiveError::custom(format!("RSS XML 解析错误: {e}"))
                                            })?
                                            .into_owned();
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if let Ok(text) = e.unescape() {
                    text_buf.push_str(&text);
                }
            }
            Ok(Event::End(e)) => {
                let local = e.name().local_name();
                match local.as_ref() {
                    b"entry" => {
                        if let Some(mut entry) = current.take() {
                            if !entry.id.is_empty() {
                                if entry.webpage_url.is_empty() {
                                    entry.webpage_url =
                                        format!("https://www.youtube.com/watch?v={}", entry.id);
                                }
                                if entry.thumbnail.is_empty() {
                                    entry.thumbnail = format!(
                                        "https://i.ytimg.com/vi/{}/hqdefault.jpg",
                                        entry.id
                                    );
                                }
                                entries.push(entry);
                            }
                        }
                    }
                    b"videoId" => {
                        if let Some(entry) = current.as_mut() {
                            entry.id = text_buf.trim().to_string();
                        }
                        text_buf.clear();
                    }
                    b"title" => {
                        if let Some(entry) = current.as_mut() {
                            entry.title = text_buf.trim().to_string();
                        }
                        text_buf.clear();
                    }
                    b"published" => {
                        if let Some(entry) = current.as_mut() {
                            // 2024-01-01T00:00:00+00:00 → 20240101
                            entry.upload_date = text_buf
                                .trim()
                                .get(..10)
                                .map(|s| s.replace('-', ""))
                                .unwrap_or_default();
                        }
                        text_buf.clear();
                    }
                    b"group" => {
                        in_media_group = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(LiveError::custom(format!("RSS XML 解析错误: {e}")));
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    async fn keep_entry(entry: Value) -> LiveResult<Option<Value>> {
        Ok(Some(entry))
    }

    fn youtube_live(
        live_url: bool,
        enable_download_live: bool,
        enable_download_playback: bool,
    ) -> YoutubeLive {
        YoutubeLive {
            url: "https://www.youtube.com/@test".to_string(),
            name: "test".to_string(),
            client: reqwest::Client::new(),
            cookie_file: None,
            live_url,
            enable_download_live,
            enable_download_playback,
            after_date: None,
            before_date: None,
            prefer_vcodec: None,
            prefer_acodec: None,
            max_resolution: None,
            max_videosize: None,
            youtube_danmaku: false,
        }
    }

    #[test]
    fn matches_youtube_urls() {
        let plugin = Youtube::new();

        assert!(plugin.matches("https://www.youtube.com/watch?v=test"));
        assert!(plugin.matches("https://m.youtube.com/watch?v=test"));
        assert!(plugin.matches("https://youtu.be/test"));
        assert!(!plugin.matches("https://example.com/watch?v=test"));
        assert!(
            !plugin.matches("https://evil.example/?next=https://www.youtube.com/feeds/videos.xml")
        );
    }

    #[test]
    fn rss_url_detection() {
        assert!(is_rss_url(
            "https://www.youtube.com/feeds/videos.xml?channel_id=UCxxx"
        ));
        assert!(is_rss_url(
            "https://www.youtube.com/feeds/videos.xml?playlist_id=UULFxxx"
        ));
        assert!(is_rss_url(
            "https://m.youtube.com/feeds/videos.xml?playlist_id=UUSHxxx"
        ));
        assert!(!is_rss_url("https://www.youtube.com/watch?v=test"));
        assert!(!is_rss_url(
            "https://www.youtube.com/results?search_query=test"
        ));
        for url in [
            "https://evil.example/?next=https://www.youtube.com/feeds/videos.xml",
            "https://notyoutube.com/feeds/videos.xml",
            "https://youtube.com.evil.example/feeds/videos.xml",
            "https://www.youtube.com/feeds/videos.xml/extra",
        ] {
            assert!(!is_rss_url(url), "{url}");
        }
    }

    #[test]
    fn infer_live_status_from_playlist_id() {
        assert_eq!(
            infer_live_status_from_url(
                "https://www.youtube.com/feeds/videos.xml?playlist_id=UULVabc"
            ),
            Some("was_live")
        );
        assert_eq!(
            infer_live_status_from_url(
                "https://www.youtube.com/feeds/videos.xml?playlist_id=UULFabc"
            ),
            None
        );
        assert_eq!(
            infer_live_status_from_url(
                "https://www.youtube.com/feeds/videos.xml?playlist_id=UUSHabc"
            ),
            None
        );
        assert_eq!(
            infer_live_status_from_url("https://www.youtube.com/feeds/videos.xml?channel_id=UCabc"),
            None
        );
    }

    #[test]
    fn parse_rss_xml_extracts_entries() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns:yt="http://www.youtube.com/xml/schemas/2015"
      xmlns:media="http://search.yahoo.com/mrss/"
      xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <id>yt:video:abc123</id>
    <yt:videoId>abc123</yt:videoId>
    <yt:channelId>UCtest</yt:channelId>
    <title>Test Video 1</title>
    <link rel="alternate" href="https://www.youtube.com/watch?v=abc123&amp;feature=feed"/>
    <author><name>Test Channel</name></author>
    <published>2024-06-15T12:00:00+00:00</published>
    <media:group>
      <media:title>Test Video 1</media:title>
      <media:thumbnail url="https://i.ytimg.com/vi/abc123/maxresdefault.jpg" width="1280" height="720"/>
    </media:group>
  </entry>
  <entry>
    <id>yt:video:def456</id>
    <yt:videoId>def456</yt:videoId>
    <title>中文标题 Video 2</title>
    <link rel="alternate" href="https://www.youtube.com/watch?v=def456"/>
    <published>2024-07-20T08:30:00+00:00</published>
    <media:group>
      <media:thumbnail url="https://i.ytimg.com/vi/def456/hqdefault.jpg"/>
    </media:group>
  </entry>
</feed>"#;

        let entries = parse_rss_xml(xml).unwrap();
        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].id, "abc123");
        assert_eq!(entries[0].title, "Test Video 1");
        assert_eq!(
            entries[0].webpage_url,
            "https://www.youtube.com/watch?v=abc123&feature=feed"
        );
        assert_eq!(
            entries[0].thumbnail,
            "https://i.ytimg.com/vi/abc123/maxresdefault.jpg"
        );
        assert_eq!(entries[0].upload_date, "20240615");

        assert_eq!(entries[1].id, "def456");
        assert_eq!(entries[1].title, "中文标题 Video 2");
        assert_eq!(entries[1].upload_date, "20240720");
    }

    #[test]
    fn rss_entry_converts_to_ytdlp_value() {
        let entry = RssEntry {
            id: "abc123".to_string(),
            title: "Test".to_string(),
            webpage_url: "https://www.youtube.com/watch?v=abc123".to_string(),
            thumbnail: "https://i.ytimg.com/vi/abc123/hqdefault.jpg".to_string(),
            upload_date: "20240615".to_string(),
        };

        let value = rss_entry_to_value(&entry, Some("was_live"));
        assert_eq!(value.get("id").and_then(Value::as_str), Some("abc123"));
        assert_eq!(value.get("title").and_then(Value::as_str), Some("Test"));
        assert_eq!(
            value.get("webpage_url").and_then(Value::as_str),
            Some("https://www.youtube.com/watch?v=abc123")
        );
        assert_eq!(
            value.get("upload_date").and_then(Value::as_str),
            Some("20240615")
        );
        assert_eq!(
            value.get("live_status").and_then(Value::as_str),
            Some("was_live")
        );
        assert_eq!(value.get("_type").and_then(Value::as_str), Some("url"));

        // live_status = None 序列化为 null
        let value_null = rss_entry_to_value(&entry, None);
        assert!(value_null.get("live_status").unwrap().is_null());
    }

    #[test]
    fn live_url_allows_ongoing_live() {
        // 回归测试：/live URL 直播中且开启直播下载时应放行，
        // 原 match 会落入 `_ if self.live_url` 臂被误拒
        let live = youtube_live(true, true, true);
        assert!(live.live_status_allowed(Some("is_live")));

        let entry = json!({"id": "abc", "live_status": "is_live"});
        assert!(live.entry_allowed(entry.as_object().unwrap(), &HashSet::new()));
    }

    #[test]
    fn live_status_matrix() {
        for live_url in [false, true] {
            for enable_live in [false, true] {
                for enable_playback in [false, true] {
                    let live = youtube_live(live_url, enable_live, enable_playback);

                    // 预告一律跳过
                    assert!(!live.live_status_allowed(Some("is_upcoming")));
                    // 直播中仅由直播下载开关控制，与 /live URL 无关
                    assert_eq!(live.live_status_allowed(Some("is_live")), enable_live);
                    // 回放：/live URL 一律跳过，否则由回放开关控制
                    assert_eq!(
                        live.live_status_allowed(Some("was_live")),
                        !live_url && enable_playback
                    );
                    // 普通视频/未知状态：仅 /live URL 跳过
                    assert_eq!(live.live_status_allowed(None), !live_url);
                    assert_eq!(live.live_status_allowed(Some("post_live")), !live_url);
                }
            }
        }
    }

    #[test]
    fn archived_entry_skipped_unless_live() {
        let live = youtube_live(false, true, true);
        let archive: HashSet<String> = ["abc".to_string()].into();

        // 已归档的回放跳过
        let vod = json!({"id": "abc", "live_status": "was_live"});
        assert!(!live.entry_allowed(vod.as_object().unwrap(), &archive));

        // 直播中的条目即使已在 archive 中也不算已下载
        let living = json!({"id": "abc", "live_status": "is_live"});
        assert!(live.entry_allowed(living.as_object().unwrap(), &archive));

        // 未归档的回放放行
        let fresh = json!({"id": "xyz", "live_status": "was_live"});
        assert!(live.entry_allowed(fresh.as_object().unwrap(), &archive));
    }

    #[test]
    fn date_decision_respects_range() {
        let mut live = youtube_live(false, true, true);
        live.after_date = Some("20240101".to_string());
        live.before_date = Some("20241231".to_string());

        assert_eq!(
            live.date_decision(json!({}), "20240601"),
            EntryDecision::Select(json!({}))
        );
        // 早于 after_date：列表按时间倒序，停止扫描
        assert_eq!(
            live.date_decision(json!({}), "20231231"),
            EntryDecision::Stop
        );
        // 晚于 before_date：跳过当前条目继续扫描
        assert_eq!(
            live.date_decision(json!({}), "20250101"),
            EntryDecision::Skip
        );
    }

    #[tokio::test]
    async fn selects_first_allowed_entry_from_flat_playlist() {
        let live = youtube_live(false, true, true);
        let archive: HashSet<String> = ["done".to_string()].into();
        let info = json!({
            "_type": "playlist",
            "entries": [
                {"_type": "url", "id": "up", "url": "https://www.youtube.com/watch?v=up", "live_status": "is_upcoming"},
                {"_type": "url", "id": "done", "url": "https://www.youtube.com/watch?v=done", "live_status": "was_live"},
                {"_type": "url", "id": "vod", "url": "https://www.youtube.com/watch?v=vod", "live_status": "was_live"},
            ]
        });

        match live
            .select_entry(&info, &archive, &keep_entry)
            .await
            .unwrap()
        {
            EntryDecision::Select(entry) => {
                assert_eq!(entry.get("id").and_then(Value::as_str), Some("vod"));
            }
            other => panic!("应命中 vod 条目，实际为 {other:?}"),
        }
    }

    #[tokio::test]
    async fn stops_scanning_when_older_than_after_date() {
        let mut live = youtube_live(false, true, true);
        live.after_date = Some("20240101".to_string());
        let info = json!({
            "_type": "playlist",
            "entries": [
                {"_type": "url", "id": "old", "url": "https://www.youtube.com/watch?v=old", "upload_date": "20230101"},
                {"_type": "url", "id": "new", "url": "https://www.youtube.com/watch?v=new", "upload_date": "20240601"},
            ]
        });

        // 首个条目早于 after_date 即停止扫描当前列表，后续条目不再命中
        assert_eq!(
            live.select_entry(&info, &HashSet::new(), &keep_entry)
                .await
                .unwrap(),
            EntryDecision::Skip
        );
    }

    #[tokio::test]
    async fn rss_playlist_feeds_into_select_entry() {
        // 验证 RSS 解析出的 playlist Value 能被 select_entry 正确遍历
        let live = youtube_live(false, true, true);
        let rss_entries = vec![
            RssEntry {
                id: "v1".to_string(),
                title: "Video 1".to_string(),
                webpage_url: "https://www.youtube.com/watch?v=v1".to_string(),
                thumbnail: "https://i.ytimg.com/vi/v1/hqdefault.jpg".to_string(),
                upload_date: "20240601".to_string(),
            },
            RssEntry {
                id: "v2".to_string(),
                title: "Video 2".to_string(),
                webpage_url: "https://www.youtube.com/watch?v=v2".to_string(),
                thumbnail: "https://i.ytimg.com/vi/v2/hqdefault.jpg".to_string(),
                upload_date: "20240602".to_string(),
            },
        ];
        let json_entries: Vec<Value> = rss_entries
            .iter()
            .map(|e| rss_entry_to_value(e, None))
            .collect();
        let playlist = json!({
            "_type": "playlist",
            "entries": json_entries,
        });

        match live
            .select_entry(&playlist, &HashSet::new(), &keep_entry)
            .await
            .unwrap()
        {
            EntryDecision::Select(entry) => {
                assert_eq!(entry.get("id").and_then(Value::as_str), Some("v1"));
            }
            other => panic!("应命中 v1，实际为 {other:?}"),
        }
    }

    #[tokio::test]
    async fn rss_selection_rechecks_resolved_status_and_continues() {
        let playlist = json!({
            "_type": "playlist",
            "entries": [
                rss_entry_to_value(&RssEntry { id: "first".into(), ..Default::default() }, None),
                rss_entry_to_value(&RssEntry { id: "next".into(), ..Default::default() }, None),
            ],
        });
        for enable_live in [false, true] {
            for enable_playback in [false, true] {
                let live = youtube_live(false, enable_live, enable_playback);
                for (status, allowed) in [
                    ("is_upcoming", false),
                    ("is_live", enable_live),
                    ("was_live", enable_playback),
                    ("not_live", true),
                ] {
                    let resolve = |mut entry: Value| {
                        assert!(entry["live_status"].is_null());
                        entry["_type"] = json!("video");
                        entry["live_status"] = if entry["id"] == "first" {
                            json!(status)
                        } else {
                            json!("not_live")
                        };
                        std::future::ready(Ok(Some(entry)))
                    };
                    let EntryDecision::Select(entry) = live
                        .select_entry(&playlist, &HashSet::new(), &resolve)
                        .await
                        .unwrap()
                    else {
                        panic!("应继续扫描并命中可下载条目");
                    };
                    assert_eq!(entry["id"], if allowed { "first" } else { "next" });
                }
            }
        }

        let live = youtube_live(false, true, true);
        let resolve = |entry: Value| {
            let resolved = (entry["id"] != "first").then_some(entry);
            std::future::ready(Ok(resolved))
        };
        let EntryDecision::Select(entry) = live
            .select_entry(&playlist, &HashSet::new(), &resolve)
            .await
            .unwrap()
        else {
            panic!("解析失败后应继续扫描下一条");
        };
        assert_eq!(entry["id"], "next");
    }

    #[tokio::test]
    async fn rss_request_does_not_send_cookie_file() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = axum::Router::new().route(
            "/",
            axum::routing::get(|headers: axum::http::HeaderMap| async move {
                assert!(!headers.contains_key("cookie"));
                "<feed><entry><videoId>test</videoId><title>Test</title></entry></feed>"
            }),
        );
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let cookie_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            cookie_file.path(),
            ".youtube.com\tTRUE\t/\tFALSE\t9999999999\tSID\ttest-cookie\n",
        )
        .unwrap();
        let mut live = youtube_live(false, true, true);
        live.url = format!("http://{addr}/");
        live.cookie_file = Some(cookie_file.path().to_path_buf());

        let playlist = live.fetch_rss_entries().await.unwrap().unwrap();
        assert_eq!(playlist["entries"][0]["id"], "test");
        server.abort();
    }
}
