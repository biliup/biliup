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
use serde_json::Value;
use std::collections::HashMap;

const CC_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct CC {
    re: Regex,
}

impl Default for CC {
    fn default() -> Self {
        Self::new()
    }
}

impl CC {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"https?://cc\.163\.com").unwrap(),
        }
    }
}

impl DownloadPlugin for CC {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    fn create_downloader(&self, ctx: &mut PluginContext) -> Box<dyn DownloadBase> {
        Box::new(CCDownloader::new(
            ctx.client(),
            ctx.live_streamer().url.clone(),
            ctx.live_streamer().remark.clone(),
            ctx.config()
                .cc_protocol
                .clone()
                .unwrap_or_else(|| "hls".to_string()),
        ))
    }

    fn name(&self) -> &str {
        "CC"
    }
}

struct CCDownloader {
    client: Client,
    url: String,
    name: String,
    protocol: String,
}

impl CCDownloader {
    fn new(client: Client, url: String, name: String, protocol: String) -> Self {
        Self {
            client,
            url,
            name,
            protocol,
        }
    }

    fn room_id(&self) -> Result<String, Report<AppError>> {
        Regex::new(r"(\d{4,})")
            .unwrap()
            .captures(&self.url)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| Report::new(AppError::Custom("网易 CC 直播间地址错误".to_string())))
    }

    async fn channel_id(&self, room_id: &str) -> Result<Option<String>, Report<AppError>> {
        let value: Value = self
            .client
            .get(format!(
                "https://api.cc.163.com/v1/activitylives/anchor/lives?anchor_ccid={room_id}"
            ))
            .header(reqwest::header::USER_AGENT, CC_USER_AGENT)
            .send()
            .await
            .change_context(AppError::Custom("获取网易 CC 直播间信息失败".to_string()))?
            .json()
            .await
            .change_context(AppError::Custom("解析网易 CC 直播间信息失败".to_string()))?;

        let Some(room) = value["data"].get(room_id) else {
            return Ok(None);
        };
        if room.as_object().map(|object| object.len()).unwrap_or(0) <= 1 {
            return Ok(None);
        }
        Ok(room["channel_id"].as_i64().map(|id| id.to_string()))
    }

    async fn channel_info(&self, channel_id: &str) -> Result<Value, Report<AppError>> {
        let value: Value = self
            .client
            .get(format!(
                "https://cc.163.com/live/channel/?channelids={channel_id}"
            ))
            .header(reqwest::header::USER_AGENT, CC_USER_AGENT)
            .send()
            .await
            .change_context(AppError::Custom("获取网易 CC 频道信息失败".to_string()))?
            .json()
            .await
            .change_context(AppError::Custom("解析网易 CC 频道信息失败".to_string()))?;

        value["data"]
            .as_array()
            .and_then(|data| data.first())
            .cloned()
            .ok_or_else(|| Report::new(AppError::Custom("网易 CC 频道信息为空".to_string())))
    }

    fn stream_url(&self, channel: &Value) -> Result<String, Report<AppError>> {
        if self.protocol == "hls" {
            return channel["sharefile"]
                .as_str()
                .filter(|url| !url.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    Report::new(AppError::Custom("网易 CC HLS 直播流为空".to_string()))
                });
        }

        channel["quickplay"]["resolution"]
            .as_object()
            .and_then(|resolutions| {
                resolutions
                    .values()
                    .max_by_key(|level| level["vbr"].as_i64().unwrap_or(0))
            })
            .and_then(|level| level["cdn"].as_object())
            .and_then(|cdn| cdn.values().find_map(Value::as_str))
            .filter(|url| !url.is_empty())
            .map(str::to_string)
            .ok_or_else(|| Report::new(AppError::Custom("网易 CC 直播流为空".to_string())))
    }
}

#[async_trait]
impl DownloadBase for CCDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        let room_id = self.room_id()?;
        let Some(channel_id) = self.channel_id(&room_id).await? else {
            return Ok(StreamStatus::Offline);
        };
        let channel = self.channel_info(&channel_id).await?;
        let raw_stream_url = self.stream_url(&channel)?;
        let title = channel["title"]
            .as_str()
            .filter(|title| !title.is_empty())
            .unwrap_or(&room_id)
            .to_string();

        Ok(StreamStatus::Live {
            stream_info: Box::new(StreamInfoExt {
                streamer_info: StreamerInfo {
                    id: -1,
                    name: self.name.clone(),
                    url: self.url.clone(),
                    title,
                    date: Utc::now(),
                    live_cover_path: String::new(),
                },
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| {
                    if self.protocol == "hls" {
                        "m3u8".to_string()
                    } else {
                        "flv".to_string()
                    }
                }),
                raw_stream_url,
                platform: "cc".to_string(),
                stream_headers: HashMap::new(),
            }),
        })
    }
}
