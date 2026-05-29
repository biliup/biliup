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

const KILAKILA_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

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

impl DownloadPlugin for Kilakila {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    fn create_downloader(&self, ctx: &mut PluginContext) -> Box<dyn DownloadBase> {
        Box::new(KilakilaDownloader::new(
            ctx.client(),
            ctx.live_streamer().url.clone(),
            ctx.live_streamer().remark.clone(),
            ctx.config()
                .kila_protocol
                .clone()
                .unwrap_or_else(|| "hls".to_string()),
        ))
    }

    fn name(&self) -> &str {
        "Kilakila"
    }
}

struct KilakilaDownloader {
    client: Client,
    url: String,
    name: String,
    protocol: String,
}

impl KilakilaDownloader {
    fn new(client: Client, url: String, name: String, protocol: String) -> Self {
        Self {
            client,
            url,
            name,
            protocol,
        }
    }

    fn room_id(&self) -> Result<String, Report<AppError>> {
        if !self.url.contains("/PcLive/index/detail") && !self.url.contains("/room/") {
            return Err(Report::new(AppError::Custom(
                "Kilakila 直播间地址类型不支持".to_string(),
            )));
        }

        Regex::new(r"(\d+)")
            .unwrap()
            .captures(&self.url)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| Report::new(AppError::Custom("Kilakila 直播间地址错误".to_string())))
    }

    async fn room_info(&self, room_id: &str) -> Result<KilakilaResponse, Report<AppError>> {
        self.client
            .get("https://live.kilakila.cn/LiveRoom/getRoomInfo")
            .query(&[("roomId", room_id)])
            .header(reqwest::header::USER_AGENT, KILAKILA_USER_AGENT)
            .header(reqwest::header::REFERER, "https://live.kilakila.cn/")
            .send()
            .await
            .change_context(AppError::Custom("获取 Kilakila 直播间信息失败".to_string()))?
            .json()
            .await
            .change_context(AppError::Custom("解析 Kilakila 直播间信息失败".to_string()))
    }
}

#[async_trait]
impl DownloadBase for KilakilaDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        let room_id = self.room_id()?;
        let response = self.room_info(&room_id).await?;
        if response.header.code != 200 || response.body.status != 4 {
            return Ok(StreamStatus::Offline);
        }

        let raw_stream_url = if self.protocol == "flv" {
            response.body.flv_play_url
        } else {
            response.body.hls_play_url
        }
        .filter(|url| !url.is_empty())
        .ok_or_else(|| Report::new(AppError::Custom("Kilakila 直播流为空".to_string())))?;

        Ok(StreamStatus::Live {
            stream_info: Box::new(StreamInfoExt {
                streamer_info: StreamerInfo {
                    id: -1,
                    name: self.name.clone(),
                    url: self.url.clone(),
                    title: response.body.title.unwrap_or(room_id),
                    date: Utc::now(),
                    live_cover_path: response.body.back_pic.unwrap_or_default(),
                },
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| {
                    if self.protocol == "flv" {
                        "flv".to_string()
                    } else {
                        "m3u8".to_string()
                    }
                }),
                raw_stream_url,
                platform: "kilakila".to_string(),
                stream_headers: HashMap::new(),
            }),
        })
    }
}

#[derive(Deserialize)]
struct KilakilaResponse {
    #[serde(rename = "h")]
    header: KilakilaHeader,
    #[serde(rename = "b")]
    body: KilakilaBody,
}

#[derive(Deserialize)]
struct KilakilaHeader {
    code: i32,
}

#[derive(Deserialize)]
struct KilakilaBody {
    status: i32,
    title: Option<String>,
    #[serde(rename = "backPic")]
    back_pic: Option<String>,
    #[serde(rename = "flvPlayUrl")]
    flv_play_url: Option<String>,
    #[serde(rename = "hlsPlayUrl")]
    hls_play_url: Option<String>,
}
