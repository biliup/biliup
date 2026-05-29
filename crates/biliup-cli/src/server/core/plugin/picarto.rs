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

const PICARTO_API_CHANNEL: &str = "https://ptvintern.picarto.tv/api/channel/detail";
const PICARTO_HLS_URL: &str = "https://{origin}.picarto.tv/stream/hls/{stream_name}/index.m3u8";

pub struct Picarto {
    re: Regex,
}

impl Default for Picarto {
    fn default() -> Self {
        Self::new()
    }
}

impl Picarto {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"(?:https?://)?(?:www\.)?picarto\.tv/(?P<id>[^/?&]+)").unwrap(),
        }
    }
}

impl DownloadPlugin for Picarto {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    fn create_downloader(&self, ctx: &mut PluginContext) -> Box<dyn DownloadBase> {
        Box::new(PicartoDownloader::new(
            ctx.client(),
            ctx.live_streamer().url.clone(),
            ctx.live_streamer().remark.clone(),
        ))
    }

    fn name(&self) -> &str {
        "Picarto"
    }
}

struct PicartoDownloader {
    client: Client,
    url: String,
    name: String,
}

impl PicartoDownloader {
    fn new(client: Client, url: String, name: String) -> Self {
        Self { client, url, name }
    }

    fn username(&self) -> Result<String, Report<AppError>> {
        Regex::new(r"(?:https?://)?(?:www\.)?picarto\.tv/([^/?&]+)")
            .unwrap()
            .captures(&self.url)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| Report::new(AppError::Custom("Picarto 直播间地址错误".to_string())))
    }

    async fn channel_detail(&self, username: &str) -> Result<PicartoResponse, Report<AppError>> {
        self.client
            .get(format!("{PICARTO_API_CHANNEL}/{username}"))
            .send()
            .await
            .change_context(AppError::Custom("获取 Picarto 直播间信息失败".to_string()))?
            .json()
            .await
            .change_context(AppError::Custom("解析 Picarto 直播间信息失败".to_string()))
    }
}

#[async_trait]
impl DownloadBase for PicartoDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        let username = self.username()?;
        let response = self.channel_detail(&username).await?;
        let Some(channel) = response.channel else {
            return Ok(StreamStatus::Offline);
        };
        if channel.private {
            return Ok(StreamStatus::Offline);
        }

        let Some(loadbalancer) = response.get_load_balancer_url else {
            return Ok(StreamStatus::Offline);
        };
        let Some(multistreams) = response.get_multi_streams else {
            return Ok(StreamStatus::Offline);
        };
        let Some(stream) = multistreams
            .streams
            .into_iter()
            .find(|stream| stream.channel_id == channel.id)
        else {
            return Ok(StreamStatus::Offline);
        };

        let raw_stream_url = PICARTO_HLS_URL
            .replace("{origin}", &loadbalancer.origin)
            .replace("{stream_name}", &stream.stream_name);
        let title = channel.title.unwrap_or(username);

        Ok(StreamStatus::Live {
            stream_info: Box::new(StreamInfoExt {
                streamer_info: StreamerInfo {
                    id: -1,
                    name: self.name.clone(),
                    url: self.url.clone(),
                    title,
                    date: Utc::now(),
                    live_cover_path: stream.thumbnail_image.unwrap_or_default(),
                },
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "m3u8".to_string()),
                raw_stream_url,
                platform: "picarto".to_string(),
                stream_headers: HashMap::new(),
            }),
        })
    }
}

#[derive(Deserialize)]
struct PicartoResponse {
    channel: Option<PicartoChannel>,
    #[serde(rename = "getLoadBalancerUrl")]
    get_load_balancer_url: Option<PicartoLoadBalancer>,
    #[serde(rename = "getMultiStreams")]
    get_multi_streams: Option<PicartoMultiStreams>,
}

#[derive(Deserialize)]
struct PicartoChannel {
    id: i64,
    #[serde(default)]
    private: bool,
    title: Option<String>,
}

#[derive(Deserialize)]
struct PicartoLoadBalancer {
    origin: String,
}

#[derive(Deserialize)]
struct PicartoMultiStreams {
    #[serde(default)]
    streams: Vec<PicartoStream>,
}

#[derive(Deserialize)]
struct PicartoStream {
    #[serde(rename = "channelId")]
    channel_id: i64,
    stream_name: String,
    thumbnail_image: Option<String>,
}
