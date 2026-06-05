//! Douyin (TikTok China) live danmaku protocol implementation.
//!
//! Douyin uses Protocol Buffers over WebSocket:
//! - PushFrame wrapper with gzip-compressed payload
//! - Response containing message list
//! - ChatMessage for danmaku content

use std::io::Read;
use std::time::Duration;

use async_trait::async_trait;
use flate2::read::GzDecoder;
use regex::Regex;
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, ORIGIN, REFERER, USER_AGENT};
use tracing::debug;

use crate::codec::protobuf::{ProtoReader, ProtoWriter};
use crate::error::{DanmakuError, Result};
use crate::message::{ChatMessage, DEFAULT_COLOR, DanmakuEvent};
use crate::protocols::{ConnectionInfo, DecodeResult, HeartbeatConfig, Platform, PlatformContext};

/// Heartbeat packet.
const HEARTBEAT: &[u8] = b":\x02hb";
const USER_AGENT_STRING: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const DOUYIN_WS_HOSTS: &[&str] = &[
    "wss://webcast100-ws-web-lq.douyin.com/webcast/im/push/v2/",
    "wss://webcast100-ws-web-hl.douyin.com/webcast/im/push/v2/",
    "wss://webcast100-ws-web-lf.douyin.com/webcast/im/push/v2/",
];
const DEFAULT_TTWID: &str = "1%7Cu7ogdHsSmHtxbt4hjDCNvcLfVJz78CTM0TTWU8Hio8w%7C1751545220%7C18aac967e501e9d6c13384335ced3523c46a0b1cc4535c7213bc2506a7f462c8";
const XBOGUS_ALPHABET: &[u8; 64] =
    b"Dkdpgh4ZKsQB80/Mfvw36XI1R25+WUAlEi7NLboqYTOPuzmFjJnryx9HVGcaStCe";
const STANDARD_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const EMPTY_MD5_BYTES: [u8; 2] = [0x45, 0x3f];

const fn build_lookup() -> [u8; 128] {
    let mut table = [0u8; 128];
    let mut i = 0;
    while i < 64 {
        table[STANDARD_ALPHABET[i] as usize] = XBOGUS_ALPHABET[i];
        i += 1;
    }
    table
}

const ALPHABET_LOOKUP: [u8; 128] = build_lookup();

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
    fn get_x_ms_stub(params: &[(&str, &str)]) -> [u8; 32] {
        let sig_params: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(",");

        hex_md5(sig_params.as_bytes())
    }

    /// Build WebSocket URL with parameters.
    fn build_ws_url(room_id: &str, host: &str) -> String {
        let user_unique_id = Self::generate_user_unique_id();
        let version_code = "180800";
        let webcast_sdk_version = "1.0.15";
        let update_version_code = "1.0.15";

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
        let signature = generate_xbogus(&x_ms_stub, 1);
        let signature = String::from_utf8_lossy(&signature);

        let ws_params = [
            ("app_name", "douyin_web".to_string()),
            ("compress", "gzip".to_string()),
            ("device_platform", "web".to_string()),
            ("browser_language", "zh-CN".to_string()),
            ("browser_platform", "Win32".to_string()),
            ("browser_name", "Mozilla".to_string()),
            ("browser_version", "120.0.0.0".to_string()),
            ("aid", "6383".to_string()),
            ("live_id", "1".to_string()),
            ("enter_from", "web_live".to_string()),
            ("version_code", version_code.to_string()),
            ("webcast_sdk_version", webcast_sdk_version.to_string()),
            ("update_version_code", update_version_code.to_string()),
            ("host", "https://live.douyin.com".to_string()),
            ("did_rule", "3".to_string()),
            ("identity", "audience".to_string()),
            ("endpoint", "live_pc".to_string()),
            ("need_persist_msg_count", "15".to_string()),
            ("heartbeatDuration", "0".to_string()),
            ("room_id", room_id.to_string()),
            ("user_unique_id", user_unique_id),
            ("signature", signature.to_string()),
        ];

        let query: String = ws_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        format!("{}?{}", host, query)
    }

    fn build_connection_info(room_id: &str) -> ConnectionInfo {
        let urls = DOUYIN_WS_HOSTS
            .iter()
            .map(|host| Self::build_ws_url(room_id, host))
            .collect::<Vec<_>>();
        let mut iter = urls.into_iter();
        ConnectionInfo::new(iter.next().unwrap()).with_fallback_ws_urls(iter.collect())
    }

    fn default_headers(context: &PlatformContext) -> HeaderMap {
        let mut headers = HeaderMap::new();

        let user_agent = context
            .extra
            .get("user-agent")
            .map(String::as_str)
            .unwrap_or(USER_AGENT_STRING);
        if let Ok(value) = HeaderValue::from_str(user_agent) {
            headers.insert(USER_AGENT, value);
        } else {
            headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_STRING));
        }

        headers.insert(ORIGIN, HeaderValue::from_static("https://live.douyin.com"));

        let referer = context
            .extra
            .get("referer")
            .map(String::as_str)
            .unwrap_or("https://live.douyin.com/");
        if let Ok(value) = HeaderValue::from_str(referer) {
            headers.insert(REFERER, value);
        } else {
            headers.insert(
                REFERER,
                HeaderValue::from_static("https://live.douyin.com/"),
            );
        }

        let cookie = context
            .cookie
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(|value| {
                if value.contains("ttwid") {
                    value.to_string()
                } else if value.ends_with(';') {
                    format!("{}ttwid={};", value, DEFAULT_TTWID)
                } else {
                    format!("{};ttwid={};", value, DEFAULT_TTWID)
                }
            })
            .unwrap_or_else(|| format!("ttwid={};", DEFAULT_TTWID));
        if let Ok(value) = HeaderValue::from_str(&cookie) {
            headers.insert(COOKIE, value);
        }

        headers
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
        writer.write_string(7, "ack");
        writer.write_bytes(8, internal_ext.as_bytes());
        writer.into_buffer()
    }
}

