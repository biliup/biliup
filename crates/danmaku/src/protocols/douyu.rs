//! Douyu live danmaku protocol implementation.
//!
//! Douyu uses STT (Serialized Text Transfer) format over WebSocket:
//! - Binary header: length (4 bytes LE) + length (4 bytes LE) + type (4 bytes LE)
//! - Body: STT-encoded text + null terminator
//! - Message types: loginreq, joingroup, chatmsg, dgb (gift), uenter

use std::time::Duration;

use async_trait::async_trait;
use regex::Regex;
use tracing::debug;

use crate::codec::stt;
use crate::error::{DanmakuError, Result};
use crate::message::{ChatMessage, DanmakuEvent, EnterMessage, GiftMessage, DEFAULT_COLOR};
use crate::protocols::{
    ConnectionInfo, DecodeResult, HeartbeatConfig, Platform, PlatformContext, RegistrationData,
};

/// WebSocket URL for Douyu danmaku.
const WSS_URL: &str = "wss://danmuproxy.douyu.com:8506/";

/// Message type code.
const MSG_TYPE: u32 = 689; // 0x02b1

/// Heartbeat packet: type@=mrkl/
const HEARTBEAT: &[u8] = &[
    0x14, 0x00, 0x00, 0x00, // length = 20
    0x14, 0x00, 0x00, 0x00, // length = 20
    0xb1, 0x02, 0x00, 0x00, // type = 689
    0x74, 0x79, 0x70, 0x65, 0x40, 0x3d, 0x6d, 0x72, 0x6b, 0x6c, 0x2f, 0x00, // type@=mrkl/\0
];

/// Color mapping for Douyu danmaku.
fn color_from_code(code: Option<&str>) -> u32 {
    match code {
        Some("0") => 16777215, // white
        Some("1") => 16717077, // red
        Some("2") => 2000880,  // green
        Some("3") => 8046667,  // blue
        Some("4") => 16744192, // orange
        Some("5") => 10172916, // purple
        Some("6") => 16738740, // pink
        _ => DEFAULT_COLOR,
    }
}

/// Douyu live danmaku protocol.
pub struct Douyu;

impl Douyu {
    /// Create a new Douyu protocol handler.
    pub fn new() -> Self {
        Self
    }

    /// Extract room ID from URL.
    fn extract_room_id(url: &str) -> Option<String> {
        let re = Regex::new(r"douyu\.com/(\d+)").ok()?;
        re.captures(url)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    }

    /// Build a STT packet with header.
    fn build_packet(data: &str) -> Vec<u8> {
        let body = data.as_bytes();
        let length = (9 + body.len()) as u32; // header(8) + type(4) - length(4) + body + null

        let mut packet = Vec::with_capacity(12 + body.len() + 1);

        // Length (twice, little-endian)
        packet.extend_from_slice(&length.to_le_bytes());
        packet.extend_from_slice(&length.to_le_bytes());

        // Message type (little-endian)
        packet.extend_from_slice(&MSG_TYPE.to_le_bytes());

        // Body + null terminator
        packet.extend_from_slice(body);
        packet.push(0x00);

        packet
    }

    /// Parse messages from raw data.
    fn parse_messages(data: &[u8]) -> Vec<DanmakuEvent> {
        let mut events = Vec::new();

        // Find all messages (they end with null byte and start with "type@=")
        let mut start = 0;
        while start < data.len() {
            // Skip header (12 bytes)
            if start + 12 > data.len() {
                break;
            }

            let length = u32::from_le_bytes([
                data[start],
                data[start + 1],
                data[start + 2],
                data[start + 3],
            ]) as usize;

            if start + 4 + length > data.len() {
                break;
            }

            // Extract body (skip header, remove null terminator)
            let body_start = start + 12;
            let body_end = start + 4 + length - 1; // -1 for null terminator

            if body_end > body_start && body_end <= data.len() {
                if let Ok(text) = std::str::from_utf8(&data[body_start..body_end]) {
                    if let Some(event) = Self::parse_stt_message(text) {
                        events.push(event);
                    }
                }
            }

            start += 4 + length;
        }

        events
    }

