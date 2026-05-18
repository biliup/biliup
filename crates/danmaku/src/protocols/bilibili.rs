//! Bilibili live danmaku protocol implementation.
//!
//! Bilibili uses a binary protocol over WebSocket with the following features:
//!
//! - 16-byte header with packet length, version, operation code
//! - Version 2: zlib compressed payload
//! - Version 3: brotli compressed payload
//! - JSON messages for chat, gifts, super chat, etc.

use std::collections::BTreeMap;
use std::io::Read;
use std::time::Duration;

use async_trait::async_trait;
use byteorder::{BigEndian, ByteOrder};
use flate2::read::ZlibDecoder;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, COOKIE, ORIGIN, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::debug;

use crate::error::{DanmakuError, Result};
use crate::message::{ChatMessage, DanmakuEvent, GiftMessage, GuardBuyMessage, SuperChatMessage, DEFAULT_COLOR};
use crate::protocols::{
    ConnectionInfo, DecodeResult, HeartbeatConfig, Platform, PlatformContext, RegistrationData,
};
use crate::protocols::wbi::WbiSigner;

/// User agent for Bilibili requests.
const USER_AGENT_STRING: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// Default WebSocket URL.
const DEFAULT_WS_URL: &str = "wss://broadcastlv.chat.bilibili.com/sub";

/// Heartbeat packet.
/// Header: len=31, header_len=16, ver=1, op=2, seq=1
/// Body: "[object Object] "
const HEARTBEAT: &[u8] = &[
    0x00, 0x00, 0x00, 0x1f, // packet length = 31
    0x00, 0x10,             // header length = 16
    0x00, 0x01,             // version = 1
    0x00, 0x00, 0x00, 0x02, // operation = 2 (heartbeat)
    0x00, 0x00, 0x00, 0x01, // sequence = 1
    // "[object Object] "
    0x5b, 0x6f, 0x62, 0x6a, 0x65, 0x63, 0x74, 0x20,
    0x4f, 0x62, 0x6a, 0x65, 0x63, 0x74, 0x5d, 0x20,
];

/// Operation codes.
#[allow(dead_code)]
mod op {
    pub const HEARTBEAT: u32 = 2;
    pub const HEARTBEAT_REPLY: u32 = 3;
    pub const NOTIFICATION: u32 = 5;
    pub const AUTH: u32 = 7;
    pub const AUTH_REPLY: u32 = 8;
}

/// Protocol versions.
mod ver {
    pub const RAW_JSON: u16 = 0;
    pub const POPULARITY: u16 = 1;
    pub const ZLIB: u16 = 2;
    pub const BROTLI: u16 = 3;
}

/// Room init API response.
#[derive(Debug, Deserialize)]
struct RoomInitResponse {
    data: RoomInitData,
}

#[derive(Debug, Deserialize)]
struct RoomInitData {
    room_id: u64,
}

// Note: DanmuInfo API response is now parsed manually via serde_json::Value
// to handle error responses gracefully (e.g., -352 risk control).

/// Authentication data sent to WebSocket.
#[derive(Debug, Serialize)]
struct AuthData {
    uid: u64,
    roomid: u64,
    protover: u8,
    platform: &'static str,
    #[serde(rename = "type")]
    auth_type: u8,
    key: String,
}

/// Bilibili live danmaku protocol.
pub struct Bilibili {
    client: reqwest::Client,
    wbi_signer: WbiSigner,
}

impl Bilibili {
    /// Create a new Bilibili protocol handler.
    pub fn new() -> Self {
        let client = reqwest::Client::new();
        let wbi_signer = WbiSigner::new(client.clone());
        Self { client, wbi_signer }
    }

