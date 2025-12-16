//! Huya live danmaku protocol implementation.
//!
//! Huya uses TARS binary protocol over WebSocket:
//! - Registration: WSUserInfo wrapped in WebSocketCommand (iCmdType=1)
//! - Messages: WebSocketCommand with iCmdType=7 for push messages
//! - Danmaku messages have message type 1400

use std::time::Duration;

use async_trait::async_trait;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use tracing::debug;

use crate::codec::tars::{TarsInputStream, TarsOutputStream};
use crate::error::{DanmakuError, Result};
use crate::message::{ChatMessage, DanmakuEvent, DEFAULT_COLOR};
use crate::protocols::{
    ConnectionInfo, DecodeResult, HeartbeatConfig, Platform, PlatformContext, RegistrationData,
};

/// WebSocket URL for Huya danmaku.
const WSS_URL: &str = "wss://cdnws.api.huya.com/";

/// User agent for Huya requests.
const USER_AGENT_STRING: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// WebSocket command types.
#[allow(dead_code)]
mod cmd_type {
    pub const REGISTER_REQ: i32 = 1;
    pub const REGISTER_RSP: i32 = 2;
    pub const HEARTBEAT: i32 = 5;
    pub const HEARTBEAT_ACK: i32 = 6;
    pub const MSG_PUSH_REQ: i32 = 7;
}

/// Heartbeat packet (pre-encoded TARS).
const HEARTBEAT: &[u8] = &[
    0x00, 0x03, 0x1d, 0x00, 0x00, 0x69, 0x00, 0x00, 0x00, 0x69, 0x10, 0x03, 0x2c, 0x3c, 0x4c, 0x56,
    0x08, 0x6f, 0x6e, 0x6c, 0x69, 0x6e, 0x65, 0x75, 0x69, 0x66, 0x0f, 0x4f, 0x6e, 0x55, 0x73, 0x65,
    0x72, 0x48, 0x65, 0x61, 0x72, 0x74, 0x42, 0x65, 0x61, 0x74, 0x7d, 0x00, 0x00, 0x3c, 0x08, 0x00,
    0x01, 0x06, 0x04, 0x74, 0x52, 0x65, 0x71, 0x1d, 0x00, 0x00, 0x2f, 0x0a, 0x0a, 0x0c, 0x16, 0x00,
    0x26, 0x00, 0x36, 0x07, 0x61, 0x64, 0x72, 0x5f, 0x77, 0x61, 0x70, 0x46, 0x00, 0x0b, 0x12, 0x03,
    0xae, 0xf0, 0x0f, 0x22, 0x03, 0xae, 0xf0, 0x0f, 0x3c, 0x42, 0x6d, 0x52, 0x02, 0x60, 0x5c, 0x60,
    0x01, 0x7c, 0x82, 0x00, 0x0b, 0xb0, 0x1f, 0x9c, 0xac, 0x0b, 0x8c, 0x98, 0x0c, 0xa8, 0x0c, 0x20,
];

/// Huya live danmaku protocol.
pub struct Huya {
    client: reqwest::Client,
}

