use super::{
    DanmakuSource, DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus,
    LiveStream, RuntimeOptions, YtDlpBackend, YtDlpOptions, media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
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
        // 列表型 URL 先用 --flat-playlist 轻量枚举（对齐 Python extract_info(process=False)），
        // 命中的条目再单条完整提取
        let Some(info) = self.extract_info(&self.url, true).await? else {
            return Ok(LiveStatus::Offline);
        };
        let archive_ids = self.archive_ids().await;
        let entry = match self.select_entry(&info, &archive_ids).await? {
            EntryDecision::Select(entry) => entry,
            EntryDecision::Skip | EntryDecision::Stop => return Ok(LiveStatus::Offline),
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
                    // 直播完成后 yt-dlp 也会写入 archive（对齐 youtube.py:312），
                    // 避免直播结束后同一视频被当作回放重复下载
                    download_archive: Some(std::path::PathBuf::from("archive.txt")),
                    extra_ytdlp_args: Vec::new(),
                })),
            }),
        })
    }

    async fn extract_info(&self, url: &str, flat: bool) -> LiveResult<Option<Value>> {
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

    /// 递归扫描播放列表，返回首个命中的条目（对齐 Python loop_entries）。
    /// Stop 只终止当前列表的扫描，向上层表现为未命中。
    fn select_entry<'a>(
        &'a self,
        value: &'a Value,
        archive_ids: &'a HashSet<String>,
    ) -> Pin<Box<dyn Future<Output = LiveResult<EntryDecision>> + Send + 'a>> {
        Box::pin(async move {
            let Some(object) = value.as_object() else {
                return Ok(EntryDecision::Skip);
            };

            if object.get("_type").and_then(Value::as_str) == Some("playlist") {
                let entries = object.get("entries").and_then(Value::as_array);
                for entry in entries.into_iter().flatten() {
                    match self.select_entry(entry, archive_ids).await? {
                        selected @ EntryDecision::Select(_) => return Ok(selected),
                        EntryDecision::Stop => return Ok(EntryDecision::Skip),
                        EntryDecision::Skip => {}
                    }
                }
                return Ok(EntryDecision::Skip);
            }

            self.evaluate_entry(value, object, archive_ids).await
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
        if candidate.get("upload_date").and_then(Value::as_str).is_none()
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

    async fn resolve_entry(&self, entry: Value) -> LiveResult<Value> {
        if entry.get("_type").and_then(Value::as_str) != Some("url") {
            return Ok(entry);
        }

        let Some(url) = string_field(&entry, &["webpage_url", "url"]) else {
            return Ok(entry);
        };

        // 命中的 flat 条目单条完整提取，获得下载所需字段（对齐 youtube.py:292-293）
        Ok(self.extract_info(&url, false).await?.unwrap_or(entry))
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
    use serde_json::json;

    fn youtube_live(
        live_url: bool,
        enable_download_live: bool,
        enable_download_playback: bool,
    ) -> YoutubeLive {
        YoutubeLive {
            url: "https://www.youtube.com/@test".to_string(),
            name: "test".to_string(),
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
        assert_eq!(live.date_decision(json!({}), "20231231"), EntryDecision::Stop);
        // 晚于 before_date：跳过当前条目继续扫描
        assert_eq!(live.date_decision(json!({}), "20250101"), EntryDecision::Skip);
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

        match live.select_entry(&info, &archive).await.unwrap() {
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
            live.select_entry(&info, &HashSet::new()).await.unwrap(),
            EntryDecision::Skip
        );
    }
}