fn hex_md5(input: &[u8]) -> [u8; 32] {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = md5::compute(input);
    let mut output = [0u8; 32];
    for (i, byte) in digest.0.iter().enumerate() {
        output[i * 2] = HEX[(byte >> 4) as usize];
        output[i * 2 + 1] = HEX[(byte & 0x0f) as usize];
    }
    output
}

fn hex_byte(h: u8, l: u8) -> u8 {
    let hi = if h >= b'a' { h - b'a' + 10 } else { h - b'0' };
    let lo = if l >= b'a' { l - b'a' + 10 } else { l - b'0' };
    (hi << 4) | lo
}

fn md5_last2(hex_str: &[u8; 32]) -> [u8; 2] {
    let mut bytes = [0u8; 16];
    for i in 0..16 {
        bytes[i] = hex_byte(hex_str[i * 2], hex_str[i * 2 + 1]);
    }
    let hash = md5::compute(bytes);
    [hash.0[14], hash.0[15]]
}

fn rc4_encrypt(key: u8, data: &mut [u8]) {
    let mut s: [u8; 256] = core::array::from_fn(|i| i as u8);
    let mut j: usize = 0;

    for i in 0..256 {
        j = (j + s[i] as usize + key as usize) % 256;
        s.swap(i, j);
    }

    let mut i: usize = 0;
    j = 0;
    for byte in data.iter_mut() {
        i = (i + 1) % 256;
        j = (j + s[i] as usize) % 256;
        s.swap(i, j);
        *byte ^= s[(s[i] as usize + s[j] as usize) % 256];
    }
}

fn encode_base64(data: &[u8; 12], out: &mut [u8; 16]) {
    const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut input = 0;
    let mut output = 0;
    while input < 12 {
        let b0 = data[input] as usize;
        let b1 = data[input + 1] as usize;
        let b2 = data[input + 2] as usize;

        out[output] = ALPHABET_LOOKUP[B64[(b0 >> 2) & 0x3f] as usize];
        out[output + 1] = ALPHABET_LOOKUP[B64[((b0 << 4) | (b1 >> 4)) & 0x3f] as usize];
        out[output + 2] = ALPHABET_LOOKUP[B64[((b1 << 2) | (b2 >> 6)) & 0x3f] as usize];
        out[output + 3] = ALPHABET_LOOKUP[B64[b2 & 0x3f] as usize];

        input += 3;
        output += 4;
    }
}

fn generate_xbogus(ms_stub: &[u8; 32], counter: u8) -> [u8; 16] {
    let random1 = rand::random::<u8>();
    let random2 = (rand::random::<u8>() as u16 * 255 / 256) as u8;
    let header = 0x40 | (random1 & 0x1f);
    let md5_bytes = md5_last2(ms_stub);
    let mut payload: [u8; 10] = [
        counter & 0x3f,
        0,
        1,
        0x0e,
        EMPTY_MD5_BYTES[0],
        EMPTY_MD5_BYTES[1],
        md5_bytes[0],
        md5_bytes[1],
        random2,
        0,
    ];
    payload[9] = payload[..9].iter().fold(0, |acc, &item| acc ^ item);
    rc4_encrypt(random2, &mut payload);

    let mut final_data = [0u8; 12];
    final_data[0] = header;
    final_data[1] = random2;
    final_data[2..].copy_from_slice(&payload);

    let mut result = [0u8; 16];
    encode_base64(&final_data, &mut result);
    result
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

        if let Some(ws_url) = context.extra.get("ws_url") {
            debug!("Douyin WebSocket URL: {}", ws_url);
            return Ok(
                ConnectionInfo::new(ws_url.clone()).with_headers(Self::default_headers(context))
            );
        }

        let ws_info = Self::build_connection_info(&room_id);
        debug!("Douyin WebSocket URL: {}", ws_info.ws_url);

        Ok(ws_info.with_headers(Self::default_headers(context)))
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

    #[test]
    fn test_build_ack_matches_python_behavior() {
        let ack = Douyin::build_ack(12345, "internal_src:dim|seq:1");
        let mut reader = ProtoReader::new(&ack);
        let fields = reader.parse_all();

        assert_eq!(fields[&2][0].as_u64(), Some(12345));
        assert_eq!(fields[&7][0].as_str(), Some("ack"));
        assert_eq!(fields[&8][0].as_str(), Some("internal_src:dim|seq:1"));
    }

    #[test]
    fn test_default_headers_use_context_values() {
        let context = PlatformContext::new().with_cookie("ttwid=test;");
        let mut context = context;
        context
            .extra
            .insert("user-agent".to_string(), "TestAgent/1.0".to_string());
        context.extra.insert(
            "referer".to_string(),
            "https://live.douyin.com/123".to_string(),
        );

        let headers = Douyin::default_headers(&context);
        assert_eq!(headers[USER_AGENT], "TestAgent/1.0");
        assert_eq!(headers[REFERER], "https://live.douyin.com/123");
        assert_eq!(headers[COOKIE], "ttwid=test;");
        assert_eq!(headers[ORIGIN], "https://live.douyin.com");
    }
}
