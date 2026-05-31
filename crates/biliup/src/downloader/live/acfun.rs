use super::{
    DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus, LiveStream,
    media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use rand::Rng;
use regex::Regex;
use reqwest::header::{COOKIE, REFERER, USER_AGENT};
use serde::Deserialize;
use std::collections::HashMap;

const ACFUN_VISITOR_LOGIN_URL: &str = "https://id.app.acfun.cn/rest/app/visitor/login";
const ACFUN_START_PLAY_URL: &str = "https://api.kuaishouzt.com/rest/zt/live/web/startPlay";
const ACFUN_REFERER: &str = "https://live.acfun.cn/";
const ACFUN_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct Acfun {
    re: Regex,
}

impl Default for Acfun {
    fn default() -> Self {
        Self::new()
    }
}

impl Acfun {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"(?:https?://)?(?:(?:www|m|live)\.)?acfun\.cn").unwrap(),
        }
    }
}

#[async_trait]
impl LivePlugin for Acfun {
    fn name(&self) -> &'static str {
        "Acfun"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        AcfunLive::new(request).check_stream().await
    }
}

struct AcfunLive {
    client: reqwest::Client,
    url: String,
    name: String,
}

impl AcfunLive {
    fn new(request: LiveRequest) -> Self {
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        let room_id = self.room_id()?;
        let did = format!("web_{}", random_name(16));
        let visitor = self.visitor_login(&did).await?;
        let Some(data) = self.start_play(&room_id, &did, visitor).await? else {
            return Ok(LiveStatus::Offline);
        };
        let raw_stream_url = self.select_stream_url(&data)?;

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: data.caption,
                date: Utc::now(),
                live_cover_url: String::new(),
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "flv".to_string()),
                raw_stream_url,
                platform: "acfun".to_string(),
                stream_headers: HashMap::from([
                    ("referer".to_string(), ACFUN_REFERER.to_string()),
                    ("user-agent".to_string(), ACFUN_USER_AGENT.to_string()),
                ]),
                danmaku: None,
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }

    fn room_id(&self) -> LiveResult<String> {
        Regex::new(r"(?:https?://)?(?:(?:www|m|live)\.)?acfun\.cn/live/(\d+)")
            .unwrap()
            .captures(&self.url)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| LiveError::custom("Acfun 直播间地址错误"))
    }

    async fn visitor_login(&self, did: &str) -> LiveResult<VisitorLoginResponse> {
        let response: VisitorLoginResponse = self
            .client
            .post(ACFUN_VISITOR_LOGIN_URL)
            .header(USER_AGENT, ACFUN_USER_AGENT)
            .header(COOKIE, format!("_did={did};"))
            .form(&[("sid", "acfun.api.visitor")])
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("Acfun 游客登录失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析 Acfun 游客登录响应失败: {err}")))?;

        if response.result != 0 {
            return Err(LiveError::custom(format!(
                "Acfun 游客登录返回错误: {}",
                response.result
            )));
        }

        Ok(response)
    }

    async fn start_play(
        &self,
        room_id: &str,
        did: &str,
        visitor: VisitorLoginResponse,
    ) -> LiveResult<Option<StartPlayData>> {
        let params = [
            ("subBiz", "mainApp".to_string()),
            ("kpn", "ACFUN_APP".to_string()),
            ("kpf", "PC_WEB".to_string()),
            ("userId", visitor.user_id.to_string()),
            ("did", did.to_string()),
            ("acfun.api.visitor_st", visitor.visitor_st),
        ];
        let response: StartPlayResponse = self
            .client
            .post(ACFUN_START_PLAY_URL)
            .header(USER_AGENT, ACFUN_USER_AGENT)
            .header(REFERER, ACFUN_REFERER)
            .query(&params)
            .form(&[
                ("authorId", room_id.to_string()),
                ("pullStreamType", "FLV".to_string()),
            ])
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取 Acfun 直播流信息失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析 Acfun 直播流信息失败: {err}")))?;

        if response.result != 1 {
            return Ok(None);
        }

        response
            .data
            .map(Some)
            .ok_or_else(|| LiveError::custom("Acfun 直播流数据为空"))
    }

    fn select_stream_url(&self, data: &StartPlayData) -> LiveResult<String> {
        let video_play_res: VideoPlayRes = serde_json::from_str(&data.video_play_res)
            .map_err(|err| LiveError::custom(format!("解析 Acfun 播放地址失败: {err}")))?;

        video_play_res
            .live_adaptive_manifest
            .first()
            .and_then(|manifest| manifest.adaptation_set.representation.last())
            .map(|representation| representation.url.clone())
            .filter(|url| !url.is_empty())
            .ok_or_else(|| LiveError::custom("Acfun 可用直播流为空"))
    }
}

#[derive(Deserialize)]
struct VisitorLoginResponse {
    result: i32,
    #[serde(rename = "userId")]
    user_id: i64,
    #[serde(rename = "acfun.api.visitor_st")]
    visitor_st: String,
}

#[derive(Deserialize)]
struct StartPlayResponse {
    result: i32,
    data: Option<StartPlayData>,
}

#[derive(Deserialize)]
struct StartPlayData {
    caption: String,
    #[serde(rename = "videoPlayRes")]
    video_play_res: String,
}

#[derive(Deserialize)]
struct VideoPlayRes {
    #[serde(rename = "liveAdaptiveManifest")]
    live_adaptive_manifest: Vec<LiveAdaptiveManifest>,
}

#[derive(Deserialize)]
struct LiveAdaptiveManifest {
    #[serde(rename = "adaptationSet")]
    adaptation_set: AdaptationSet,
}

#[derive(Deserialize)]
struct AdaptationSet {
    representation: Vec<Representation>,
}

#[derive(Deserialize)]
struct Representation {
    url: String,
}

fn random_name(len: usize) -> String {
    let mut rng = rand::thread_rng();
    let first = rng.gen_range(b'a'..=b'z') as char;
    let charset = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let rest = (1..len)
        .map(|_| charset[rng.gen_range(0..charset.len())] as char)
        .collect::<String>();
    format!("{first}{rest}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_acfun_urls() {
        let plugin = Acfun::new();

        assert!(plugin.matches("https://live.acfun.cn/live/12345"));
        assert!(plugin.matches("https://www.acfun.cn/live/12345"));
        assert!(!plugin.matches("https://example.com/live/12345"));
    }

    #[test]
    fn parses_room_id() {
        let live = AcfunLive {
            client: reqwest::Client::new(),
            url: "https://live.acfun.cn/live/12345".to_string(),
            name: String::new(),
        };

        assert_eq!(live.room_id().unwrap(), "12345");
    }
}