    /// Build default headers for requests.
    fn default_headers(cookie: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate"));
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("zh-CN,zh;q=0.8,en-US;q=0.5,en;q=0.3"));
        headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_STRING));
        headers.insert(ORIGIN, HeaderValue::from_static("https://live.bilibili.com"));
        headers.insert(REFERER, HeaderValue::from_static("https://live.bilibili.com"));

        // Generate fake buvid3
        let buvid3 = generate_fake_buvid3();
        let cookie_value = if let Some(c) = cookie {
            format!("buvid3={};{}", buvid3, c)
        } else {
            format!("buvid3={};", buvid3)
        };
        if let Ok(value) = HeaderValue::from_str(&cookie_value) {
            headers.insert(COOKIE, value);
        }

        headers
    }

    /// Extract room ID from URL.
    fn extract_room_id(url: &str) -> Option<String> {
        let re = Regex::new(r"live\.bilibili\.com/(\d+)").ok()?;
        re.captures(url)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
    }

    /// Get real room ID from short ID.
    async fn get_real_room_id(&self, short_id: &str, headers: &HeaderMap) -> Result<u64> {
        let url = format!(
            "https://api.live.bilibili.com/room/v1/Room/room_init?id={}",
            short_id
        );

        let resp: RoomInitResponse = self
            .client
            .get(&url)
            .headers(headers.clone())
            .timeout(Duration::from_secs(5))
            .send()
            .await?
            .json()
            .await?;

        Ok(resp.data.room_id)
    }

    /// Get danmaku connection info with WBI signing.
    async fn get_danmu_info(&self, room_id: u64, headers: &HeaderMap) -> Result<(String, String)> {
        // Build params
        let mut params = BTreeMap::new();
        params.insert("id".to_string(), room_id.to_string());
        params.insert("type".to_string(), "0".to_string());
        params.insert("web_location".to_string(), "444.8".to_string());

        // Sign with WBI
        if let Err(e) = self.wbi_signer.sign(&mut params, headers).await {
            debug!("WBI signing failed: {}, using default WebSocket URL", e);
            return Ok((DEFAULT_WS_URL.to_string(), String::new()));
        }

        // Build URL with signed params
        let query_string: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!(
            "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo?{}",
            query_string
        );

        debug!("getDanmuInfo URL: {}", url);

        // Make request
        let response = self
            .client
            .get(&url)
            .headers(headers.clone())
            .timeout(Duration::from_secs(5))
            .send()
            .await?
            .text()
            .await?;

        // Parse JSON response
        let json: Value = serde_json::from_str(&response)
            .map_err(|e| DanmakuError::Decode(format!("Invalid JSON: {}", e)))?;

        // Check if API returned an error
        let code = json.get("code").and_then(|v| v.as_i64()).unwrap_or(-1);
        if code != 0 {
            let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("unknown");
            debug!("getDanmuInfo returned error code {}: {}, using default WebSocket URL", code, msg);
            return Ok((DEFAULT_WS_URL.to_string(), String::new()));
        }

        // Parse successful response
        let data = json.get("data").ok_or_else(|| {
            DanmakuError::Decode("Missing data field in response".to_string())
        })?;

        let token = data.get("token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let ws_url = data.get("host_list")
            .and_then(|v| v.as_array())
            .and_then(|list| list.first())
            .and_then(|host| {
                let h = host.get("host")?.as_str()?;
                let p = host.get("wss_port")?.as_u64()?;
                Some(format!("wss://{}:{}/sub", h, p))
            })
            .unwrap_or_else(|| DEFAULT_WS_URL.to_string());

        debug!("Got token and WebSocket URL: {}", ws_url);
        Ok((ws_url, token))
    }

    /// Build authentication packet.
    fn build_auth_packet(uid: u64, room_id: u64, token: &str) -> Vec<u8> {
        let auth_data = AuthData {
            uid,
            roomid: room_id,
            protover: 3,
            platform: "web",
            auth_type: 2,
            key: token.to_string(),
        };

        let json_data = serde_json::to_vec(&auth_data).unwrap();
        build_packet(&json_data, op::AUTH)
    }

    /// Decode a single packet, handling compression.
    fn decode_packet(data: &[u8]) -> Vec<DecodedPacket> {
        let mut packets = Vec::new();
        let mut offset = 0;

        while offset + 16 <= data.len() {
            let packet_len = BigEndian::read_u32(&data[offset..offset + 4]) as usize;
            let _header_len = BigEndian::read_u16(&data[offset + 4..offset + 6]);
            let version = BigEndian::read_u16(&data[offset + 6..offset + 8]);
            let operation = BigEndian::read_u32(&data[offset + 8..offset + 12]);
            let _sequence = BigEndian::read_u32(&data[offset + 12..offset + 16]);

            if offset + packet_len > data.len() {
                break;
            }

            let body = &data[offset + 16..offset + packet_len];

            match version {
                ver::ZLIB => {
                    // Zlib compressed
                    if let Ok(decompressed) = decompress_zlib(body) {
                        packets.extend(Self::decode_packet(&decompressed));
                    }
                }
                ver::BROTLI => {
                    // Brotli compressed
                    if let Ok(decompressed) = decompress_brotli(body) {
                        packets.extend(Self::decode_packet(&decompressed));
                    }
                }
                ver::RAW_JSON | ver::POPULARITY => {
                    // Raw data
                    packets.push(DecodedPacket {
                        operation,
                        body: body.to_vec(),
                    });
                }
                _ => {
                    debug!("Unknown protocol version: {}", version);
                }
            }

            offset += packet_len;
        }

        packets
    }

    /// Parse a notification message (op=5) into danmaku events.
    fn parse_notification(body: &[u8]) -> Option<DanmakuEvent> {
        let json: Value = serde_json::from_slice(body).ok()?;
        let cmd = json.get("cmd")?.as_str()?;

        // Handle DANMU_MSG variants (e.g., "DANMU_MSG:4:0:2:2:2:0")
        let cmd_base = cmd.split(':').next().unwrap_or(cmd);

        match cmd_base {
            "DANMU_MSG" => Self::parse_danmu_msg(&json),
            "SEND_GIFT" => Self::parse_gift(&json),
            "SUPER_CHAT_MESSAGE" => Self::parse_super_chat(&json),
            "GUARD_BUY" => Self::parse_guard_buy(&json),
            "LIVE_INTERACTIVE_GAME" => Self::parse_interactive_danmaku(&json),
            _ => None,
        }
    }

    /// Parse DANMU_MSG.
    fn parse_danmu_msg(json: &Value) -> Option<DanmakuEvent> {
        let info = json.get("info")?.as_array()?;

        // info[1] = content
        let content = info.get(1)?.as_str()?.to_string();

        // info[2][0] = uid, info[2][1] = name
        let user_info = info.get(2)?.as_array()?;
        let uid = user_info.get(0)?.as_u64();
        let name = user_info.get(1)?.as_str().map(|s| s.to_string());

        // info[0][3] = color
        let meta = info.get(0)?.as_array()?;
        let color = meta.get(3)
            .and_then(|v| v.as_u64())
            .map(|c| c as u32)
            .unwrap_or(DEFAULT_COLOR);

        // Check for emoticon
        let content = if let Some(extra_obj) = meta.get(15) {
            if let Some(extra_str) = extra_obj.get("extra").and_then(|v| v.as_str()) {
                if let Ok(extra) = serde_json::from_str::<Value>(extra_str) {
                    if let Some(emoticon) = extra.get("emoticon_unique").and_then(|v| v.as_str()) {
                        if !emoticon.is_empty() {
                            format!("表情【{}】", emoticon)
                        } else {
                            content
                        }
                    } else {
                        content
                    }
                } else {
                    content
                }
            } else {
                content
            }
        } else {
            content
        };

        let mut chat = ChatMessage::new(content).with_color(color);
        if let Some(n) = name {
            chat = chat.with_name(n);
        }
        if let Some(u) = uid {
            chat = chat.with_uid(u);
        }

        Some(DanmakuEvent::Chat(chat))
    }

    /// Parse SEND_GIFT.
    fn parse_gift(json: &Value) -> Option<DanmakuEvent> {
        let data = json.get("data")?;

        let name = data.get("uname")?.as_str()?.to_string();
        let uid = data.get("uid")?.as_u64()?;
        let gift_name = data.get("giftName")?.as_str()?.to_string();
        let price = data.get("price").and_then(|v| v.as_u64()).unwrap_or(0);
        let num = data.get("num").and_then(|v| v.as_u64()).unwrap_or(1) as u32;

        let content = format!("{}投喂了{}个{}", name, num, gift_name);

        Some(DanmakuEvent::Gift(GiftMessage {
            name,
            uid,
            gift_name,
            price,
            num,
            content,
            timestamp: chrono::Utc::now(),
        }))
    }

    /// Parse SUPER_CHAT_MESSAGE.
    fn parse_super_chat(json: &Value) -> Option<DanmakuEvent> {
        let data = json.get("data")?;

        let user_info = data.get("user_info")?;
        let name = user_info.get("uname")?.as_str()?.to_string();
        let uid = data.get("uid")?.as_u64()?;
        let content = data.get("message")?.as_str()?.to_string();
        let price = data.get("price").and_then(|v| v.as_u64()).unwrap_or(0) * 1000;

        Some(DanmakuEvent::SuperChat(SuperChatMessage {
            name,
            uid,
            content,
            price,
            timestamp: chrono::Utc::now(),
        }))
    }

    /// Parse GUARD_BUY.
    fn parse_guard_buy(json: &Value) -> Option<DanmakuEvent> {
        let data = json.get("data")?;

        let name = data.get("username")?.as_str()?.to_string();
        let uid = data.get("uid")?.as_u64()?;
        let gift_name = data.get("gift_name")?.as_str()?.to_string();
        let price = data.get("price").and_then(|v| v.as_u64()).unwrap_or(0);
        let num = data.get("num").and_then(|v| v.as_u64()).unwrap_or(1) as u32;

        Some(DanmakuEvent::GuardBuy(GuardBuyMessage {
            name,
            uid,
            gift_name,
            price,
            num,
            timestamp: chrono::Utc::now(),
        }))
    }

    /// Parse LIVE_INTERACTIVE_GAME (interactive danmaku).
    fn parse_interactive_danmaku(json: &Value) -> Option<DanmakuEvent> {
        let data = json.get("data")?;

        let name = data.get("uname").and_then(|v| v.as_str()).map(|s| s.to_string());
        let content = data.get("msg")?.as_str()?.to_string();

        let mut chat = ChatMessage::new(content).with_color(DEFAULT_COLOR);
        if let Some(n) = name {
            chat = chat.with_name(n);
        }

        Some(DanmakuEvent::Chat(chat))
    }
}

