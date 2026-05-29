use crate::server::common::util::media_ext_from_url;
use crate::server::core::plugin::{DownloadBase, DownloadPlugin, StreamInfoExt, StreamStatus};
use crate::server::errors::AppError;
use crate::server::infrastructure::context::PluginContext;
use crate::server::infrastructure::models::StreamerInfo;
use async_trait::async_trait;
use chrono::Utc;
use error_stack::{Report, ResultExt, bail};
use rand::Rng;
use regex::Regex;
use reqwest::Client;
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

impl DownloadPlugin for Acfun {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    fn create_downloader(&self, ctx: &mut PluginContext) -> Box<dyn DownloadBase> {
        Box::new(AcfunDownloader::new(
            ctx.client(),
            ctx.live_streamer().url.clone(),
            ctx.live_streamer().remark.clone(),
        ))
    }

    fn name(&self) -> &str {
        "Acfun"
    }
}

struct AcfunDownloader {
    client: Client,
    url: String,
    name: String,
}

impl AcfunDownloader {
    fn new(client: Client, url: String, name: String) -> Self {
        Self { client, url, name }
    }

    fn room_id(&self) -> Result<String, Report<AppError>> {
        Regex::new(r"(?:https?://)?(?:(?:www|m|live)\.)?acfun\.cn/live/(\d+)")
            .unwrap()
            .captures(&self.url)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| Report::new(AppError::Custom("Acfun 直播间地址错误".to_string())))
    }

    async fn visitor_login(&self, did: &str) -> Result<VisitorLoginResponse, Report<AppError>> {
        let response: VisitorLoginResponse = self
            .client
            .post(ACFUN_VISITOR_LOGIN_URL)
            .header(USER_AGENT, ACFUN_USER_AGENT)
            .header(COOKIE, format!("_did={did};"))
            .form(&[("sid", "acfun.api.visitor")])
            .send()
            .await
            .change_context(AppError::Custom("Acfun 游客登录失败".to_string()))?
            .json()
            .await
            .change_context(AppError::Custom("解析 Acfun 游客登录响应失败".to_string()))?;

        if response.result != 0 {
            bail!(AppError::Custom(format!(
                "Acfun 游客登录返回错误: {}",
                response.result
            )))
        }

        Ok(response)
    }

    async fn start_play(
        &self,
        room_id: &str,
        did: &str,
        visitor: VisitorLoginResponse,
    ) -> Result<Option<StartPlayData>, Report<AppError>> {
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
            .change_context(AppError::Custom("获取 Acfun 直播流信息失败".to_string()))?
            .json()
            .await
            .change_context(AppError::Custom("解析 Acfun 直播流信息失败".to_string()))?;

        if response.result != 1 {
            return Ok(None);
        }

        response
            .data
            .map(Some)
            .ok_or_else(|| Report::new(AppError::Custom("Acfun 直播流数据为空".to_string())))
    }

    fn select_stream_url(&self, data: &StartPlayData) -> Result<String, Report<AppError>> {
        let video_play_res: VideoPlayRes = serde_json::from_str(&data.video_play_res)
            .change_context(AppError::Custom("解析 Acfun 播放地址失败".to_string()))?;

        video_play_res
            .live_adaptive_manifest
            .first()
            .and_then(|manifest| manifest.adaptation_set.representation.last())
            .map(|representation| representation.url.clone())
            .filter(|url| !url.is_empty())
            .ok_or_else(|| Report::new(AppError::Custom("Acfun 可用直播流为空".to_string())))
    }
}

#[async_trait]
impl DownloadBase for AcfunDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        let room_id = self.room_id()?;
        let did = format!("web_{}", random_name(16));
        let visitor = self.visitor_login(&did).await?;
        let Some(data) = self.start_play(&room_id, &did, visitor).await? else {
            return Ok(StreamStatus::Offline);
        };
        let raw_stream_url = self.select_stream_url(&data)?;

        Ok(StreamStatus::Live {
            stream_info: Box::new(StreamInfoExt {
                streamer_info: StreamerInfo {
                    id: -1,
                    name: self.name.clone(),
                    url: self.url.clone(),
                    title: data.caption,
                    date: Utc::now(),
                    live_cover_path: String::new(),
                },
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "flv".to_string()),
                raw_stream_url,
                platform: "acfun".to_string(),
                stream_headers: HashMap::from([
                    ("referer".to_string(), ACFUN_REFERER.to_string()),
                    ("user-agent".to_string(), ACFUN_USER_AGENT.to_string()),
                ]),
            }),
        })
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
