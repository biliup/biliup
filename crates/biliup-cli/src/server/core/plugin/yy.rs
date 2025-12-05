use crate::server::common::util::media_ext_from_url;
use crate::server::core::plugin::{DownloadBase, DownloadPlugin, StreamInfoExt, StreamStatus};
use crate::server::errors::AppError;
use crate::server::infrastructure::context::{Context, PluginContext};
use crate::server::infrastructure::models::StreamerInfo;
use async_trait::async_trait;
use chrono::Utc;
use error_stack::{Report, ResultExt, bail};
use regex::Regex;
use reqwest::header::HeaderMap;
use serde_json::json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct YYDownloader {
    fake_headers: HeaderMap,
    client: reqwest::Client,
    url: String,
    name: String,
}

impl YYDownloader {
    fn new(client: reqwest::Client, url: &str, name: &str) -> YYDownloader {
        Self {
            fake_headers: Default::default(),
            client,
            url: url.to_string(),
            name: name.to_string(),
        }
    }
}

#[async_trait]
impl DownloadBase for YYDownloader {
    async fn check_stream(&mut self) -> Result<StreamStatus, Report<AppError>> {
        let mut fake_headers = self.fake_headers.clone();
        // 设置headers
        fake_headers.insert("content-type", "text/plain;charset=UTF-8".parse().unwrap());
        fake_headers.insert("referer", "https://www.yy.com/".parse().unwrap());

        // 提取房间ID
        let rid = match self.url.split("www.yy.com/").nth(1) {
            Some(part) => part.split('/').next().unwrap_or(""),
            None => {
                bail!(AppError::Custom("直播间地址错误".to_string()))
            }
        };

        if rid.is_empty() {
            bail!(AppError::Custom("rid 为空".to_string()))
        }

        // 获取时间戳
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("时间错误");
        let millis_13 = now.as_millis() as u64;
        let millis_10 = now.as_secs();

        // 构建JSON数据
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
        })
        .to_string();

        // 构建URL
        // 发送POST请求并处理响应
        let result = self
            .client
            .post(format!(
                "https://stream-manager.yy.com/v3/channel/streams?uid=0&cid={}&sid={}&appid=0&sequence={}&encode=json",
                rid, rid, millis_13
            ))
            .timeout(std::time::Duration::from_secs(30))
            .headers(self.fake_headers.clone())
            .body(data)
            .send()
            .await
            .change_context(AppError::Custom(format!("rid: {rid}")))?
            .json::<serde_json::Value>()
            .await
            .change_context(AppError::Custom(format!("解析json出错 rid: {rid}")))?;

        let Some(stream_url) = result
            .get("avp_info_res")
            .and_then(|info| info.get("stream_line_addr"))
            .and_then(|addr| addr.as_object())
            .and_then(|obj| obj.values().next())
            .and_then(|val| val.get("cdn_info"))
            .and_then(|cdn| cdn.get("url"))
            .and_then(|url| url.as_str())
        else {
            return Ok(StreamStatus::Offline);
        };
        let raw_stream_url = stream_url.to_string();
        Ok(StreamStatus::Live {
            stream_info: Box::new(StreamInfoExt {
                streamer_info: StreamerInfo {
                    id: -1,
                    name: self.name.clone(),
                    url: self.url.clone(),
                    title: "".to_string(),
                    date: Utc::now(),
                    live_cover_path: "".to_string(),
                },
                suffix: media_ext_from_url(&raw_stream_url).unwrap(),
                raw_stream_url,
                platform: String::from("YY"),
                stream_headers: HashMap::new(),
            }),
        })
    }
}

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

impl DownloadPlugin for YY {
    fn matches(&self, url: &str) -> bool {
        self.re.is_match(url)
    }

    fn create_downloader(&self, ctx: &mut PluginContext) -> Box<dyn DownloadBase> {
        let url = ctx.live_streamer().url.to_string();
        let name = ctx.live_streamer().remark.to_string();

        Box::new(YYDownloader::new(ctx.client(), &url, &name))
    }

    fn name(&self) -> &str {
        "YY"
    }
}