    /// Parse a single STT message.
    fn parse_stt_message(text: &str) -> Option<DanmakuEvent> {
        let msg = stt::decode(text);

        let msg_type = msg.get_str("type")?;

        match msg_type {
            "chatmsg" => {
                let name = msg.get_str("nn").unwrap_or("").to_string();
                let content = msg.get_str("txt").unwrap_or("").to_string();
                let color = color_from_code(msg.get_str("col"));

                if content.is_empty() {
                    return None;
                }

                let mut chat = ChatMessage::new(content).with_color(color);
                if !name.is_empty() {
                    chat = chat.with_name(name);
                }
                if let Some(uid_str) = msg.get_str("uid") {
                    if let Ok(uid) = uid_str.parse::<u64>() {
                        chat = chat.with_uid(uid);
                    }
                }

                Some(DanmakuEvent::Chat(chat))
            }
            "dgb" => {
                // Gift message
                let name = msg.get_str("nn").unwrap_or("").to_string();
                let gift_name = msg.get_str("gfn").unwrap_or("礼物").to_string();
                let num: u32 = msg.get_str("gfcnt").and_then(|s| s.parse().ok()).unwrap_or(1);
                let uid: u64 = msg.get_str("uid").and_then(|s| s.parse().ok()).unwrap_or(0);

                let content = format!("{}投喂了{}个{}", name, num, gift_name);

                Some(DanmakuEvent::Gift(GiftMessage {
                    name,
                    uid,
                    gift_name,
                    price: 0, // Douyu doesn't provide price in basic message
                    num,
                    content,
                    timestamp: chrono::Utc::now(),
                }))
            }
            "uenter" => {
                let name = msg.get_str("nn").unwrap_or("").to_string();
                let uid: Option<u64> = msg.get_str("uid").and_then(|s| s.parse().ok());

                Some(DanmakuEvent::Enter(EnterMessage {
                    name,
                    uid,
                    timestamp: chrono::Utc::now(),
                }))
            }
            "loginres" => {
                debug!("Douyu login response received");
                None
            }
            _ => {
                debug!("Unknown Douyu message type: {}", msg_type);
                None
            }
        }
    }
}

impl Default for Douyu {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Platform for Douyu {
    fn name(&self) -> &'static str {
        "Douyu"
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
                .ok_or_else(|| DanmakuError::Decode("Invalid Douyu URL".to_string()))?
        };

        // Build registration packets
        let login_packet = Self::build_packet(&format!("type@=loginreq/roomid@={}/", room_id));
        let join_packet =
            Self::build_packet(&format!("type@=joingroup/rid@={}/gid@=-9999/", room_id));

        Ok(ConnectionInfo::new(WSS_URL).with_registration(vec![
            RegistrationData::Binary(login_packet),
            RegistrationData::Binary(join_packet),
        ]))
    }

    fn heartbeat_config(&self) -> HeartbeatConfig {
        HeartbeatConfig::binary(HEARTBEAT.to_vec(), Duration::from_secs(30))
    }

    fn decode_message(&self, msg: &[u8]) -> Result<DecodeResult> {
        let events = Self::parse_messages(msg);
        Ok(DecodeResult::with_events(events))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_room_id() {
        assert_eq!(
            Douyu::extract_room_id("https://www.douyu.com/123456"),
            Some("123456".to_string())
        );
        assert_eq!(
            Douyu::extract_room_id("https://douyu.com/789"),
            Some("789".to_string())
        );
    }

    #[test]
    fn test_build_packet() {
        let data = "type@=mrkl/";
        let packet = Douyu::build_packet(data);

        // Length should be 9 + 11 = 20
        assert_eq!(packet[0..4], [0x14, 0x00, 0x00, 0x00]);
        assert_eq!(packet[4..8], [0x14, 0x00, 0x00, 0x00]);
        assert_eq!(packet[8..12], [0xb1, 0x02, 0x00, 0x00]);
        assert_eq!(&packet[12..23], data.as_bytes());
        assert_eq!(packet[23], 0x00);
    }

    #[test]
    fn test_parse_stt_message() {
        let text = "type@=chatmsg/nn@=TestUser/txt@=Hello World/col@=1/";
        let event = Douyu::parse_stt_message(text);

        assert!(event.is_some());
        if let Some(DanmakuEvent::Chat(chat)) = event {
            assert_eq!(chat.name, Some("TestUser".to_string()));
            assert_eq!(chat.content, "Hello World");
            assert_eq!(chat.color, 16717077); // red
        }
    }
}
