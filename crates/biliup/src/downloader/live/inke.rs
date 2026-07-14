use super::{
    DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus, LiveStream,
    media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;

pub struct Inke {
    re: Regex,
}

impl Default for Inke {
    fn default() -> Self {
        Self::new()
    }
}

impl Inke {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"(?:https?://)?(?:(?:www)\.)?inke\.cn").unwrap(),
        }
    }
}

#[async_trait]
impl LivePlugin for Inke {
    fn name(&self) -> &'static str {
        "Inke"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        InkeLive::new(request).check_stream().await
    }
}

struct InkeLive {
    client: reqwest::Client,
    url: String,
    name: String,
}

impl InkeLive {
    fn new(request: LiveRequest) -> Self {
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        let uid = self.uid()?;
        let response: InkeResponse = self
            .client
            .get(format!(
                "https://webapi.busi.inke.cn/web/live_share_pc?uid={uid}"
            ))
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取映客直播间信息失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析映客直播间信息失败: {err}")))?;

        let Some(data) = response.data.filter(|_| response.error_code == 0) else {
            return Ok(LiveStatus::Offline);
        };
        if !data.status {
            return Ok(LiveStatus::Offline);
        }
        let raw_stream_url = data
            .live_addr
            .first()
            .map(|addr| addr.stream_addr.clone())
            .filter(|url| !url.is_empty())
            .ok_or_else(|| LiveError::custom("映客直播流为空"))?;

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: data.title.unwrap_or(uid),
                date: Utc::now(),
                live_cover_url: data.image.unwrap_or_default(),
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "flv".to_string()),
                raw_stream_url,
                platform: "inke".to_string(),
                stream_headers: HashMap::new(),
                danmaku: None,
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }

    fn uid(&self) -> LiveResult<String> {
        Regex::new(r"uid=([a-zA-Z0-9]+)")
            .unwrap()
            .captures(&self.url)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| LiveError::custom("映客直播间地址错误"))
    }
}

#[derive(Deserialize)]
struct InkeResponse {
    error_code: i32,
    #[serde(default)]
    data: Option<InkeData>,
}

#[derive(Deserialize)]
struct InkeData {
    status: bool,
    #[serde(default)]
    live_addr: Vec<InkeLiveAddr>,
    #[serde(alias = "live_name")]
    title: Option<String>,
    image: Option<String>,
}

#[derive(Deserialize)]
struct InkeLiveAddr {
    stream_addr: String,
}
