use super::{
    DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus, LiveStream,
    media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
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

#[async_trait]
impl LivePlugin for Picarto {
    fn name(&self) -> &'static str {
        "Picarto"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        PicartoLive::new(request).check_stream().await
    }
}

struct PicartoLive {
    client: reqwest::Client,
    url: String,
    name: String,
}

impl PicartoLive {
    fn new(request: LiveRequest) -> Self {
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        let username = self.username()?;
        let response = self.channel_detail(&username).await?;
        let Some(channel) = response.channel else {
            return Ok(LiveStatus::Offline);
        };
        if channel.private {
            return Ok(LiveStatus::Offline);
        }

        let Some(loadbalancer) = response.get_load_balancer_url else {
            return Ok(LiveStatus::Offline);
        };
        let Some(multistreams) = response.get_multi_streams else {
            return Ok(LiveStatus::Offline);
        };
        let Some(stream) = multistreams
            .streams
            .into_iter()
            .find(|stream| stream.channel_id == channel.id)
        else {
            return Ok(LiveStatus::Offline);
        };

        let raw_stream_url = PICARTO_HLS_URL
            .replace("{origin}", &loadbalancer.origin)
            .replace("{stream_name}", &stream.stream_name);
        let title = channel.title.unwrap_or(username);

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title,
                date: Utc::now(),
                live_cover_url: stream.thumbnail_image.unwrap_or_default(),
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "m3u8".to_string()),
                raw_stream_url,
                platform: "picarto".to_string(),
                stream_headers: HashMap::new(),
                danmaku: None,
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }

    fn username(&self) -> LiveResult<String> {
        Regex::new(r"(?:https?://)?(?:www\.)?picarto\.tv/([^/?&]+)")
            .unwrap()
            .captures(&self.url)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| LiveError::custom("Picarto 直播间地址错误"))
    }

    async fn channel_detail(&self, username: &str) -> LiveResult<PicartoResponse> {
        self.client
            .get(format!("{PICARTO_API_CHANNEL}/{username}"))
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取 Picarto 直播间信息失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析 Picarto 直播间信息失败: {err}")))
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