impl Default for Bilibili {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Platform for Bilibili {
    fn name(&self) -> &'static str {
        "Bilibili"
    }

    async fn get_connection_info(
        &self,
        url: &str,
        context: &PlatformContext,
    ) -> Result<ConnectionInfo> {
        let uid = context.uid.unwrap_or(0);
        let cookie = context.cookie.as_deref();
        let headers = Self::default_headers(cookie);

        // Get room ID
        let room_id = if let Some(ref id) = context.room_id {
            id.parse::<u64>().unwrap_or(0)
        } else {
            let short_id = Self::extract_room_id(url)
                .ok_or_else(|| DanmakuError::Decode("Invalid Bilibili URL".to_string()))?;
            self.get_real_room_id(&short_id, &headers).await?
        };

        // Get danmaku info
        let (ws_url, token) = self.get_danmu_info(room_id, &headers).await?;

        // Build auth packet
        let auth_packet = Self::build_auth_packet(uid, room_id, &token);

        Ok(ConnectionInfo::new(ws_url)
            .with_registration(vec![RegistrationData::Binary(auth_packet)])
            .with_headers(headers))
    }

    fn heartbeat_config(&self) -> HeartbeatConfig {
        HeartbeatConfig::binary(HEARTBEAT.to_vec(), Duration::from_secs(30))
    }

