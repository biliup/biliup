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

const BIGO_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct Bigo {
    re: Regex,
}

impl Default for Bigo {
    fn default() -> Self {
        Self::new()
    }
}

impl Bigo {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"(?:https?://)?www\.bigo\.tv").unwrap(),
        }
    }
}

impl DownloadPlugin for Bigo {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    fn create_downloader(&self, ctx: &mut PluginContext) -> Box<dyn DownloadBase> {
        Box::new(BigoDownloader::new(
            ctx.client(),
            ctx.live_streamer().url.clone(),
            ctx.live_streamer().remark.clone(),
        ))
    }

    fn name(&self) -> &str {
        "Bigo"
    }
}

struct BigoDownloader {
    client: Client,
    url: String,
    name: String,
}

impl BigoDownloader {
    fn new(client: Client, url: String, name: String) -> Self {
        Self { client, url, name }
    }

    fn room_id(&self) -> Result<String, Report<AppError>> {
        let room_id = self
            .url
            .split('/')
            .next_back()
            .and_then(|value| value.split('?').next())
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        room_id.ok_or_else(|| Report::new(AppError::Custom("Bigo 直播间地址错误".to_string())))
    }

    async fn room_info(&self, room_id: &str) -> Result<BigoResponse, Report<AppError>> {
        self.client
            .post("https://ta.bigo.tv/official_website/studio/getInternalStudioInfo")
            .header(reqwest::header::USER_AGENT, BIGO_USER_AGENT)
            .header(reqwest::header::ACCEPT, "application/json")
            .form(&[("siteId", room_id)])
            .send()
            .await
            .change_context(AppError::Custom("获取 Bigo 直播间信息失败".to_string()))?
            .json()
            .await
            .change_context(AppError::Custom("解析 Bigo 直播间信息失败".to_string()))
    }
}

#[async_trait]
impl DownloadBase for BigoDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        let room_id = self.room_id()?;
        let response = self.room_info(&room_id).await?;
        if response.code != 0 {
            return Ok(StreamStatus::Offline);
        }
        let Some(data) = response.data else {
            return Ok(StreamStatus::Offline);
        };
        if data.alive != Some(1) {
            return Ok(StreamStatus::Offline);
        }

        let Some(raw_stream_url) = data.hls_src.filter(|url| !url.is_empty()) else {
            return Ok(StreamStatus::Offline);
        };

        Ok(StreamStatus::Live {
            stream_info: Box::new(StreamInfoExt {
                streamer_info: StreamerInfo {
                    id: -1,
                    name: self.name.clone(),
                    url: self.url.clone(),
                    title: data.room_topic.unwrap_or(room_id),
                    date: Utc::now(),
                    live_cover_path: String::new(),
                },
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "m3u8".to_string()),
                raw_stream_url,
                platform: "bigo".to_string(),
                stream_headers: HashMap::new(),
            }),
        })
    }
}

#[derive(Deserialize)]
struct BigoResponse {
    code: i32,
    data: Option<BigoData>,
}

#[derive(Deserialize)]
struct BigoData {
    alive: Option<i32>,
    hls_src: Option<String>,
    #[serde(rename = "roomTopic")]
    room_topic: Option<String>,
}
