use super::{
    DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus, LiveStream,
    media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

const CC_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub struct CC {
    re: Regex,
}

impl Default for CC {
    fn default() -> Self {
        Self::new()
    }
}

impl CC {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"https?://cc\.163\.com").unwrap(),
        }
    }
}

#[async_trait]
impl LivePlugin for CC {
    fn name(&self) -> &'static str {
        "CC"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        CCLive::new(request).check_stream().await
    }
}

struct CCLive {
    client: reqwest::Client,
    url: String,
    name: String,
    protocol: String,
}

impl CCLive {
    fn new(request: LiveRequest) -> Self {
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
            protocol: request.options.cc.protocol,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        let room_id = self.room_id()?;
        let Some(channel_id) = self.channel_id(&room_id).await? else {
            return Ok(LiveStatus::Offline);
        };
        let channel = self.channel_info(&channel_id).await?;
        let raw_stream_url = self.stream_url(&channel)?;
        let title = channel["title"]
            .as_str()
            .filter(|title| !title.is_empty())
            .unwrap_or(&room_id)
            .to_string();
        let suffix = media_ext_from_url(&raw_stream_url).unwrap_or_else(|| {
            if self.protocol == "hls" {
                "m3u8".to_string()
            } else {
                "flv".to_string()
            }
        });

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title,
                date: Utc::now(),
                live_cover_url: String::new(),
                raw_stream_url,
                platform: "cc".to_string(),
                stream_headers: HashMap::new(),
                suffix,
                danmaku: None,
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }

    fn room_id(&self) -> LiveResult<String> {
        Regex::new(r"(\d{4,})")
            .unwrap()
            .captures(&self.url)
            .map(|captures| captures[1].to_string())
            .ok_or_else(|| LiveError::custom("网易 CC 直播间地址错误"))
    }

    async fn channel_id(&self, room_id: &str) -> LiveResult<Option<String>> {
        let value: Value = self
            .client
            .get(format!(
                "https://api.cc.163.com/v1/activitylives/anchor/lives?anchor_ccid={room_id}"
            ))
            .header(reqwest::header::USER_AGENT, CC_USER_AGENT)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取网易 CC 直播间信息失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析网易 CC 直播间信息失败: {err}")))?;

        let Some(room) = value["data"].get(room_id) else {
            return Ok(None);
        };
        if room.as_object().map(|object| object.len()).unwrap_or(0) <= 1 {
            return Ok(None);
        }
        Ok(room["channel_id"].as_i64().map(|id| id.to_string()))
    }

    async fn channel_info(&self, channel_id: &str) -> LiveResult<Value> {
        let value: Value = self
            .client
            .get(format!(
                "https://cc.163.com/live/channel/?channelids={channel_id}"
            ))
            .header(reqwest::header::USER_AGENT, CC_USER_AGENT)
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("获取网易 CC 频道信息失败: {err}")))?
            .json()
            .await
            .map_err(|err| LiveError::custom(format!("解析网易 CC 频道信息失败: {err}")))?;

        value["data"]
            .as_array()
            .and_then(|data| data.first())
            .cloned()
            .ok_or_else(|| LiveError::custom("网易 CC 频道信息为空"))
    }

    fn stream_url(&self, channel: &Value) -> LiveResult<String> {
        if self.protocol == "hls" {
            return channel["sharefile"]
                .as_str()
                .filter(|url| !url.is_empty())
                .map(str::to_string)
                .ok_or_else(|| LiveError::custom("网易 CC HLS 直播流为空"));
        }

        channel["quickplay"]["resolution"]
            .as_object()
            .and_then(|resolutions| {
                resolutions
                    .values()
                    .max_by_key(|level| level["vbr"].as_i64().unwrap_or(0))
            })
            .and_then(|level| level["cdn"].as_object())
            .and_then(|cdn| cdn.values().find_map(Value::as_str))
            .filter(|url| !url.is_empty())
            .map(str::to_string)
            .ok_or_else(|| LiveError::custom("网易 CC 直播流为空"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_cc_urls() {
        let plugin = CC::new();

        assert!(plugin.matches("https://cc.163.com/12345"));
        assert!(!plugin.matches("https://example.com/12345"));
    }
}
