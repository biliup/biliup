//! Douyin (TikTok China) live danmaku protocol implementation.
//!
//! Douyin uses Protocol Buffers over WebSocket:
//! - PushFrame wrapper with gzip-compressed payload
//! - Response containing message list
//! - ChatMessage for danmaku content
//!
//! NOTE: This implementation requires a valid signature to connect.
//! The signature calculation requires JavaScript execution (webmssdk.js).
//! Without proper signature, connections will likely be rejected.

use std::io::Read;
use std::time::Duration;

use async_trait::async_trait;
use flate2::read::GzDecoder;
use regex::Regex;
use tracing::debug;

use crate::codec::protobuf::{ProtoReader, ProtoWriter};
use crate::error::{DanmakuError, Result};
use crate::message::{ChatMessage, DanmakuEvent, DEFAULT_COLOR};
use crate::protocols::{
    ConnectionInfo, DecodeResult, HeartbeatConfig, Platform, PlatformContext,
};

/// Heartbeat packet.
const HEARTBEAT: &[u8] = b":\x02hb";

/// Douyin live danmaku protocol.
pub struct Douyin;

impl Douyin {
    /// Create a new Douyin protocol handler.
    pub fn new() -> Self {
        Self
    }

    /// Extract room ID from URL.
    fn extract_room_id(url: &str) -> Option<String> {
        // https://live.douyin.com/123456789
        let re = Regex::new(r"live\.douyin\.com/(\d+)").ok()?;
        re.captures(url)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    }

