use super::{
    DownloaderHint, LiveError, LivePlugin, LiveRequest, LiveResult, LiveStatus, LiveStream,
    media_ext_from_url,
};
use async_trait::async_trait;
use chrono::Utc;
use regex::Regex;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::Duration;

pub struct YY {
    re: Regex,
}

impl Default for YY {
    fn default() -> Self {
        Self::new()
    }
}

impl YY {
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"(?:https?://)?(?:www\.)?yy\.com").unwrap(),
        }
    }
}

#[async_trait]
impl LivePlugin for YY {
    fn name(&self) -> &'static str {
        "YY"
    }

    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    async fn check_stream(&self, request: LiveRequest) -> LiveResult<LiveStatus> {
        YYLive::new(request).check_stream().await
    }
}

struct YYLive {
    client: reqwest::Client,
    url: String,
    name: String,
}

impl YYLive {
    fn new(request: LiveRequest) -> Self {
        Self {
            client: request.client,
            url: request.url,
            name: request.name,
        }
    }

    async fn check_stream(&self) -> LiveResult<LiveStatus> {
        let rid = room_id(&self.url)?;
        let millis_13 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| LiveError::custom(format!("获取系统时间失败: {err}")))?
            .as_millis() as u64;
        let millis_10 = millis_13 / 1000;

        let data = json!({
            "head": {
                "seq": millis_13,
                "appidstr": "0",
                "bidstr": "121",
                "cidstr": rid,
                "sidstr": rid,
                "uid64": 0,
                "client_type": 108,
                "client_ver": "5.11.0-alpha.4",
                "stream_sys_ver": 1,
                "app": "yylive_web",
                "playersdk_ver": "5.11.0-alpha.4",
                "thundersdk_ver": "0",
                "streamsdk_ver": "5.11.0-alpha.4"
            },
            "client_attribute": {
                "client": "web",
                "model": "",
                "cpu": "",
                "graphics_card": "",
                "os": "chrome",
                "osversion": "106.0.0.0",
                "vsdk_version": "",
                "app_identify": "",
                "app_version": "",
                "business": "",
                "width": "1536",
                "height": "864",
                "scale": "",
                "client_type": 8,
                "h265": 0
            },
            "avp_parameter": {
                "version": 1,
                "client_type": 8,
                "service_type": 0,
                "imsi": 0,
                "send_time": millis_10,
                "line_seq": -1,
                "gear": 4,
                "ssl": 1,
                "stream_format": 0
            }
        });

        let result = self
            .client
            .post(format!(
                "https://stream-manager.yy.com/v3/channel/streams?uid=0&cid={rid}&sid={rid}&appid=0&sequence={millis_13}&encode=json"
            ))
            .header("content-type", "text/plain;charset=UTF-8")
            .header("referer", "https://www.yy.com/")
            .timeout(Duration::from_secs(30))
            .body(data.to_string())
            .send()
            .await
            .map_err(|err| LiveError::custom(format!("请求 YY 流信息失败 rid: {rid}: {err}")))?
            .json::<Value>()
            .await
            .map_err(|err| LiveError::custom(format!("解析 YY 流信息失败 rid: {rid}: {err}")))?;

        let Some(raw_stream_url) = result
            .get("avp_info_res")
            .and_then(|info| info.get("stream_line_addr"))
            .and_then(Value::as_object)
            .and_then(|obj| obj.values().next())
            .and_then(|val| val.get("cdn_info"))
            .and_then(|cdn| cdn.get("url"))
            .and_then(Value::as_str)
            .filter(|url| !url.is_empty())
            .map(str::to_string)
        else {
            return Ok(LiveStatus::Offline);
        };

        Ok(LiveStatus::Live {
            stream: Box::new(LiveStream {
                name: self.name.clone(),
                url: self.url.clone(),
                title: String::new(),
                date: Utc::now(),
                live_cover_url: String::new(),
                raw_stream_url: raw_stream_url.clone(),
                platform: "yy".to_string(),
                stream_headers: HashMap::new(),
                suffix: media_ext_from_url(&raw_stream_url).unwrap_or_else(|| "flv".to_string()),
                danmaku: None,
                downloader_hint: DownloaderHint::StreamGears,
                runtime_options: None,
            }),
        })
    }
}

fn room_id(url: &str) -> LiveResult<&str> {
    let Some(part) = url.split("www.yy.com/").nth(1) else {
        return Err(LiveError::custom("YY 直播间地址错误"));
    };
    let rid = part.split('/').next().unwrap_or("");
    if rid.is_empty() {
        return Err(LiveError::custom("YY rid 为空"));
    }
    Ok(rid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_yy_urls() {
        let plugin = YY::new();

        assert!(plugin.matches("https://www.yy.com/12345"));
        assert!(plugin.matches("http://yy.com/12345"));
        assert!(!plugin.matches("https://example.com/12345"));
    }

    #[test]
    fn parses_room_id() {
        assert_eq!(room_id("https://www.yy.com/12345").unwrap(), "12345");
        assert_eq!(room_id("https://www.yy.com/12345/foo").unwrap(), "12345");
        assert!(room_id("https://example.com/12345").is_err());
    }
}
