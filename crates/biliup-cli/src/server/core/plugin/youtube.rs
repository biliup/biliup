use crate::server::common::util::{danmaku_filename_template, media_ext_from_url};
use crate::server::core::downloader::ytdlp::{
    Backend as YtDlpBackend, DownloadConfig as YtDlpConfig, YouTubeDownloader as YtDlpDownloader,
};
use crate::server::core::downloader::{
    DanmakuClient, DownloaderRuntime, DownloaderType, RustDanmakuClient,
};
use crate::server::core::plugin::{DownloadBase, DownloadPlugin, StreamInfoExt, StreamStatus};
use crate::server::errors::AppError;
use crate::server::infrastructure::context::PluginContext;
use crate::server::infrastructure::models::StreamerInfo;
use async_trait::async_trait;
use chrono::Utc;
use danmaku_client::RecorderConfig;
use error_stack::{Report, ResultExt};
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
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

impl DownloadPlugin for Youtube {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    fn create_downloader(&self, ctx: &mut PluginContext) -> Box<dyn DownloadBase> {
        let config = ctx.config();
        let user = config.user.clone().unwrap_or_default();
        Box::new(YoutubeDownloader::new(
            ctx.live_streamer().url.clone(),
            ctx.live_streamer().remark.clone(),
            user.youtube_cookie,
            config.youtube_enable_download_live.unwrap_or(true),
            config.youtube_enable_download_playback.unwrap_or(true),
            config.youtube_after_date,
            config.youtube_before_date,
            config.youtube_prefer_vcodec,
            config.youtube_prefer_acodec,
            config.youtube_max_resolution,
            config.youtube_max_videosize,
            config
                .youtube_danmaku
                .or(config.ytb_danmaku)
                .unwrap_or(false),
            ctx.live_streamer()
                .filename_prefix
                .clone()
                .or(config.filename_prefix.clone()),
        ))
    }

    fn name(&self) -> &str {
        "Youtube"
    }
}

struct YoutubeDownloader {
    url: String,
    name: String,
    cookie_file: Option<PathBuf>,
    live_url: bool,
    enable_download_live: bool,
    enable_download_playback: bool,
    after_date: Option<String>,
    before_date: Option<String>,
    prefer_vcodec: Option<String>,
    prefer_acodec: Option<String>,
    max_resolution: Option<u32>,
    max_videosize: Option<String>,
    danmaku: Option<Arc<dyn DanmakuClient + Send + Sync>>,
    selected: Option<YoutubeSelection>,
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

impl YoutubeDownloader {
    fn new(
        url: String,
        name: String,
        cookie_file: Option<PathBuf>,
        enable_download_live: bool,
        enable_download_playback: bool,
        after_date: Option<String>,
        before_date: Option<String>,
        prefer_vcodec: Option<String>,
        prefer_acodec: Option<String>,
        max_resolution: Option<u32>,
        max_videosize: Option<String>,
        youtube_danmaku: bool,
        filename_prefix: Option<String>,
    ) -> Self {
        let live_url = is_youtube_live_url(&url);
        let danmaku = youtube_danmaku.then(|| {
            Arc::new(RustDanmakuClient::new(RecorderConfig::new(
                url.clone(),
                PathBuf::from(danmaku_filename_template(filename_prefix.as_deref(), &name)),
            ))) as Arc<dyn DanmakuClient + Send + Sync>
        });

        Self {
            url,
            name,
            cookie_file,
            live_url,
            enable_download_live,
            enable_download_playback,
            after_date,
            before_date,
            prefer_vcodec,
            prefer_acodec,
            max_resolution,
            max_videosize,
            danmaku,
            selected: None,
        }
    }

