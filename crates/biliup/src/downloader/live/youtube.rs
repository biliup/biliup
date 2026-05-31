use super::{
    DanmakuSource, DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus,
    LiveStream, RuntimeOptions, YtDlpBackend, YtDlpOptions, media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;

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
            re: Regex::new(r"https?://(?:(?:(?:www|m)\.)?youtube\.com|youtu\.be)/").unwrap(),
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

impl YoutubeLive {
    fn new(request: LiveRequest) -> Self {
        let options = request.options.youtube;
        let live_url = is_youtube_live_url(&request.url);
        Self {
            url: request.url,
            name: request.name,
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
        let Some(info) = self.extract_info(&self.url).await? else {
            return Ok(LiveStatus::Offline);
        };
        let archive_ids = self.archive_ids().await;
        let Some(entry) = self.select_entry(&info, &archive_ids) else {
            return Ok(LiveStatus::Offline);
        };
        let entry = self.resolve_entry(entry).await?;
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
                    download_archive: (!selection.is_live)
                        .then(|| std::path::PathBuf::from("archive.txt")),
                    extra_ytdlp_args: Vec::new(),
                })),
            }),
        })
    }

    async fn extract_info(&self, url: &str) -> LiveResult<Option<Value>> {
        let mut command = Command::new("yt-dlp");
        command
            .stdin(Stdio::null())
            .arg("--dump-single-json")
            .arg("--skip-download")
            .arg("--ignore-errors")
            .arg("--extractor-retries")
            .arg("0")
            .arg("--no-warnings");

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

    fn select_entry<'a>(
        &self,
        value: &'a Value,
        archive_ids: &HashSet<String>,
    ) -> Option<&'a Value> {
        let object = value.as_object()?;
        if self.entry_allowed(object, archive_ids) {
            return Some(value);
        }

        object
            .get("entries")
            .and_then(Value::as_array)
            .and_then(|entries| {
                entries
                    .iter()
                    .find_map(|entry| self.select_entry(entry, archive_ids))
            })
    }

    fn entry_allowed(&self, object: &Map<String, Value>, archive_ids: &HashSet<String>) -> bool {
        match object.get("live_status").and_then(Value::as_str) {
            Some("is_upcoming") => return false,
            Some("is_live") if !self.enable_download_live => return false,
            Some("was_live") if self.live_url || !self.enable_download_playback => return false,
            _ if self.live_url => return false,
            _ => {}
        }

        if object.get("_type").and_then(Value::as_str) == Some("playlist") {
            return false;
        }

        let is_live = object.get("live_status").and_then(Value::as_str) == Some("is_live");
        if !is_live
            && let Some(id) = object.get("id").and_then(Value::as_str)
            && archive_ids.contains(id)
        {
            return false;
        }

        self.date_allowed(object.get("upload_date"))
    }

    fn date_allowed(&self, upload_date: Option<&Value>) -> bool {
        if self.after_date.is_none() && self.before_date.is_none() {
            return true;
        }

        let Some(upload_date) = upload_date.and_then(Value::as_str) else {
            return false;
        };

        if let Some(after_date) = &self.after_date
            && upload_date < after_date.as_str()
        {
            return false;
        }
        if let Some(before_date) = &self.before_date
            && upload_date > before_date.as_str()
        {
            return false;
        }
        true
    }

    async fn resolve_entry(&self, entry: &Value) -> LiveResult<Value> {
        if entry.get("_type").and_then(Value::as_str) != Some("url") {
            return Ok(entry.clone());
        }

        let Some(url) = string_field(entry, &["webpage_url", "url"]) else {
            return Ok(entry.clone());
        };

        Ok(self
            .extract_info(&url)
            .await?
            .unwrap_or_else(|| entry.clone()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_youtube_urls() {
        let plugin = Youtube::new();

        assert!(plugin.matches("https://www.youtube.com/watch?v=test"));
        assert!(plugin.matches("https://m.youtube.com/watch?v=test"));
        assert!(plugin.matches("https://youtu.be/test"));
        assert!(!plugin.matches("https://example.com/watch?v=test"));
    }
}
