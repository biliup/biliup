use super::{
    DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus, LiveStream,
    RuntimeOptions, YtDlpBackend, YtDlpOptions, media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;

pub struct General {
    re: Regex,
}

impl Default for General {
    fn default() -> Self {
        Self::new()
    }
}

impl General {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"https?://").unwrap(),
        }
    }
}

#[async_trait]
impl LivePlugin for General {
    fn name(&self) -> &'static str {
        "General"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        GeneralLive::new(request).check_stream().await
    }
}

struct GeneralLive {
    url: String,
    name: String,
}

#[derive(Clone)]
struct GeneralSelection {
    webpage_url: String,
    download_url: Option<String>,
    title: String,
    thumbnail: String,
    raw_stream_url: String,
    suffix: String,
    is_live: bool,
    use_stream_url: bool,
}

impl GeneralLive {
    fn new(request: LiveRequest) -> Self {
        Self {
            url: request.url,
            name: request.name,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        let info = self.extract_info(&self.url).await?;
        let selection = if let Some(info) = info
            && let Some(entry) = self.select_entry(&info)
        {
            let entry = self.resolve_entry(entry).await?;
            self.selection(&entry)
        } else if let Some(raw_stream_url) = self.streamlink_url().await {
            GeneralSelection {
                webpage_url: self.url.clone(),
                download_url: Some(self.url.clone()),
                title: self.name.clone(),
                thumbnail: String::new(),
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "flv".to_string()),
                raw_stream_url,
                is_live: true,
                use_stream_url: true,
            }
        } else {
            return Ok(LiveStatus::Offline);
        };

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: selection.title.clone(),
                date: Utc::now(),
                live_cover_url: selection.thumbnail.clone(),
                raw_stream_url: selection.raw_stream_url.clone(),
                platform: "general".to_string(),
                stream_headers: HashMap::new(),
                suffix: selection.suffix.clone(),
                danmaku: None,
                downloader_hint: if selection.use_stream_url {
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
                    cookies_file: None,
                    prefer_vcodec: None,
                    prefer_acodec: None,
                    max_filesize: None,
                    max_height: None,
                    download_archive: None,
                    extra_ytdlp_args: Vec::new(),
                })),
            }),
        })
    }

    async fn extract_info(&self, url: &str) -> LiveResult<Option<Value>> {
        let output = Command::new("yt-dlp")
            .stdin(Stdio::null())
            .arg("--dump-single-json")
            .arg("--skip-download")
            .arg("--ignore-errors")
            .arg("--extractor-retries")
            .arg("0")
            .arg("--no-warnings")
            .arg(url)
            .output()
            .await
            .map_err(|err| {
                LiveError::custom(format!("运行 yt-dlp 失败，请确认已安装并在 PATH 中: {err}"))
            })?;

        if !output.status.success() {
            return Ok(None);
        }

        parse_ytdlp_stdout(&output.stdout)
    }

    async fn streamlink_url(&self) -> Option<String> {
        let output = Command::new("streamlink")
            .stdin(Stdio::null())
            .arg("--stream-url")
            .arg(&self.url)
            .arg("best")
            .output()
            .await
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .next()
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .map(str::to_string)
    }

    fn select_entry<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        let object = value.as_object()?;
        if self.entry_allowed(value) {
            return Some(value);
        }

        object
            .get("entries")
            .and_then(Value::as_array)
            .and_then(|entries| entries.iter().find_map(|entry| self.select_entry(entry)))
    }

    fn entry_allowed(&self, entry: &Value) -> bool {
        if entry.get("live_status").and_then(Value::as_str) == Some("is_upcoming") {
            return false;
        }
        if entry.get("_type").and_then(Value::as_str) == Some("playlist") {
            return false;
        }

        string_field(entry, &["webpage_url", "original_url", "url"]).is_some()
            || entry.get("formats").and_then(Value::as_array).is_some()
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

    fn selection(&self, entry: &Value) -> GeneralSelection {
        let webpage_url = string_field(entry, &["webpage_url", "original_url", "url"])
            .unwrap_or_else(|| self.url.clone());
        let title =
            string_field(entry, &["fulltitle", "title"]).unwrap_or_else(|| self.name.clone());
        let thumbnail = string_field(entry, &["thumbnail"]).unwrap_or_default();
        let is_live = entry.get("live_status").and_then(Value::as_str) == Some("is_live");
        let stream_url = is_live.then(|| self.stream_url(entry)).flatten();
        let use_stream_url = stream_url.is_some();
        let raw_stream_url = stream_url.unwrap_or_else(|| webpage_url.clone());
        let suffix = if use_stream_url {
            media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "flv".to_string())
        } else {
            "mp4".to_string()
        };

        GeneralSelection {
            webpage_url: webpage_url.clone(),
            download_url: Some(webpage_url),
            title,
            thumbnail,
            raw_stream_url,
            suffix,
            is_live,
            use_stream_url,
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
}

fn parse_ytdlp_stdout(stdout: &[u8]) -> LiveResult<Option<Value>> {
    let stdout = String::from_utf8_lossy(stdout);
    let stdout = stdout.trim();
    if stdout.is_empty() || stdout == "null" {
        return Ok(None);
    }
    serde_json::from_str(stdout)
        .map(Some)
        .map_err(|err| LiveError::custom(format!("解析通用下载信息失败: {err}")))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_http_urls() {
        let plugin = General::new();

        assert!(plugin.matches("https://example.com/live"));
        assert!(plugin.matches("http://example.com/video"));
        assert!(!plugin.matches("rtmp://example.com/live"));
    }
}