    fn decode_message(&self, msg: &[u8]) -> Result<DecodeResult> {
        let packets = Self::decode_packet(msg);
        let mut events = Vec::new();

        for packet in packets {
            match packet.operation {
                op::NOTIFICATION => {
                    if let Some(event) = Self::parse_notification(&packet.body) {
                        events.push(event);
                    }
                }
                op::HEARTBEAT_REPLY => {
                    // Heartbeat reply contains popularity count (4 bytes big-endian)
                    // We ignore it for now
                }
                op::AUTH_REPLY => {
                    debug!("Auth reply received");
                }
                _ => {
                    debug!("Unknown operation: {}", packet.operation);
                }
            }
        }

        Ok(DecodeResult::with_events(events))
    }
}

/// A decoded packet.
struct DecodedPacket {
    operation: u32,
    body: Vec<u8>,
}

/// Build a packet with the given body and operation code.
fn build_packet(body: &[u8], operation: u32) -> Vec<u8> {
    let packet_len = 16 + body.len();
    let mut packet = Vec::with_capacity(packet_len);

    // Header
    packet.extend_from_slice(&(packet_len as u32).to_be_bytes()); // packet length
    packet.extend_from_slice(&16u16.to_be_bytes());               // header length
    packet.extend_from_slice(&1u16.to_be_bytes());                // version
    packet.extend_from_slice(&operation.to_be_bytes());           // operation
    packet.extend_from_slice(&1u32.to_be_bytes());                // sequence

    // Body
    packet.extend_from_slice(body);

    packet
}

