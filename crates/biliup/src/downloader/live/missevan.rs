use super::{
    DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus, LiveStream,
    media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

const MISSEVAN_API_URL: &str = "https://fm.missevan.com/api/v2/live";

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

#[async_trait]
impl LivePlugin for Missevan {
    fn name(&self) -> &'static str {
        "Missevan"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        MissevanLive::new(request).check_stream().await
    }
}

struct MissevanLive {
    client: reqwest::Client,
    url: String,
    name: String,
}

impl MissevanLive {
    fn new(request: LiveRequest) -> Self {
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        let room_id = self.room_id().await?;
        let value: Value = self
            .client
            .get(format!("{MISSEVAN_API_URL}/{room_id}"))
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取猫耳直播间信息失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析猫耳直播间信息失败: {err}")))?;

        if value["code"].as_i64().unwrap_or(-1) != 0 {
            return Ok(LiveStatus::Offline);
        }
        let room = &value["info"]["room"];
        if room["status"]["open"].as_i64() == Some(0) {
            return Ok(LiveStatus::Offline);
        }
        let raw_stream_url = room["channel"]["flv_pull_url"]
            .as_str()
            .filter(|url| !url.is_empty())
            .map(str::to_string)
            .ok_or_else(|| LiveError::custom("猫耳直播流为空"))?;
        let title = room["name"].as_str().unwrap_or(&room_id).to_string();
        let cover = room["cover"].as_str().unwrap_or_default().to_string();

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title,
                date: Utc::now(),
                live_cover_url: cover,
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "flv".to_string()),
                raw_stream_url,
                platform: "missevan".to_string(),
                stream_headers: HashMap::new(),
                danmaku: None,
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }

    async fn room_id(&self) -> LiveResult<String> {
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
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取猫耳用户页面失败: {err}")))?
            .text()
            .await
            .map_err(|err| LiveError::custom(format!("读取猫耳用户页面失败: {err}")))?;

        Regex::new(r#"data-id="(\d+)""#)
            .unwrap()
            .captures(&text)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| LiveError::custom("猫耳直播间地址错误"))
    }
}
