use super::{
    DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus, LiveStream,
    media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use reqwest::header::{REFERER, USER_AGENT};
use serde::Deserialize;
use std::collections::HashMap;

const KILAKILA_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const KILAKILA_REFERER: &str = "https://live.kilakila.cn/";

pub struct Kilakila {
    re: Regex,
}

impl Default for Kilakila {
    fn default() -> Self {
        Self::new()
    }
}

impl Kilakila {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"https?://(?:live\.kilakila\.cn|www\.hongdoufm\.com)").unwrap(),
        }
    }
}

#[async_trait]
impl LivePlugin for Kilakila {
    fn name(&self) -> &'static str {
        "Kilakila"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        KilakilaLive::new(request).check_stream().await
    }
}

struct KilakilaLive {
    client: reqwest::Client,
    url: String,
    name: String,
    protocol: String,
}

impl KilakilaLive {
    fn new(request: LiveRequest) -> Self {
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
            protocol: request.options.kilakila.protocol,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        let room_id = self.room_id()?;
        let response: KilakilaResponse = self
            .client
            .get(format!(
                "https://live.kilakila.cn/LiveRoom/getRoomInfo?roomId={room_id}"
            ))
            .header(USER_AGENT, KILAKILA_USER_AGENT)
            .header(REFERER, KILAKILA_REFERER)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取 Kilakila 直播间信息失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析 Kilakila 直播间信息失败: {err}")))?;

        if response.h.code != 200 || response.b.status != 4 {
            return Ok(LiveStatus::Offline);
        }
        let raw_stream_url = if self.protocol == "flv" {
            response.b.flv_play_url
        } else {
            response.b.hls_play_url
        }
        .filter(|url| !url.is_empty())
        .ok_or_else(|| LiveError::custom("Kilakila 直播流为空"))?;

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: response.b.title.unwrap_or(room_id),
                date: Utc::now(),
                live_cover_url: response.b.cover_url.unwrap_or_default(),
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| {
                    if self.protocol == "flv" {
                        "flv".to_string()
                    } else {
                        "m3u8".to_string()
                    }
                }),
                raw_stream_url,
                platform: "kilakila".to_string(),
                stream_headers: HashMap::from([
                    ("referer".to_string(), KILAKILA_REFERER.to_string()),
                    ("user-agent".to_string(), KILAKILA_USER_AGENT.to_string()),
                ]),
                danmaku: None,
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }

    fn room_id(&self) -> LiveResult<String> {
        let marker = if self.url.contains("/PcLive/index/detail") {
            "/PcLive/index/detail/"
        } else if self.url.contains("/room/") {
            "/room/"
        } else {
            return Err(LiveError::custom("Kilakila 直播间地址错误"));
        };
        self.url
            .split(marker)
            .nth(1)
            .and_then(|value| value.split(['/', '?']).next())
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .ok_or_else(|| LiveError::custom("Kilakila 房间号为空"))
    }
}

#[derive(Deserialize)]
struct KilakilaResponse {
    h: KilakilaHeader,
    b: KilakilaBody,
}

#[derive(Deserialize)]
struct KilakilaHeader {
    code: i32,
}

#[derive(Deserialize)]
struct KilakilaBody {
    status: i32,
    #[serde(rename = "flvPlayUrl")]
    flv_play_url: Option<String>,
    #[serde(rename = "hlsPlayUrl")]
    hls_play_url: Option<String>,
    title: Option<String>,
    #[serde(rename = "coverUrl", alias = "backPic")]
    cover_url: Option<String>,
}