    async fn extract_info(&self, url: &str) -> Result<Option<Value>, Report<AppError>> {
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

        let output = command
            .arg(url)
            .output()
            .await
            .change_context(AppError::Custom(
                "运行 yt-dlp 失败，请确认已安装并在 PATH 中".to_string(),
            ))?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stdout = stdout.trim();
        if stdout.is_empty() || stdout == "null" {
            return Ok(None);
        }

        serde_json::from_str(stdout)
            .map(Some)
            .change_context(AppError::Custom("解析 YouTube 信息失败".to_string()))
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

    async fn resolve_entry(&self, entry: &Value) -> Result<Value, Report<AppError>> {
        if entry.get("_type").and_then(Value::as_str) != Some("url") {
            return Ok(entry.clone());
        }

        let Some(url) = Self::string_field(entry, &["webpage_url", "url"]) else {
            return Ok(entry.clone());
        };

        Ok(self
            .extract_info(&url)
            .await?
            .unwrap_or_else(|| entry.clone()))
    }

    fn selection(&self, entry: &Value) -> YoutubeSelection {
        let webpage_url = Self::string_field(entry, &["webpage_url", "original_url", "url"])
            .unwrap_or_else(|| self.url.clone());
        let title =
            Self::string_field(entry, &["fulltitle", "title"]).unwrap_or_else(|| self.name.clone());
        let thumbnail = Self::string_field(entry, &["thumbnail"]).unwrap_or_default();
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

    fn ytdlp_runtime(&self, downloader_type: DownloaderType) -> DownloaderRuntime {
        let selection = self.selected.as_ref().cloned().unwrap_or(YoutubeSelection {
            webpage_url: self.url.clone(),
            download_url: Some(self.url.clone()),
            title: self.name.clone(),
            thumbnail: String::new(),
            stream_url: None,
            is_live: false,
        });

        let cfg = YtDlpConfig {
            webpage_url: selection.webpage_url,
            download_url: selection.download_url,
            cookies_file: self.cookie_file.clone(),
            prefer_vcodec: self.prefer_vcodec.clone(),
            prefer_acodec: self.prefer_acodec.clone(),
            max_filesize: self.max_videosize.clone(),
            max_height: self.max_resolution,
            backend: if downloader_type == DownloaderType::Ytarchive {
                YtDlpBackend::YtArchive
            } else {
                YtDlpBackend::YtDlp
            },
            is_live: selection.is_live,
            use_live_cover: false,
            cover_url: (!selection.thumbnail.is_empty()).then_some(selection.thumbnail),
            download_archive: (!selection.is_live).then(|| PathBuf::from("archive.txt")),
            ..Default::default()
        };

        DownloaderRuntime::YtDlp(YtDlpDownloader::new(cfg))
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
}

#[async_trait]
impl DownloadBase for YoutubeDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        let Some(info) = self.extract_info(&self.url).await? else {
            return Ok(StreamStatus::Offline);
        };
        let archive_ids = self.archive_ids().await;
        let Some(entry) = self.select_entry(&info, &archive_ids) else {
            return Ok(StreamStatus::Offline);
        };
        let entry = self.resolve_entry(entry).await?;
        let selection = self.selection(&entry);
        if selection.is_live && selection.stream_url.is_none() {
            return Ok(StreamStatus::Offline);
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

        self.selected = Some(selection.clone());
        Ok(StreamStatus::Live {
            stream_info: Box::new(StreamInfoExt {
                streamer_info: StreamerInfo {
                    id: -1,
                    name: self.name.clone(),
                    url: self.url.clone(),
                    title: selection.title,
                    date: Utc::now(),
                    live_cover_path: selection.thumbnail,
                },
                suffix,
                raw_stream_url,
                platform: "youtube".to_string(),
                stream_headers: HashMap::new(),
            }),
        })
    }

    fn downloader(&self, downloader_type: DownloaderType) -> DownloaderRuntime {
        let Some(selection) = &self.selected else {
            return DownloaderRuntime::from_type(downloader_type);
        };

        if !selection.is_live
            || matches!(
                downloader_type,
                DownloaderType::YtDlp | DownloaderType::Ytarchive
            )
        {
            return self.ytdlp_runtime(downloader_type);
        }

        DownloaderRuntime::from_type(downloader_type)
    }

    fn danmaku_init(&self) -> Option<Arc<dyn DanmakuClient + Send + Sync>> {
        self.selected
            .as_ref()
            .filter(|selection| selection.is_live)
            .and_then(|_| self.danmaku.clone())
    }
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
