use super::{
    DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus, LiveStream,
    media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;

const BIGO_API_URL: &str = "https://ta.bigo.tv/official_website/studio/getInternalStudioInfo";

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

#[async_trait]
impl LivePlugin for Bigo {
    fn name(&self) -> &'static str {
        "Bigo"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        BigoLive::new(request).check_stream().await
    }
}

struct BigoLive {
    client: reqwest::Client,
    url: String,
    name: String,
}

impl BigoLive {
    fn new(request: LiveRequest) -> Self {
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        let room_id = self.room_id()?;
        let response: BigoResponse = self
            .client
            .post(BIGO_API_URL)
            .form(&[("siteId", room_id.as_str())])
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取 Bigo 直播间信息失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析 Bigo 直播间信息失败: {err}")))?;

        if response.code != 0 {
            return Ok(LiveStatus::Offline);
        }
        let Some(data) = response.data else {
            return Ok(LiveStatus::Offline);
        };
        if data.alive != Some(1) {
            return Ok(LiveStatus::Offline);
        }
        let Some(raw_stream_url) = data.hls_src.filter(|url| !url.is_empty()) else {
            return Ok(LiveStatus::Offline);
        };

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: data.room_topic.unwrap_or(room_id),
                date: Utc::now(),
                live_cover_url: String::new(),
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "m3u8".to_string()),
                raw_stream_url,
                platform: "bigo".to_string(),
                stream_headers: HashMap::new(),
                danmaku: None,
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }

    fn room_id(&self) -> LiveResult<String> {
        self.url
            .split('/')
            .filter(|part| !part.is_empty())
            .next_back()
            .and_then(|part| part.split(['?', '#']).next())
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .ok_or_else(|| LiveError::custom("Bigo 直播间地址错误"))
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
    #[serde(rename = "hls_src")]
    hls_src: Option<String>,
    #[serde(rename = "roomTopic")]
    room_topic: Option<String>,
}
