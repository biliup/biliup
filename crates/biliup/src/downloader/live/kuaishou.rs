use super::{
    DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus, LiveStream,
    media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use rand::Rng;
use regex::Regex;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, REFERER, USER_AGENT};
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;
use url::Url;

const KUAISHOU_HOME_URL: &str = "https://live.kuaishou.com";
const KUAISHOU_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct Kuaishou {
    re: Regex,
}

impl Default for Kuaishou {
    fn default() -> Self {
        Self::new()
    }
}

impl Kuaishou {
    pub fn new() -> Self {
        Self {
            re: Regex::new(
                r"(?:https?://)?(?:(?:live|www|v)\.)?kuaishou\.com|(?:https?://)?(?:(?:livev)\.m\.)?chenzhongtech\.com",
            )
            .unwrap(),
        }
    }
}

#[async_trait]
impl LivePlugin for Kuaishou {
    fn name(&self) -> &'static str {
        "Kuaishou"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        KuaishouLive::new(request).check_stream().await
    }
}

struct KuaishouLive {
    client: reqwest::Client,
    url: String,
    name: String,
    cookie: Option<String>,
}

impl KuaishouLive {
    fn new(request: LiveRequest) -> Self {
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
            cookie: request.options.kuaishou.cookie,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        let room_id = self.resolve_room_id().await?;
        self.warmup().await?;
        if self.get_room_page(&room_id).await?.is_none() {
            return Ok(LiveStatus::Offline);
        }
        let Some(live_stream) = self.get_room_info(&room_id).await? else {
            return Ok(LiveStatus::Offline);
        };
        let raw_stream_url = self.select_stream_url(&live_stream)?;
        let title = if live_stream.caption.is_empty() {
            room_id
        } else {
            live_stream.caption
        };

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title,
                date: Utc::now(),
                live_cover_url: live_stream.cover_url.unwrap_or_default(),
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "flv".to_string()),
                raw_stream_url,
                platform: "kuaishou".to_string(),
                stream_headers: HashMap::from([
                    ("referer".to_string(), KUAISHOU_HOME_URL.to_string()),
                    ("user-agent".to_string(), KUAISHOU_USER_AGENT.to_string()),
                ]),
                danmaku: None,
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(KUAISHOU_USER_AGENT));
        headers.insert(REFERER, HeaderValue::from_static(KUAISHOU_HOME_URL));
        if let Some(cookie) = &self.cookie
            && let Ok(cookie) = HeaderValue::from_str(cookie)
        {
            headers.insert(COOKIE, cookie);
        }
        headers
    }

    async fn resolve_room_id(&self) -> LiveResult<String> {
        if let Some(room_id) = extract_room_id(&self.url) {
            return Ok(room_id);
        }

        let resp = self
            .client
            .get(&self.url)
            .headers(self.headers())
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("解析快手跳转地址失败: {err}")))?;

        extract_room_id(resp.url().as_str()).ok_or_else(|| LiveError::custom("快手直播间地址错误"))
    }

    async fn warmup(&self) -> LiveResult<()> {
        self.client
            .get(KUAISHOU_HOME_URL)
            .headers(self.headers())
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("请求快手首页失败: {err}")))?;

        let delay = rand::thread_rng().gen_range(3000..4000);
        sleep(Duration::from_millis(delay)).await;
        Ok(())
    }

    async fn get_room_page(&self, room_id: &str) -> LiveResult<Option<String>> {
        let text = self
            .client
            .get(format!("{KUAISHOU_HOME_URL}/u/{room_id}"))
            .headers(self.headers())
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取快手直播间页面失败: {err}")))?
            .text()
            .await
            .map_err(|err| LiveError::custom(format!("读取快手直播间页面失败: {err}")))?;

        if ["错误代码22", "主播尚未开播"]
            .iter()
            .any(|key| text.contains(key))
        {
            return Ok(None);
        }

        Ok(Some(text))
    }

    async fn get_room_info(&self, room_id: &str) -> LiveResult<Option<KuaishouLiveStream>> {
        let response: KuaishouLiveDetailResponse = self
            .client
            .get(format!(
                "{KUAISHOU_HOME_URL}/live_api/liveroom/livedetail?principalId={room_id}"
            ))
            .headers(self.headers())
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取快手直播间信息失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析快手直播间信息失败: {err}")))?;

        match response.data.result {
            1 => response
                .data
                .live_stream
                .map(Some)
                .ok_or_else(|| LiveError::custom("快手直播流数据为空")),
            22 | 671 => Ok(None),
            _ => Ok(None),
        }
    }

    fn select_stream_url(&self, live_stream: &KuaishouLiveStream) -> LiveResult<String> {
        live_stream
            .play_urls
            .first()
            .and_then(|play_url| play_url.adaptation_set.representation.last())
            .map(|representation| representation.url.clone())
            .filter(|url| !url.is_empty())
            .ok_or_else(|| LiveError::custom("快手可用直播流为空"))
    }
}

#[derive(Deserialize)]
struct KuaishouLiveDetailResponse {
    data: KuaishouRoomData,
}

#[derive(Deserialize)]
struct KuaishouRoomData {
    result: i32,
    #[serde(default, rename = "liveStream")]
    live_stream: Option<KuaishouLiveStream>,
}

#[derive(Deserialize)]
struct KuaishouLiveStream {
    #[serde(default)]
    caption: String,
    #[serde(default, rename = "coverUrl")]
    cover_url: Option<String>,
    #[serde(default, rename = "playUrls")]
    play_urls: Vec<KuaishouPlayUrl>,
}

#[derive(Deserialize)]
struct KuaishouPlayUrl {
    #[serde(rename = "adaptationSet")]
    adaptation_set: KuaishouAdaptationSet,
}

#[derive(Deserialize)]
struct KuaishouAdaptationSet {
    #[serde(default)]
    representation: Vec<KuaishouRepresentation>,
}

#[derive(Deserialize)]
struct KuaishouRepresentation {
    url: String,
}

fn extract_room_id(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let path = parsed.path();
    ["/profile/", "/fw/live/", "/u/"].iter().find_map(|marker| {
        path.split_once(marker)
            .and_then(|(_, rest)| rest.split('/').next())
            .and_then(|room_id| room_id.split('?').next())
            .filter(|room_id| !room_id.is_empty())
            .map(str::to_string)
    })
}
