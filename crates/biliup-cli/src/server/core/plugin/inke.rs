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

const INKE_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

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

impl DownloadPlugin for Inke {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    fn create_downloader(&self, ctx: &mut PluginContext) -> Box<dyn DownloadBase> {
        Box::new(InkeDownloader::new(
            ctx.client(),
            ctx.live_streamer().url.clone(),
            ctx.live_streamer().remark.clone(),
        ))
    }

    fn name(&self) -> &str {
        "Inke"
    }
}

struct InkeDownloader {
    client: Client,
    url: String,
    name: String,
}

impl InkeDownloader {
    fn new(client: Client, url: String, name: String) -> Self {
        Self { client, url, name }
    }

    fn uid(&self) -> Result<String, Report<AppError>> {
        Regex::new(r"uid=([a-zA-Z0-9]+)")
            .unwrap()
            .captures(&self.url)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| Report::new(AppError::Custom("映客直播间地址错误".to_string())))
    }

    async fn live_info(&self, uid: &str) -> Result<InkeResponse, Report<AppError>> {
        self.client
            .get(format!(
                "https://webapi.busi.inke.cn/web/live_share_pc?uid={uid}"
            ))
            .header(reqwest::header::USER_AGENT, INKE_USER_AGENT)
            .send()
            .await
            .change_context(AppError::Custom("获取映客直播间信息失败".to_string()))?
            .json()
            .await
            .change_context(AppError::Custom("解析映客直播间信息失败".to_string()))
    }
}

#[async_trait]
impl DownloadBase for InkeDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        let uid = self.uid()?;
        let response = self.live_info(&uid).await?;
        if response.error_code != 0 {
            return Ok(StreamStatus::Offline);
        }
        if !response.data.status {
            return Ok(StreamStatus::Offline);
        }

        let raw_stream_url = response
            .data
            .live_addr
            .first()
            .map(|addr| addr.stream_addr.clone())
            .filter(|url| !url.is_empty())
            .ok_or_else(|| Report::new(AppError::Custom("映客直播流为空".to_string())))?;

        Ok(StreamStatus::Live {
            stream_info: Box::new(StreamInfoExt {
                streamer_info: StreamerInfo {
                    id: -1,
                    name: self.name.clone(),
                    url: self.url.clone(),
                    title: response.data.live_name,
                    date: Utc::now(),
                    live_cover_path: String::new(),
                },
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "flv".to_string()),
                raw_stream_url,
                platform: "inke".to_string(),
                stream_headers: HashMap::new(),
            }),
        })
    }
}

#[derive(Deserialize)]
struct InkeResponse {
    error_code: i32,
    data: InkeData,
}

#[derive(Deserialize)]
struct InkeData {
    status: bool,
    live_name: String,
    live_addr: Vec<InkeLiveAddr>,
}

#[derive(Deserialize)]
struct InkeLiveAddr {
    stream_addr: String,
}