/// Decompress zlib data.
fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)
        .map_err(|e| DanmakuError::Compression(format!("zlib: {}", e)))?;
    Ok(decompressed)
}

/// Decompress brotli data.
fn decompress_brotli(data: &[u8]) -> Result<Vec<u8>> {
    let mut decompressed = Vec::new();
    brotli::BrotliDecompress(&mut std::io::Cursor::new(data), &mut decompressed)
        .map_err(|e| DanmakuError::Compression(format!("brotli: {}", e)))?;
    Ok(decompressed)
}

/// Generate a fake buvid3.
fn generate_fake_buvid3() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    // Generate UUID-like string
    let uuid = format!("{:032X}", timestamp);
    format!(
        "{}-{}-{}-{}-{}infoc",
        &uuid[0..8],
        &uuid[8..12],
        &uuid[12..16],
        &uuid[16..20],
        &uuid[20..32]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_room_id() {
        assert_eq!(
            Bilibili::extract_room_id("https://live.bilibili.com/12345"),
            Some("12345".to_string())
        );
        assert_eq!(
            Bilibili::extract_room_id("https://live.bilibili.com/123?from=abc"),
            Some("123".to_string())
        );
    }

    #[test]
    fn test_build_packet() {
        let body = b"test";
        let packet = build_packet(body, op::AUTH);

        assert_eq!(BigEndian::read_u32(&packet[0..4]), 20); // 16 + 4
        assert_eq!(BigEndian::read_u16(&packet[4..6]), 16);
        assert_eq!(BigEndian::read_u16(&packet[6..8]), 1);
        assert_eq!(BigEndian::read_u32(&packet[8..12]), op::AUTH);
        assert_eq!(BigEndian::read_u32(&packet[12..16]), 1);
        assert_eq!(&packet[16..], body);
    }

    #[test]
    fn test_decode_raw_packet() {
        // Build a simple raw packet
        let body = br#"{"cmd":"TEST"}"#;
        let packet = build_packet(body, op::NOTIFICATION);

        let decoded = Bilibili::decode_packet(&packet);
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].operation, op::NOTIFICATION);
        assert_eq!(decoded[0].body, body);
    }

    #[test]
    fn test_parse_danmu_msg() {
        let json = serde_json::json!({
            "cmd": "DANMU_MSG",
            "info": [
                [0, 1, 25, 16777215, 0, 0, 0, "", 0, 0, 0, "", 0, "{}", "{}", {"extra": "{}"}],
                "Hello World",
                [12345, "TestUser", 0, 0, 0, 0, 0, ""]
            ]
        });

        let event = Bilibili::parse_danmu_msg(&json);
        assert!(event.is_some());

        if let Some(DanmakuEvent::Chat(chat)) = event {
            assert_eq!(chat.content, "Hello World");
            assert_eq!(chat.name, Some("TestUser".to_string()));
            assert_eq!(chat.uid, Some(12345));
            assert_eq!(chat.color, DEFAULT_COLOR);
        } else {
            panic!("Expected Chat event");
        }
    }

    #[test]
    fn test_generate_fake_buvid3() {
        let buvid = generate_fake_buvid3();
        assert!(buvid.ends_with("infoc"));
        assert!(buvid.contains("-"));
    }
}