    /// Generate a random user unique ID.
    fn generate_user_unique_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let base = 7300000000000000000u64;
        let range = 699999999999999999u64;
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        let id = base + (ts % range);
        id.to_string()
    }

    /// Generate X-MS-Stub (MD5 of params).
    fn get_x_ms_stub(params: &[(&str, &str)]) -> String {
        let sig_params: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(",");

        let digest = md5::compute(sig_params.as_bytes());
        format!("{:x}", digest)
    }

    /// Generate a placeholder signature.
    /// NOTE: Real signature requires JavaScript execution.
    /// This returns a placeholder that will likely not work.
    fn get_signature(_x_ms_stub: &str) -> String {
        // Without JS execution, we cannot generate a valid signature.
        // Return a placeholder - connection will likely fail.
        "00000000".to_string()
    }

    /// Build WebSocket URL with parameters.
    fn build_ws_url(room_id: &str) -> String {
        let user_unique_id = Self::generate_user_unique_id();
        let version_code = "180800";
        let webcast_sdk_version = "1.0.14-beta.0";

        // Parameters for signature
        let sig_params = [
            ("live_id", "1"),
            ("aid", "6383"),
            ("version_code", version_code),
            ("webcast_sdk_version", webcast_sdk_version),
            ("room_id", room_id),
            ("sub_room_id", ""),
            ("sub_channel_id", ""),
            ("did_rule", "3"),
            ("user_unique_id", &user_unique_id),
            ("device_platform", "web"),
            ("device_type", ""),
            ("ac", ""),
            ("identity", "audience"),
        ];

        let x_ms_stub = Self::get_x_ms_stub(&sig_params);
        let signature = Self::get_signature(&x_ms_stub);

        // WebSocket URL parameters
        let ws_params = [
            ("room_id", room_id.to_string()),
            ("compress", "gzip".to_string()),
            ("version_code", version_code.to_string()),
            ("webcast_sdk_version", webcast_sdk_version.to_string()),
            ("live_id", "1".to_string()),
            ("did_rule", "3".to_string()),
            ("user_unique_id", user_unique_id),
            ("identity", "audience".to_string()),
            ("signature", signature),
        ];

        let query: String = ws_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        format!(
            "wss://webcast5-ws-web-lf.douyin.com/webcast/im/push/v2/?{}",
            query
        )
    }

    /// Decompress gzip data.
    fn decompress_gzip(data: &[u8]) -> Result<Vec<u8>> {
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| DanmakuError::Compression(format!("gzip: {}", e)))?;
        Ok(decompressed)
    }

    /// Parse PushFrame.
    /// Fields: 1=seqId, 2=logId, 7=payloadType, 8=payload
    fn parse_push_frame(data: &[u8]) -> Option<(u64, Vec<u8>, String)> {
        let mut reader = ProtoReader::new(data);
        let fields = reader.parse_all();

        let log_id = fields.get(&2)?.first()?.as_u64()?;
        let payload_type = fields
            .get(&7)
            .and_then(|v| v.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let payload = fields.get(&8)?.first()?.as_bytes()?.to_vec();

        Some((log_id, payload, payload_type))
    }

    /// Parse Response.
    /// Fields: 1=messagesList, 5=internalExt, 9=needAck
    fn parse_response(data: &[u8]) -> Option<(Vec<(String, Vec<u8>)>, bool, String)> {
        let mut reader = ProtoReader::new(data);
        let fields = reader.parse_all();

        let mut messages = Vec::new();

        // Parse messages list (field 1, repeated)
        if let Some(msg_list) = fields.get(&1) {
            for msg_value in msg_list {
                if let Some(msg_bytes) = msg_value.as_bytes() {
                    // Parse individual message
                    // Fields: 1=method, 2=payload
                    let mut msg_reader = ProtoReader::new(msg_bytes);
                    let msg_fields = msg_reader.parse_all();

                    let method = msg_fields
                        .get(&1)
                        .and_then(|v| v.first())
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let payload = msg_fields
                        .get(&2)
                        .and_then(|v| v.first())
                        .and_then(|v| v.as_bytes())
                        .unwrap_or(&[])
                        .to_vec();

                    messages.push((method, payload));
                }
            }
        }

        let need_ack = fields
            .get(&9)
            .and_then(|v| v.first())
            .and_then(|v| v.as_u64())
            .map(|v| v != 0)
            .unwrap_or(false);

        let internal_ext = fields
            .get(&5)
            .and_then(|v| v.first())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Some((messages, need_ack, internal_ext))
    }

    /// Parse ChatMessage.
    /// Fields: 2=user(3=nickName), 3=content
    fn parse_chat_message(data: &[u8]) -> Option<(String, String)> {
        let mut reader = ProtoReader::new(data);
        let fields = reader.parse_all();

        let content = fields
            .get(&3)
            .and_then(|v| v.first())
            .and_then(|v| v.as_str())?
            .to_string();

        // Try to get username from user field (field 2)
        let name = fields
            .get(&2)
            .and_then(|v| v.first())
            .and_then(|v| v.as_bytes())
            .and_then(|user_bytes| {
                let mut user_reader = ProtoReader::new(user_bytes);
                let user_fields = user_reader.parse_all();
                user_fields
                    .get(&3) // nickName
                    .and_then(|v| v.first())
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_default();

        Some((name, content))
    }

    /// Build ACK packet.
    fn build_ack(log_id: u64, internal_ext: &str) -> Vec<u8> {
        let mut writer = ProtoWriter::new();
        writer.write_varint_field(2, log_id);
        writer.write_string(7, internal_ext);
        writer.into_buffer()
    }
}

impl Default for Douyin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Platform for Douyin {
    fn name(&self) -> &'static str {
        "Douyin"
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
                .ok_or_else(|| DanmakuError::Decode("Invalid Douyin URL".to_string()))?
        };

        let ws_url = Self::build_ws_url(&room_id);
        debug!("Douyin WebSocket URL: {}", ws_url);

        // Note: Without proper signature, connection will likely fail
        Ok(ConnectionInfo::new(ws_url))
    }

    fn heartbeat_config(&self) -> HeartbeatConfig {
        HeartbeatConfig::binary(HEARTBEAT.to_vec(), Duration::from_secs(10))
    }

    fn decode_message(&self, msg: &[u8]) -> Result<DecodeResult> {
        let mut events = Vec::new();
        let mut ack_data = None;

        // Parse PushFrame
        if let Some((log_id, payload, _payload_type)) = Self::parse_push_frame(msg) {
            // Decompress payload
            if let Ok(decompressed) = Self::decompress_gzip(&payload) {
                // Parse Response
                if let Some((messages, need_ack, internal_ext)) =
                    Self::parse_response(&decompressed)
                {
                    // Build ACK if needed
                    if need_ack {
                        ack_data = Some(Self::build_ack(log_id, &internal_ext));
                    }

                    // Process messages
                    for (method, msg_payload) in messages {
                        if method == "WebcastChatMessage" {
                            if let Some((name, content)) = Self::parse_chat_message(&msg_payload) {
                                let mut chat = ChatMessage::new(content).with_color(DEFAULT_COLOR);
                                if !name.is_empty() {
                                    chat = chat.with_name(name);
                                }
                                events.push(DanmakuEvent::Chat(chat));
                            }
                        }
                    }
                }
            }
        }

        let mut result = DecodeResult::with_events(events);
        if let Some(ack) = ack_data {
            result = result.with_ack(ack);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_room_id() {
        assert_eq!(
            Douyin::extract_room_id("https://live.douyin.com/123456789"),
            Some("123456789".to_string())
        );
    }

    #[test]
    fn test_generate_user_unique_id() {
        let id = Douyin::generate_user_unique_id();
        let num: u64 = id.parse().unwrap();
        assert!(num >= 7300000000000000000);
        assert!(num < 8000000000000000000);
    }

    #[test]
    fn test_get_x_ms_stub() {
        let params = [("room_id", "123"), ("live_id", "1")];
        let stub = Douyin::get_x_ms_stub(&params);
        assert_eq!(stub.len(), 32); // MD5 hex length
    }
}
