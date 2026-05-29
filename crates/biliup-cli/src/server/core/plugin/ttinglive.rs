use crate::server::common::util::media_ext_from_url;
use crate::server::core::plugin::{DownloadBase, DownloadPlugin, StreamInfoExt, StreamStatus};
use crate::server::errors::AppError;
use crate::server::infrastructure::context::PluginContext;
use crate::server::infrastructure::models::StreamerInfo;
use async_trait::async_trait;
use chrono::Utc;
use error_stack::{Report, ResultExt};
use regex::Regex;
use reqwest::Client;
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

impl DownloadPlugin for TTingLive {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    fn create_downloader(&self, ctx: &mut PluginContext) -> Box<dyn DownloadBase> {
        Box::new(TTingLiveDownloader::new(
            ctx.client(),
            ctx.live_streamer().url.clone(),
            ctx.live_streamer().remark.clone(),
        ))
    }

    fn name(&self) -> &str {
        "TTingLive"
    }
}

struct TTingLiveDownloader {
    client: Client,
    url: String,
    name: String,
}

impl TTingLiveDownloader {
    fn new(client: Client, url: String, name: String) -> Self {
        Self { client, url, name }
    }

    fn room_id(&self) -> Result<String, Report<AppError>> {
        Regex::new(r"/channels/(\d+)/live")
            .unwrap()
            .captures(&self.url)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| Report::new(AppError::Custom("TTingLive 直播间地址错误".to_string())))
    }

    async fn room_info(
        &self,
        room_id: &str,
    ) -> Result<Option<TTingLiveResponse>, Report<AppError>> {
        let response = self
            .client
            .get(format!(
                "https://api.ttinglive.com/api/channels/{room_id}/stream?option=all"
            ))
            .header(reqwest::header::USER_AGENT, TTINGLIVE_USER_AGENT)
            .send()
            .await
            .change_context(AppError::Custom(
                "获取 TTingLive 直播间信息失败".to_string(),
            ))?;

        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(Report::new(AppError::Custom(format!(
                "获取 TTingLive 直播间信息失败: {}",
                response.status()
            ))));
        }

        response
            .json()
            .await
            .map(Some)
            .change_context(AppError::Custom(
                "解析 TTingLive 直播间信息失败".to_string(),
            ))
    }

    async fn best_variant_url(&self, playlist_url: &str) -> Result<String, Report<AppError>> {
        let text = self
            .client
            .get(playlist_url)
            .header(reqwest::header::USER_AGENT, TTINGLIVE_USER_AGENT)
            .send()
            .await
            .change_context(AppError::Custom("获取 TTingLive 播放列表失败".to_string()))?
            .text()
            .await
            .change_context(AppError::Custom("读取 TTingLive 播放列表失败".to_string()))?;

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

        best_url
            .ok_or_else(|| Report::new(AppError::Custom("TTingLive 播放列表解析失败".to_string())))
    }
}

#[async_trait]
impl DownloadBase for TTingLiveDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        let room_id = self.room_id()?;
        let Some(info) = self.room_info(&room_id).await? else {
            return Ok(StreamStatus::Offline);
        };
        let source_url = info
            .sources
            .first()
            .map(|source| source.url.clone())
            .filter(|url| !url.is_empty())
            .ok_or_else(|| Report::new(AppError::Custom("TTingLive 直播流为空".to_string())))?;
        let raw_stream_url = self.best_variant_url(&source_url).await?;

        Ok(StreamStatus::Live {
            stream_info: Box::new(StreamInfoExt {
                streamer_info: StreamerInfo {
                    id: -1,
                    name: self.name.clone(),
                    url: self.url.clone(),
                    title: info.title.unwrap_or(room_id),
                    date: Utc::now(),
                    live_cover_path: info.thumb_url.unwrap_or_default(),
                },
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "m3u8".to_string()),
                raw_stream_url,
                platform: "ttinglive".to_string(),
                stream_headers: HashMap::new(),
            }),
        })
    }
}

fn resolve_url(base: &str, value: &str) -> Result<String, Report<AppError>> {
    if value.starts_with("http://") || value.starts_with("https://") {
        return Ok(value.to_string());
    }

    Url::parse(base)
        .and_then(|url| url.join(value))
        .map(|url| url.to_string())
        .change_context(AppError::Custom(
            "解析 TTingLive 播放列表地址失败".to_string(),
        ))
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
