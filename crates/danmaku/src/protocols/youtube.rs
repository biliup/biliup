//! YouTube live chat polling implementation.

use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE;
use rand::Rng;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde_json::{Value, json};
use urlencoding::encode;

use crate::error::{DanmakuError, Result};
use crate::message::{ChatMessage, DanmakuEvent};
use crate::protocols::{ConnectionInfo, DecodeResult, HeartbeatConfig, Platform, PlatformContext};

const USER_AGENT_STRING: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const API_KEY: &str = "AIzaSyAO_FJ2SlqU84TSTEHLGCiIlwY_Y9_11qcW8";

pub struct YouTube {
    client: reqwest::Client,
}

impl YouTube {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    fn default_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_STRING));
        headers
    }

    fn extract_channel_id(text: &str) -> Option<String> {
        [
            r#""channelId":"(UC[^"\\]{22})""#,
            r#"\\"channelId\\":\\"(UC[^"\\]{22})\\""#,
            r#""externalChannelId":"(UC[^"\\]{22})""#,
            r#"\\"externalChannelId\\":\\"(UC[^"\\]{22})\\""#,
            r#""browseId":"(UC[^"\\]{22})""#,
            r#"\\"browseId\\":\\"(UC[^"\\]{22})\\""#,
            r#"/channel/(UC[A-Za-z0-9_-]{22})"#,
            r#"(UC[A-Za-z0-9_-]{22})"#,
        ]
        .into_iter()
        .find_map(|pattern| {
            Regex::new(pattern)
                .ok()?
                .captures(text)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str().to_string())
        })
    }

    fn extract_live_video_id(text: &str) -> Option<String> {
        let re = Regex::new(
            r#""gridVideoRenderer"(?s:.+?)"label":"(?:LIVE|LIVE NOW|PREMIERING NOW)"(?s:.+?)"videoId":"([^"]+)""#,
        )
        .ok()?;
        re.captures(text)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    }

    async fn resolve_channel_id(&self, url: &str) -> Result<String> {
        if let Some(caps) = Regex::new(r"youtube\.com/channel/([^/?]+)")
            .map_err(|e| DanmakuError::Decode(e.to_string()))?
            .captures(url)
            && let Some(channel_id) = caps.get(1)
        {
            return Ok(channel_id.as_str().to_string());
        }

        let video_id = Regex::new(r"(?:youtube\.com/watch\?v=|youtu\.be/)([^/?&]+)")
            .map_err(|e| DanmakuError::Decode(e.to_string()))?
            .captures(url)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .ok_or_else(|| DanmakuError::Decode("Invalid YouTube URL".to_string()))?;

        let text = self
            .client
            .get(format!("https://www.youtube.com/embed/{video_id}"))
            .headers(Self::default_headers())
            .send()
            .await?
            .text()
            .await?;

        Self::extract_channel_id(&text)
            .ok_or_else(|| DanmakuError::Decode("YouTube channel id not found".to_string()))
    }

    async fn resolve_live_video_id(&self, channel_id: &str) -> Result<String> {
        let text = self
            .client
            .get(format!(
                "https://www.youtube.com/channel/{channel_id}/videos"
            ))
            .headers(Self::default_headers())
            .send()
            .await?
            .text()
            .await?;

        Self::extract_live_video_id(&text)
            .ok_or_else(|| DanmakuError::Decode("YouTube live video id not found".to_string()))
    }

    async fn initial_continuation(&self, url: &str) -> Result<String> {
        let channel_id = self.resolve_channel_id(url).await?;
        let video_id = match extract_video_id(url) {
            Some(video_id) => video_id,
            None => self.resolve_live_video_id(&channel_id).await?,
        };
        Ok(liveparam(&video_id, &channel_id, 1, false))
    }

    async fn initial_replay_continuation(
        &self,
        url: &str,
        context: &PlatformContext,
    ) -> Result<String> {
        let video_id = context
            .extra
            .get("video_id")
            .cloned()
            .or_else(|| extract_video_id(url))
            .ok_or_else(|| DanmakuError::Decode("YouTube replay video id not found".to_string()))?;
        let channel_id = match context.extra.get("channel_id") {
            Some(channel_id) => channel_id.clone(),
            None => self.resolve_channel_id(url).await?,
        };
        let seek_time = context
            .extra
            .get("seek_time")
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or_default();
        let topchat_only = context
            .extra
            .get("topchat_only")
            .map(|value| matches!(value.as_str(), "1" | "true" | "True"))
            .unwrap_or(false);

        Ok(arcparam(&video_id, seek_time, topchat_only, &channel_id))
    }

    fn api_url() -> String {
        format!(
            "https://www.youtube.com/youtubei/v1/live_chat/get_live_chat?key={}",
            API_KEY
        )
    }
}

