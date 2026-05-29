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
use serde_json::Value;
use std::collections::HashMap;

const MISSEVAN_API_URL: &str = "https://fm.missevan.com/api/v2/live";
const MISSEVAN_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct Missevan {
    re: Regex,
}

impl Default for Missevan {
    fn default() -> Self {
        Self::new()
    }
}

impl Missevan {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"(?:https?://)?(?:(?:www|fm)\.)?missevan\.com").unwrap(),
        }
    }
}

impl DownloadPlugin for Missevan {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    fn create_downloader(&self, ctx: &mut PluginContext) -> Box<dyn DownloadBase> {
        Box::new(MissevanDownloader::new(
            ctx.client(),
            ctx.live_streamer().url.clone(),
            ctx.live_streamer().remark.clone(),
        ))
    }

    fn name(&self) -> &str {
        "Missevan"
    }
}

struct MissevanDownloader {
    client: Client,
    url: String,
    name: String,
}

impl MissevanDownloader {
    fn new(client: Client, url: String, name: String) -> Self {
        Self { client, url, name }
    }

    async fn room_id(&self) -> Result<String, Report<AppError>> {
        if let Some(room_id) = Regex::new(r"/(\d+)")
            .unwrap()
            .captures(&self.url)
            .map(|captures| captures[1].to_string())
        {
            return Ok(room_id);
        }

        let text = self
            .client
            .get(&self.url)
            .header(reqwest::header::USER_AGENT, MISSEVAN_USER_AGENT)
            .send()
            .await
            .change_context(AppError::Custom("获取 Missevan 用户页面失败".to_string()))?
            .text()
            .await
            .change_context(AppError::Custom("读取 Missevan 用户页面失败".to_string()))?;

        Regex::new(r#"data-id="(\d+)""#)
            .unwrap()
            .captures(&text)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| Report::new(AppError::Custom("Missevan 直播间地址错误".to_string())))
    }

    async fn room_info(&self, room_id: &str) -> Result<Option<Value>, Report<AppError>> {
        let value: Value = self
            .client
            .get(format!("{MISSEVAN_API_URL}/{room_id}"))
            .header(reqwest::header::USER_AGENT, MISSEVAN_USER_AGENT)
            .send()
            .await
            .change_context(AppError::Custom("获取 Missevan 直播间信息失败".to_string()))?
            .json()
            .await
            .change_context(AppError::Custom("解析 Missevan 直播间信息失败".to_string()))?;

        if value.get("code").and_then(Value::as_i64) != Some(0) {
            return Ok(None);
        }
        Ok(Some(value))
    }
}

#[async_trait]
impl DownloadBase for MissevanDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        let room_id = self.room_id().await?;
        let Some(info) = self.room_info(&room_id).await? else {
            return Ok(StreamStatus::Offline);
        };

        let room = &info["info"]["room"];
        if room["status"]["open"].as_i64() == Some(0) {
            return Ok(StreamStatus::Offline);
        }

        let Some(raw_stream_url) = room["channel"]["flv_pull_url"]
            .as_str()
            .filter(|url| !url.is_empty())
            .map(str::to_string)
        else {
            return Ok(StreamStatus::Offline);
        };
        let title = room["name"]
            .as_str()
            .filter(|title| !title.is_empty())
            .unwrap_or(&room_id)
            .to_string();

        Ok(StreamStatus::Live {
            stream_info: Box::new(StreamInfoExt {
                streamer_info: StreamerInfo {
                    id: -1,
                    name: self.name.clone(),
                    url: self.url.clone(),
                    title,
                    date: Utc::now(),
                    live_cover_path: String::new(),
                },
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "flv".to_string()),
                raw_stream_url,
                platform: "missevan".to_string(),
                stream_headers: HashMap::new(),
            }),
        })
    }
}
