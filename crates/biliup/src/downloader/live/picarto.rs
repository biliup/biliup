use super::{
    BatchCheckRequest, DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus,
    LiveStream, media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;

const PICARTO_API_CHANNEL: &str = "https://ptvintern.picarto.tv/api/channel/detail";
const PICARTO_HLS_URL: &str = "https://{origin}.picarto.tv/stream/hls/{stream_name}/index.m3u8";
/// 批量检测的 explore 接口：按观看人数倒序、每页 100 条、含成人内容。
const PICARTO_API_EXPLORE: &str = "https://ptvintern.picarto.tv/api/explore?first=100&page={page}&filter_params%5Badult%5D=true&order_by%5Bfield%5D=viewers&order_by%5Border%5D=DESC&type=stream";

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

    fn supports_batch_check(&self) -> bool {
        true
    }

    /// 批量检测：分页拉取 explore 列表（当前正在直播的频道），
    /// 返回其中用户名命中待检测 URL 的那些 URL（对齐 picarto.py:61-78）。
    async fn batch_check(&self, request: BatchCheckRequest) -> LiveResult<Vec<String>> {
        // 用户名（小写）-> 原始待检测 URL，便于用 explore 结果反查
        let mut wanted: HashMap<String, String> = HashMap::new();
        for url in &request.urls {
            if let Some(name) = self.username_of(url) {
                wanted.insert(name.to_lowercase(), url.clone());
            }
        }
        if wanted.is_empty() {
            return Ok(Vec::new());
        }

        let mut live_urls = Vec::new();
        let mut page = 1u32;
        loop {
            let explore: PicartoExplore = request
                .client
                .get(PICARTO_API_EXPLORE.replace("{page}", &page.to_string()))
                .send()
                .await
                .map_err(|err| LiveError::custom(format!("获取 Picarto explore 失败: {err}")))?
                .json()
                .await
                .map_err(|err| {
                    LiveError::custom(format!("解析 Picarto explore 失败: {err}"))
                })?;

            for entry in &explore.data {
                if let Some(url) = wanted.remove(&entry.name.to_lowercase()) {
                    live_urls.push(url);
                }
            }

            if wanted.is_empty() || explore.next_page_url.is_none() {
                break;
            }
            page += 1;
        }

        Ok(live_urls)
    }
}

impl Picarto {
    fn username_of(&self, url: &str) -> Option<String> {
        self.re
            .captures(url)
            .and_then(|caps| caps.name("id"))
            .map(|m| m.as_str().to_string())
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

#[derive(Deserialize)]
struct PicartoExplore {
    #[serde(default)]
    data: Vec<PicartoExploreEntry>,
    #[serde(default)]
    next_page_url: Option<String>,
}

#[derive(Deserialize)]
struct PicartoExploreEntry {
    #[serde(default)]
    name: String,
}
