use super::{
    DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus, LiveStream,
    media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use reqwest::StatusCode;
use serde::Deserialize;
use std::collections::HashMap;
use url::Url;

const TTINGLIVE_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct TTingLive {
    re: Regex,
}

impl Default for TTingLive {
    fn default() -> Self {
        Self::new()
    }
}

impl TTingLive {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"(?:https?://)?www\.ttinglive\.com").unwrap(),
        }
    }
}

#[async_trait]
impl LivePlugin for TTingLive {
    fn name(&self) -> &'static str {
        "TTingLive"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        TTingLiveLive::new(request).check_stream().await
    }
}

struct TTingLiveLive {
    client: reqwest::Client,
    url: String,
    name: String,
}

impl TTingLiveLive {
    fn new(request: LiveRequest) -> Self {
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        let room_id = self.room_id()?;
        let Some(info) = self.room_info(&room_id).await? else {
            return Ok(LiveStatus::Offline);
        };
        let source_url = info
            .sources
            .first()
            .map(|source| source.url.clone())
            .filter(|url| !url.is_empty())
            .ok_or_else(|| LiveError::custom("TTingLive 直播流为空"))?;
        let raw_stream_url = self.best_variant_url(&source_url).await?;

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: info.title.unwrap_or(room_id),
                date: Utc::now(),
                live_cover_url: info.thumb_url.unwrap_or_default(),
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "m3u8".to_string()),
                raw_stream_url,
                platform: "ttinglive".to_string(),
                stream_headers: HashMap::new(),
                danmaku: None,
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }

    fn room_id(&self) -> LiveResult<String> {
        Regex::new(r"/channels/(\d+)/live")
            .unwrap()
            .captures(&self.url)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| LiveError::custom("TTingLive 直播间地址错误"))
    }

    async fn room_info(&self, room_id: &str) -> LiveResult<Option<TTingLiveResponse>> {
        let response = self
            .client
            .get(format!(
                "https://api.ttinglive.com/api/channels/{room_id}/stream?option=all"
            ))
            .header(reqwest::header::USER_AGENT, TTINGLIVE_USER_AGENT)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取 TTingLive 直播间信息失败: {err}")))?;

        if response.status() == StatusCode::BAD_REQUEST {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(LiveError::custom(format!(
                "获取 TTingLive 直播间信息失败: {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map(Some)
            .map_err(|err| LiveError::custom(format!("解析 TTingLive 直播间信息失败: {err}")))
    }

    async fn best_variant_url(&self, playlist_url: &str) -> LiveResult<String> {
        let text = self
            .client
            .get(playlist_url)
            .header(reqwest::header::USER_AGENT, TTINGLIVE_USER_AGENT)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取 TTingLive 播放列表失败: {err}")))?
            .text()
            .await
            .map_err(|err| LiveError::custom(format!("读取 TTingLive 播放列表失败: {err}")))?;

        let mut best_bandwidth = 0_u64;
        let mut best_url = None;
        let mut last_bandwidth = None;

        for line in text.lines().map(str::trim) {
            if let Some(attributes) = line.strip_prefix("#EXT-X-STREAM-INF:") {
                last_bandwidth = Regex::new(r"BANDWIDTH=(\d+)")
                    .unwrap()
                    .captures(attributes)
                    .and_then(|captures| captures[1].parse::<u64>().ok());
                continue;
            }
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some(bandwidth) = last_bandwidth.take() else {
                continue;
            };
            if bandwidth > best_bandwidth {
                best_bandwidth = bandwidth;
                best_url = Some(resolve_url(playlist_url, line)?);
            }
        }

        best_url.ok_or_else(|| LiveError::custom("TTingLive 播放列表解析失败"))
    }
}

fn resolve_url(base: &str, value: &str) -> LiveResult<String> {
    if value.starts_with("http://") || value.starts_with("https://") {
        return Ok(value.to_string());
    }

    Url::parse(base)
        .and_then(|url| url.join(value))
        .map(|url| url.to_string())
        .map_err(|err| LiveError::custom(format!("解析 TTingLive 播放列表地址失败: {err}")))
}

#[derive(Deserialize)]
struct TTingLiveResponse {
    title: Option<String>,
    #[serde(rename = "thumbUrl")]
    thumb_url: Option<String>,
    #[serde(default)]
    sources: Vec<TTingLiveSource>,
}

#[derive(Deserialize)]
struct TTingLiveSource {
    url: String,
}