impl Default for YouTube {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Platform for YouTube {
    fn name(&self) -> &'static str {
        "YouTube"
    }

    async fn get_connection_info(
        &self,
        url: &str,
        context: &PlatformContext,
    ) -> Result<ConnectionInfo> {
        let continuation = match context.extra.get("continuation") {
            Some(value) => value.clone(),
            None if context
                .extra
                .get("replay")
                .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "True")) =>
            {
                self.initial_replay_continuation(url, context).await?
            }
            None => self.initial_continuation(url).await?,
        };
        Ok(ConnectionInfo::new(format!(
            "poll://youtube?continuation={continuation}"
        )))
    }

    fn heartbeat_config(&self) -> HeartbeatConfig {
        HeartbeatConfig::none()
    }

    fn decode_message(&self, _msg: &[u8]) -> Result<DecodeResult> {
        Ok(DecodeResult::empty())
    }

    async fn poll_messages(
        &self,
        _url: &str,
        context: &mut PlatformContext,
    ) -> Result<Vec<DanmakuEvent>> {
        let continuation = context
            .extra
            .remove("continuation")
            .ok_or_else(|| DanmakuError::Decode("YouTube continuation missing".to_string()))?;

        let body = json!({
            "context": {
                "client": {
                    "visitorData": "",
                    "userAgent": USER_AGENT_STRING,
                    "clientName": "WEB",
                    "clientVersion": youtube_client_version(),
                }
            },
            "continuation": continuation,
        });

        let value: Value = self
            .client
            .post(Self::api_url())
            .headers(Self::default_headers())
            .json(&body)
            .send()
            .await?
            .json()
            .await?;

        let live_chat = value
            .get("continuationContents")
            .and_then(|v| v.get("liveChatContinuation"))
            .ok_or_else(|| {
                DanmakuError::Decode("YouTube live chat continuation missing".to_string())
            })?;

        let continuation = live_chat
            .get("continuations")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(next_continuation)
            .ok_or_else(|| DanmakuError::Decode("YouTube next continuation missing".to_string()))?;
        context
            .extra
            .insert("continuation".to_string(), continuation);

        let events = live_chat
            .get("actions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(parse_action)
            .collect();

        Ok(events)
    }

    fn poll_interval(&self) -> Duration {
        Duration::from_secs(1)
    }
}