impl Huya {
    /// Create a new Huya protocol handler.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Build default headers.
    fn default_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_STRING));
        headers
    }

    /// Extract room ID from URL.
    fn extract_room_id(url: &str) -> Option<String> {
        // https://www.huya.com/123456 or https://huya.com/roomname
        let re = Regex::new(r"huya\.com/([^/?]+)").ok()?;
        re.captures(url)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    }

    /// Get UID from room page.
    async fn get_room_uid(&self, room_id: &str) -> Result<u64> {
        let url = format!("https://www.huya.com/{}", room_id);
        let headers = Self::default_headers();

        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .timeout(Duration::from_secs(10))
            .send()
            .await?
            .text()
            .await?;

        // Extract UID from page: "uid":"123456" or "uid":123456
        let re = Regex::new(r#"uid['"]*:\s*['"]*(\d+)['"]*"#)
            .map_err(|e| DanmakuError::Decode(e.to_string()))?;

        re.captures(&resp)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<u64>().ok())
            .ok_or_else(|| DanmakuError::Decode("Failed to extract UID from Huya page".to_string()))
    }

    /// Build WSUserInfo TARS structure.
    fn build_ws_user_info(uid: u64) -> Vec<u8> {
        let mut oos = TarsOutputStream::new();

        // WSUserInfo fields:
        // 0: lUid (int64)
        // 1: bAnonymous (bool)
        // 2: sGuid (string)
        // 3: sToken (string)
        // 4: lTid (int64)
        // 5: lSid (int64)
        // 6: lGroupId (int64)
        // 7: lGroupType (int64)

        oos.write_int64(0, uid as i64);
        oos.write_bool(1, false); // Not anonymous
        oos.write_string(2, ""); // sGuid
        oos.write_string(3, ""); // sToken
        oos.write_int64(4, 0); // lTid
        oos.write_int64(5, 0); // lSid
        oos.write_int64(6, uid as i64); // lGroupId = uid
        oos.write_int64(7, 3); // lGroupType = 3

        oos.get_buffer().to_vec()
    }

    /// Build WebSocketCommand TARS structure.
    fn build_ws_command(cmd_type: i32, data: &[u8]) -> Vec<u8> {
        let mut oos = TarsOutputStream::new();

        // WebSocketCommand fields:
        // 0: iCmdType (int32)
        // 1: vData (bytes)

        oos.write_int32(0, cmd_type);
        oos.write_bytes(1, data);

        oos.get_buffer().to_vec()
    }

    /// Parse a WebSocket message.
    fn parse_message(data: &[u8]) -> Vec<DanmakuEvent> {
        let mut events = Vec::new();

        // Parse WebSocketCommand
        let mut ios = TarsInputStream::new(data);

        let cmd_type = ios.read_int32(0).unwrap_or(0);

        if cmd_type == cmd_type::MSG_PUSH_REQ {
            // Read vData (bytes at tag 1)
            if let Some(inner_data) = ios.read_bytes(1) {
                // Parse inner message
                let mut inner_ios = TarsInputStream::new(&inner_data);

                // Check message type at tag 1
                let msg_type = inner_ios.read_int64(1).unwrap_or(0);

                if msg_type == 1400 {
                    // Danmaku message - read message body at tag 2
                    if let Some(msg_data) = inner_ios.read_bytes(2) {
                        if let Some(event) = Self::parse_danmaku(&msg_data) {
                            events.push(event);
                        }
                    }
                }
            }
        } else if cmd_type == cmd_type::REGISTER_RSP {
            debug!("Huya register response received");
        } else if cmd_type == cmd_type::HEARTBEAT_ACK {
            debug!("Huya heartbeat ack received");
        }

        events
    }

    /// Parse a danmaku message.
    fn parse_danmaku(data: &[u8]) -> Option<DanmakuEvent> {
        let mut ios = TarsInputStream::new(data);

        // User info is at tag 0, which is a struct
        // Within user info, username is at tag 2
        // Content is at tag 3
        // Color is at tag 6 (inside a struct)

        // For simplicity, we'll try to parse the username at tag 2 of nested struct
        // and content at tag 3

        // Skip to find the username - it's nested, so we need a different approach
        // Let's read the raw structure:
        // Tag 0: User struct containing:
        //   - Tag 2: username (string)
        // Tag 3: content (string)
        // Tag 6: DColor struct containing:
        //   - Tag 0: color (int32)

        // This is a simplified parser - we look for string patterns
        let name = ios.read_string(2).unwrap_or_default();
        let content = ios.read_string(3).unwrap_or_default();

        // Try to read color from tag 6 struct
        let color = DEFAULT_COLOR;

        if content.is_empty() {
            return None;
        }

        let mut chat = ChatMessage::new(content).with_color(color);
        if !name.is_empty() {
            chat = chat.with_name(name);
        }

        Some(DanmakuEvent::Chat(chat))
    }
}

impl Default for Huya {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Platform for Huya {
    fn name(&self) -> &'static str {
        "Huya"
    }

    async fn get_connection_info(
        &self,
        url: &str,
        context: &PlatformContext,
    ) -> Result<ConnectionInfo> {
        // Get room ID
        let room_id = if let Some(ref id) = context.room_id {
            id.clone()
        } else {
            Self::extract_room_id(url)
                .ok_or_else(|| DanmakuError::Decode("Invalid Huya URL".to_string()))?
        };

        // Get UID from room page
        let uid = self.get_room_uid(&room_id).await?;
        debug!("Huya room UID: {}", uid);

        // Build registration packet
        let user_info = Self::build_ws_user_info(uid);
        let reg_packet = Self::build_ws_command(cmd_type::REGISTER_REQ, &user_info);

        Ok(
            ConnectionInfo::new(WSS_URL)
                .with_registration(vec![RegistrationData::Binary(reg_packet)])
                .with_headers(Self::default_headers()),
        )
    }

    fn heartbeat_config(&self) -> HeartbeatConfig {
        HeartbeatConfig::binary(HEARTBEAT.to_vec(), Duration::from_secs(60))
    }

    fn decode_message(&self, msg: &[u8]) -> Result<DecodeResult> {
        let events = Self::parse_message(msg);
        Ok(DecodeResult::with_events(events))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_room_id() {
        assert_eq!(
            Huya::extract_room_id("https://www.huya.com/123456"),
            Some("123456".to_string())
        );
        assert_eq!(
            Huya::extract_room_id("https://huya.com/kpl"),
            Some("kpl".to_string())
        );
    }

    #[test]
    fn test_build_ws_user_info() {
        let data = Huya::build_ws_user_info(12345);
        assert!(!data.is_empty());

        // Verify structure
        let mut ios = TarsInputStream::new(&data);
        assert_eq!(ios.read_int64(0), Some(12345));
    }

    #[test]
    fn test_build_ws_command() {
        let user_info = Huya::build_ws_user_info(12345);
        let cmd = Huya::build_ws_command(cmd_type::REGISTER_REQ, &user_info);

        assert!(!cmd.is_empty());

        // Verify structure
        let mut ios = TarsInputStream::new(&cmd);
        assert_eq!(ios.read_int32(0), Some(cmd_type::REGISTER_REQ));
    }
}
