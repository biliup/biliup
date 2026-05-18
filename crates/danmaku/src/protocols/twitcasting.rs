//! Twitcasting JSON protocol implementation.
//!
//! Twitcasting uses a simple JSON-over-WebSocket protocol:
//!
//! - Get WebSocket URL via HTTP POST to eventpubsuburl.php
//! - No heartbeat required
//! - Messages are JSON arrays containing comment objects

use std::time::Duration;

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, CACHE_CONTROL, REFERER, USER_AGENT};
use serde::Deserialize;

use crate::error::{DanmakuError, Result};
use crate::message::{ChatMessage, DanmakuEvent};
use crate::protocols::{
    ConnectionInfo, DecodeResult, HeartbeatConfig, Platform, PlatformContext,
};

/// User agent for Twitcasting requests.
const USER_AGENT_STRING: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// Response from eventpubsuburl.php
#[derive(Debug, Deserialize)]
struct EventPubSubResponse {
    url: String,
}

/// Comment message structure
#[derive(Debug, Deserialize)]
struct CommentMessage {
    message: String,
    #[serde(default)]
    from_user: Option<UserInfo>,
}

/// User info in comment
#[derive(Debug, Deserialize)]
struct UserInfo {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    id: Option<String>,
}

/// Twitcasting JSON protocol implementation.
pub struct Twitcasting {
    client: reqwest::Client,
}

impl Twitcasting {
    /// Create a new Twitcasting protocol handler.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Build default headers for requests.
    fn default_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br"));
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        headers.insert(REFERER, HeaderValue::from_static("https://twitcasting.tv/"));
        headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_STRING));
        headers
    }
}

impl Default for Twitcasting {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Platform for Twitcasting {
    fn name(&self) -> &'static str {
        "Twitcasting"
    }

    async fn get_connection_info(
        &self,
        _url: &str,
        context: &PlatformContext,
    ) -> Result<ConnectionInfo> {
        let movie_id = context
            .movie_id
            .as_ref()
            .ok_or_else(|| DanmakuError::Decode("movie_id is required for Twitcasting".to_string()))?;

        let password = context.password.as_deref().unwrap_or("");

        // Request WebSocket URL
        let resp: EventPubSubResponse = self
            .client
            .post("https://twitcasting.tv/eventpubsuburl.php")
            .headers(Self::default_headers())
            .form(&[("movie_id", movie_id.as_str()), ("password", password)])
            .timeout(Duration::from_secs(5))
            .send()
            .await?
            .json()
            .await?;

        Ok(ConnectionInfo::new(resp.url).with_headers(Self::default_headers()))
    }

    fn heartbeat_config(&self) -> HeartbeatConfig {
        // Twitcasting doesn't require heartbeat
        HeartbeatConfig::none()
    }

    fn decode_message(&self, msg: &[u8]) -> Result<DecodeResult> {
        let text = std::str::from_utf8(msg)
            .map_err(|e| DanmakuError::Decode(e.to_string()))?;

        let mut events = Vec::new();

        // Each line can contain a JSON array of comments
        for line in text.lines() {
            if line.is_empty() {
                continue;
            }

            // Try to parse as JSON array
            match serde_json::from_str::<Vec<CommentMessage>>(line) {
                Ok(comments) => {
                    for comment in comments {
                        let mut chat = ChatMessage::new(comment.message);

                        if let Some(user) = comment.from_user {
                            if let Some(name) = user.name {
                                chat = chat.with_name(name);
                            }
                        }

                        events.push(DanmakuEvent::Chat(chat));
                    }
                }
                Err(_) => {
                    // Not a comment message, might be control message
                    continue;
                }
            }
        }

        Ok(DecodeResult::with_events(events))
    }

    fn is_text_protocol(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_comment() {
        let twitcasting = Twitcasting::new();
        let msg = br#"[{"message":"Hello World!","from_user":{"name":"TestUser","id":"123"}}]"#;

        let result = twitcasting.decode_message(msg).unwrap();
        assert_eq!(result.events.len(), 1);

        if let DanmakuEvent::Chat(chat) = &result.events[0] {
            assert_eq!(chat.content, "Hello World!");
            assert_eq!(chat.name, Some("TestUser".to_string()));
        } else {
            panic!("Expected Chat event");
        }
    }

    #[test]
    fn test_decode_multiple_comments() {
        let twitcasting = Twitcasting::new();
        let msg = br#"[{"message":"First"},{"message":"Second"}]"#;

        let result = twitcasting.decode_message(msg).unwrap();
        assert_eq!(result.events.len(), 2);
    }
}