fn extract_video_id(url: &str) -> Option<String> {
    Regex::new(r"(?:youtube\.com/watch\?v=|youtu\.be/)([^/?&]+)")
        .ok()?
        .captures(url)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn next_continuation(value: &Value) -> Option<String> {
    [
        "invalidationContinuationData",
        "timedContinuationData",
        "reloadContinuationData",
        "liveChatReplayContinuationData",
    ]
    .into_iter()
    .find_map(|key| {
        value
            .get(key)
            .and_then(|v| v.get("continuation"))
            .and_then(Value::as_str)
            .map(str::to_string)
    })
}

fn parse_action(action: &Value) -> Option<DanmakuEvent> {
    let renderer = action
        .get("addChatItemAction")?
        .get("item")?
        .get("liveChatTextMessageRenderer")?;
    let name = renderer
        .get("authorName")?
        .get("simpleText")?
        .as_str()?
        .to_string();
    let content = renderer
        .get("message")?
        .get("runs")?
        .as_array()?
        .iter()
        .filter_map(|run| {
            if let Some(emoji) = run.get("emoji") {
                emoji.get("shortcuts")?.as_array()?.first()?.as_str()
            } else {
                run.get("text")?.as_str()
            }
        })
        .collect::<String>();

    Some(DanmakuEvent::Chat(
        ChatMessage::new(content).with_name(name),
    ))
}

fn youtube_client_version() -> String {
    let date = chrono::Utc::now()
        .date_naive()
        .pred_opt()
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    format!("2.{}.01.00", date.format("%Y%m%d"))
}

fn liveparam(video_id: &str, channel_id: &str, past_sec: u64, topchat_only: bool) -> String {
    let times = liveparam_times(past_sec);
    build_liveparam(
        video_id,
        channel_id,
        times[0],
        times[1],
        times[2],
        times[3],
        times[4],
        topchat_only,
    )
}

fn liveparam_times(past_sec: u64) -> [u64; 5] {
    let now = chrono::Utc::now().timestamp() as f64;
    let mut rng = rand::thread_rng();
    [
        ((now - rng.gen_range(0.0..3.0)) * 1_000_000.0) as u64,
        ((now - rng.gen_range(0.01..0.99)) * 1_000_000.0) as u64,
        ((now - past_sec as f64 + rng.gen_range(0.0..1.0)) * 1_000_000.0) as u64,
        ((now - rng.gen_range(600.0..3600.0)) * 1_000_000.0) as u64,
        ((now - rng.gen_range(0.01..0.99)) * 1_000_000.0) as u64,
    ]
}

fn build_liveparam(
    video_id: &str,
    channel_id: &str,
    ts1: u64,
    ts2: u64,
    ts3: u64,
    ts4: u64,
    ts5: u64,
    topchat_only: bool,
) -> String {
    let chat_type = if topchat_only { 4 } else { 1 };

    let body = [
        nm(1, 0),
        nm(2, 0),
        nm(3, 0),
        nm(4, 0),
        rs(7, b""),
        nm(8, 0),
        rs(9, b""),
        nm(10, ts2),
        nm(11, 3),
        nm(15, 0),
    ]
    .concat();

    let entity = [
        rs(3, &liveparam_header(video_id, channel_id)),
        nm(5, ts1),
        nm(6, 0),
        nm(7, 0),
        nm(8, 1),
        rs(9, &body),
        nm(10, ts3),
        nm(11, ts4),
        nm(13, chat_type),
        rs(16, &nm(1, chat_type)),
        nm(17, 0),
        rs(19, &nm(1, 0)),
        nm(20, ts5),
    ]
    .concat();

    encode(&URL_SAFE.encode(rs(119693434, &entity))).into_owned()
}

fn arcparam(video_id: &str, seek_time: f64, _topchat_only: bool, channel_id: &str) -> String {
    let timestamp = (seek_time.max(0.0) * 1_000_000.0) as u64;
    let entity = [
        rs(3, &liveparam_header(video_id, channel_id)),
        nm(5, timestamp),
        nm(6, 0),
        nm(7, 0),
        nm(8, 0),
        nm(9, 4),
        rs(10, &nm(4, 0)),
        rs(14, &nm(1, 4)),
        nm(15, 0),
    ]
    .concat();

    encode(&URL_SAFE.encode(rs(156074452, &entity))).into_owned()
}

fn liveparam_header(video_id: &str, channel_id: &str) -> Vec<u8> {
    let s1_3 = rs(1, video_id.as_bytes());
    let s1_5 = [rs(1, channel_id.as_bytes()), rs(2, video_id.as_bytes())].concat();
    let s1 = [rs(3, &s1_3), rs(5, &s1_5)].concat();
    let s3 = rs(48687757, &rs(1, video_id.as_bytes()));
    URL_SAFE
        .encode([rs(1, &s1), rs(3, &s3), nm(4, 1)].concat())
        .into_bytes()
}

fn vn(mut value: u64) -> Vec<u8> {
    let mut buf = Vec::new();
    while value >> 7 != 0 {
        buf.push((value as u8) | 0x80);
        value >>= 7;
    }
    buf.push(value as u8);
    buf
}

fn tp(wire_type: u64, field: u64, data: Vec<u8>) -> Vec<u8> {
    [vn((field << 3) | wire_type), data].concat()
}

fn rs(field: u64, data: &[u8]) -> Vec<u8> {
    tp(2, field, [vn(data.len() as u64), data.to_vec()].concat())
}

fn nm(field: u64, value: u64) -> Vec<u8> {
    tp(0, field, vn(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_action() {
        let action = json!({
            "addChatItemAction": {
                "item": {
                    "liveChatTextMessageRenderer": {
                        "authorName": { "simpleText": "Alice" },
                        "message": { "runs": [{ "text": "hello" }] }
                    }
                }
            }
        });

        let event = parse_action(&action).unwrap();
        match event {
            DanmakuEvent::Chat(chat) => {
                assert_eq!(chat.name.as_deref(), Some("Alice"));
                assert_eq!(chat.content, "hello");
            }
            _ => panic!("expected chat"),
        }
    }

    #[test]
    fn test_liveparam_is_url_encoded() {
        let param = liveparam("video", "channel", 1, false);
        assert!(!param.is_empty());
        assert!(!param.contains('='));
    }

    #[test]
    fn test_arcparam_is_url_encoded() {
        let param = arcparam("video", -1.0, false, "channel");
        assert!(!param.is_empty());
        assert!(!param.contains('='));
    }
}
