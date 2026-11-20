//! Bilibili live danmaku protocol implementation.
//!
//! Bilibili uses a binary protocol over WebSocket with the following features:
//!
//! - 16-byte header with packet length, version, operation code
//! - Version 2: zlib compressed payload
//! - Version 3: brotli compressed payload
//! - JSON messages for chat, gifts, super chat, etc.

use std::collections::{BTreeMap, HashSet};
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use byteorder::{BigEndian, ByteOrder};
use flate2::read::ZlibDecoder;
use regex::Regex;
use reqwest::header::{
    ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, COOKIE, HeaderMap, HeaderValue, ORIGIN, REFERER,
    USER_AGENT,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{debug, info};

use crate::codec::protobuf::{ProtoReader, ProtoValue};
use crate::codec::{MAX_DECOMPRESSED_SIZE, decompress_limited};
use crate::error::{DanmakuError, Result};
use crate::message::{
    ChatMessage, DEFAULT_COLOR, DanmakuEvent, GiftMessage, GuardBuyMessage, SuperChatMessage,
};
use crate::protocols::wbi::WbiSigner;
use crate::protocols::{
    ConnectionInfo, DecodeResult, HeartbeatConfig, Platform, PlatformContext,
    RecordingDecodeResult, RegistrationData,
};

/// User agent for Bilibili requests.
const USER_AGENT_STRING: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

/// Default WebSocket URL.
const DEFAULT_WS_URL: &str = "wss://broadcastlv.chat.bilibili.com/sub";

/// Maximum nesting depth for compressed packets.
/// The real protocol only nests one level (compressed packet contains raw packets);
/// deeper nesting indicates malformed or hostile input.
const MAX_DECODE_DEPTH: u8 = 8;

/// Heartbeat packet.
/// Header: len=31, header_len=16, ver=1, op=2, seq=1
/// Body: "[object Object] "
const HEARTBEAT: &[u8] = &[
    0x00, 0x00, 0x00, 0x1f, // packet length = 31
    0x00, 0x10, // header length = 16
    0x00, 0x01, // version = 1
    0x00, 0x00, 0x00, 0x02, // operation = 2 (heartbeat)
    0x00, 0x00, 0x00, 0x01, // sequence = 1
    // "[object Object] "
    0x5b, 0x6f, 0x62, 0x6a, 0x65, 0x63, 0x74, 0x20, 0x4f, 0x62, 0x6a, 0x65, 0x63, 0x74, 0x5d, 0x20,
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

/// One application-level packet. The XML recorder keeps its representations together.
pub(crate) struct Notification {
    pub(crate) event: Option<DanmakuEvent>,
    pub(crate) detail: Option<(&'static str, Value)>,
    pub(crate) raw: Option<String>,
    pub(crate) chat_metadata: Option<ChatMetadata>,
}

pub(crate) struct ChatMetadata {
    pub(crate) mode: u64,
    pub(crate) size: u64,
    pub(crate) timestamp_ms: i64,
}

#[derive(Serialize)]
struct GiftItem {
    gift_id: u64,
    name: String,
    count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    price: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_coin: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tid: Option<String>,
}

/// Bilibili live danmaku protocol.
pub struct Bilibili {
    client: reqwest::Client,
    wbi_signer: WbiSigner,
    super_chat_ids: Mutex<HashSet<u64>>,
}

impl Bilibili {
    /// Create a new Bilibili protocol handler.
    pub fn new() -> Self {
        let client = reqwest::Client::new();
        let wbi_signer = WbiSigner::new(client.clone());
        Self {
            client,
            wbi_signer,
            super_chat_ids: Mutex::default(),
        }
    }

    fn auth_uid(context: &PlatformContext) -> Result<u64> {
        let mut cookie_uid = None;
        let mut has_session = false;
        for part in context.cookie.as_deref().unwrap_or("").split(';') {
            let Some((name, value)) = part.trim().split_once('=') else {
                continue;
            };
            if name.trim() == "SESSDATA" && !value.trim().is_empty() {
                has_session = true;
            }
            if name.trim() == "DedeUserID" {
                let uid = value
                    .trim()
                    .parse::<u64>()
                    .ok()
                    .filter(|uid| *uid > 0)
                    .ok_or_else(|| {
                        DanmakuError::Decode("Invalid Bilibili cookie UID (value redacted)".into())
                    })?;
                if cookie_uid.is_some_and(|previous| previous != uid) {
                    return Err(DanmakuError::Decode(
                        "Conflicting Bilibili cookie UIDs (values redacted)".into(),
                    ));
                }
                cookie_uid = Some(uid);
            }
        }
        if let (Some(explicit), Some(cookie)) = (context.uid, cookie_uid)
            && explicit != cookie
        {
            return Err(DanmakuError::Decode(
                "Bilibili context UID does not match cookie UID (values redacted)".into(),
            ));
        }
        // A logged-in API token cannot authenticate an anonymous uid=0 registration.
        let uid = context.uid.or(cookie_uid).unwrap_or(0);
        if has_session && cookie_uid.is_none() {
            return Err(DanmakuError::Decode(
                "Bilibili session cookie requires its account UID (credentials redacted)".into(),
            ));
        }
        Ok(uid)
    }

    /// Build default headers for requests.
    fn default_headers(cookie: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate"));
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("zh-CN,zh;q=0.8,en-US;q=0.5,en;q=0.3"),
        );
        headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_STRING));
        headers.insert(
            ORIGIN,
            HeaderValue::from_static("https://live.bilibili.com"),
        );
        headers.insert(
            REFERER,
            HeaderValue::from_static("https://live.bilibili.com"),
        );

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
            let msg = json
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            debug!(
                "getDanmuInfo returned error code {}: {}, using default WebSocket URL",
                code, msg
            );
            return Ok((DEFAULT_WS_URL.to_string(), String::new()));
        }

        // Parse successful response
        let data = json
            .get("data")
            .ok_or_else(|| DanmakuError::Decode("Missing data field in response".to_string()))?;

        let token = data
            .get("token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let ws_url = data
            .get("host_list")
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
        let mut decompression_budget = MAX_DECOMPRESSED_SIZE;
        Self::decode_packet_at_depth(data, 0, &mut decompression_budget)
    }

    fn decode_packet_at_depth(
        data: &[u8],
        depth: u8,
        decompression_budget: &mut usize,
    ) -> Vec<DecodedPacket> {
        let mut packets = Vec::new();
        let mut offset = 0;

        while offset + 16 <= data.len() {
            let packet_len = BigEndian::read_u32(&data[offset..offset + 4]) as usize;
            let header_len = BigEndian::read_u16(&data[offset + 4..offset + 6]) as usize;
            let version = BigEndian::read_u16(&data[offset + 6..offset + 8]);
            let operation = BigEndian::read_u32(&data[offset + 8..offset + 12]);
            let _sequence = BigEndian::read_u32(&data[offset + 12..offset + 16]);

            // 声明长度必须至少覆盖 16 字节包头，否则该帧不可信：
            // packet_len < 16 时反向切片会 panic，packet_len == 0 还会死循环
            if header_len < 16 || packet_len < header_len {
                debug!(
                    "Malformed packet length {}, dropping remaining {} bytes",
                    packet_len,
                    data.len() - offset
                );
                break;
            }

            if packet_len > data.len() - offset {
                break;
            }

            let body = &data[offset + header_len..offset + packet_len];

            match version {
                ver::ZLIB | ver::BROTLI if depth >= MAX_DECODE_DEPTH => {
                    debug!(
                        "Packet nesting exceeds depth {}, dropping",
                        MAX_DECODE_DEPTH
                    );
                }
                ver::ZLIB => {
                    // Zlib compressed
                    if let Ok(decompressed) = decompress_zlib(body, *decompression_budget) {
                        *decompression_budget -= decompressed.len();
                        packets.extend(Self::decode_packet_at_depth(
                            &decompressed,
                            depth + 1,
                            decompression_budget,
                        ));
                    }
                }
                ver::BROTLI => {
                    // Brotli compressed
                    if let Ok(decompressed) = decompress_brotli(body, *decompression_budget) {
                        *decompression_budget -= decompressed.len();
                        packets.extend(Self::decode_packet_at_depth(
                            &decompressed,
                            depth + 1,
                            decompression_budget,
                        ));
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

    /// Parse a notification (op=5) without losing the original JSON.
    fn parse_notification(body: &[u8], save_detail: bool, save_raw: bool) -> Option<Notification> {
        let text = std::str::from_utf8(body).ok()?;
        let json: Value = serde_json::from_str(text).ok()?;
        let cmd = json.get("cmd")?.as_str()?;
        let cmd_base = cmd.split(':').next().unwrap_or(cmd);

        let event = match cmd_base {
            "DANMU_MSG" => Self::parse_danmu_msg(&json),
            "SEND_GIFT" => Self::parse_gift(&json),
            "SUPER_CHAT_MESSAGE" | "SUPER_CHAT_MESSAGE_JP" => Self::parse_super_chat(&json),
            "GUARD_BUY" => Self::parse_guard_buy(&json),
            "LIVE_INTERACTIVE_GAME" => Self::parse_interactive_danmaku(&json),
            _ => None,
        };
        let detail = if save_detail {
            Self::notification_detail(cmd_base, &json, event.as_ref())
        } else {
            None
        };
        let chat_metadata = match &event {
            Some(DanmakuEvent::Chat(chat)) => {
                let meta = &json["info"][0];
                Some(ChatMetadata {
                    mode: meta[1]
                        .as_u64()
                        .filter(|n| (1..=9).contains(n))
                        .unwrap_or(1),
                    size: meta[2]
                        .as_u64()
                        .filter(|n| (1..=1000).contains(n))
                        .unwrap_or(25),
                    timestamp_ms: meta[4]
                        .as_i64()
                        .filter(|n| (1_000_000_000_000..10_000_000_000_000).contains(n))
                        .unwrap_or_else(|| chat.timestamp.timestamp_millis()),
                })
            }
            _ => None,
        };
        let raw = save_raw.then(|| text.to_owned());
        (event.is_some() || detail.is_some() || raw.is_some()).then_some(Notification {
            event,
            detail,
            raw,
            chat_metadata,
        })
    }

    fn notification_detail(
        cmd: &str,
        message: &Value,
        event: Option<&DanmakuEvent>,
    ) -> Option<(&'static str, Value)> {
        let data = &message["data"];
        match event {
            Some(DanmakuEvent::Chat(chat)) => {
                let user = &message["info"][2];
                Some((
                    "danmaku",
                    json!({
                        "uid": chat.uid, "user": chat.name,
                        "is_admin": user.get(2).and_then(Value::as_u64).map(|n| n == 1),
                        "is_vip": user.get(3).and_then(Value::as_u64).map(|n| n == 1),
                        "guard_level": message["info"].get(7).and_then(Value::as_u64),
                    }),
                ))
            }
            Some(DanmakuEvent::Gift(gift)) => Some((
                "gift",
                json!({
                    "uid": gift.uid, "user": gift.name, "giftname": gift.gift_name,
                    "count": gift.num, "tid": data["tid"],
                }),
            )),
            Some(DanmakuEvent::GuardBuy(guard)) => Some((
                "guard_buy",
                json!({
                    "uid": guard.uid, "user": guard.name, "giftname": guard.gift_name,
                    "level": data["guard_level"], "count": guard.num,
                }),
            )),
            Some(DanmakuEvent::SuperChat(chat)) => Some((
                "super_chat",
                json!({
                    "uid": chat.uid, "user": chat.name, "id": data["id"],
                    "price": data["price"], "time": data["time"],
                }),
            )),
            _ => match cmd {
                "SEND_GIFT_V2" => Some(("gift", Self::parse_gift_v2(data)?)),
                "INTERACT_WORD" => {
                    let action = match data["msg_type"].as_u64()? {
                        2 => "follow",
                        3 => "share",
                        4 => "special_follow",
                        5 => "mutual_follow",
                        _ => return None,
                    };
                    Some((
                        "interact",
                        json!({"action": action, "uid": data["uid"], "user": data["uname"]}),
                    ))
                }
                "LIVE" | "PREPARING" | "WARNING" | "CUT_OFF" => Some((
                    "room_status",
                    json!({
                        "cmd": cmd,
                        "room_id": message.get("roomid").unwrap_or(&message["room_id"]),
                        "message": message.get("msg").unwrap_or(&data["msg"]),
                    }),
                )),
                _ => None,
            },
        }
    }

    fn parse_gift_v2(data: &Value) -> Option<Value> {
        let bytes = STANDARD.decode(data["pb"].as_str()?).ok()?;
        let fields = ProtoReader::new(&bytes).parse_all_strict()?;
        let first = |tag| fields.get(&tag)?.first();
        // SendGiftBroadcast: switch=11, gift_list=10. Each GiftItem has its own tid.
        if first(11)?.as_u64()? == 0 {
            return None;
        }
        let uid = first(1)?.as_u64()?;
        let name = first(2)?.as_str()?;
        let items: Vec<_> = fields
            .get(&10)?
            .iter()
            .filter_map(|item| {
                let bytes = match item {
                    ProtoValue::Bytes(bytes) => bytes.as_slice(),
                    ProtoValue::String(text) => text.as_bytes(),
                    _ => return None,
                };
                let fields = ProtoReader::new(bytes).parse_all_strict()?;
                let first = |tag| fields.get(&tag)?.first();
                Some(GiftItem {
                    gift_id: first(1)?.as_u64()?,
                    name: first(2)?.as_str()?.to_owned(),
                    count: u32::try_from(first(3)?.as_u64()?).ok()?,
                    price: first(5).and_then(ProtoValue::as_u64),
                    total_coin: first(7).and_then(ProtoValue::as_u64),
                    tid: first(9).and_then(ProtoValue::as_str).map(str::to_owned),
                })
            })
            .collect();
        (!items.is_empty()).then(|| json!({"uid": uid, "user": name, "items": items}))
    }

    /// Parse DANMU_MSG.
    fn parse_danmu_msg(json: &Value) -> Option<DanmakuEvent> {
        let info = json.get("info")?.as_array()?;

        // info[1] = content
        let content = info.get(1)?.as_str()?.to_string();

        // info[2][0] = uid, info[2][1] = name
        let user_info = info.get(2)?.as_array()?;
        let uid = user_info.first()?.as_u64();
        let name = user_info.get(1)?.as_str().map(|s| s.to_string());

        // info[0][3] = color
        let meta = info.first()?.as_array()?;
        let color = meta
            .get(3)
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
        // Preserve the legacy XML price attribute. The detail field uses the source value.
        let price = data
            .get("price")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .saturating_mul(1000);

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
        let gift_name = data["gift_name"]
            .as_str()
            .filter(|name| !name.is_empty())
            .or(match data["guard_level"].as_u64() {
                Some(1) => Some("总督"),
                Some(2) => Some("提督"),
                Some(3) => Some("舰长"),
                _ => None,
            })?
            .to_owned();
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

        let name = data
            .get("uname")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
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
        let uid = Self::auth_uid(context)?;
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
        let events = Self::decode_notifications(msg, false, false)?
            .into_iter()
            .filter_map(|notification| notification.event)
            .collect();
        Ok(DecodeResult::with_events(events))
    }

    fn decode_recording_message(
        &self,
        msg: &[u8],
        save_detail: bool,
        save_raw: bool,
    ) -> Result<RecordingDecodeResult> {
        let mut notifications = Self::decode_notifications(msg, save_detail, save_raw)?;
        for notification in &mut notifications {
            if let Some(("super_chat", detail)) = &notification.detail
                && let Some(id) = detail["id"].as_u64().filter(|&id| id != 0)
                && !self.super_chat_ids.lock().unwrap().insert(id)
            {
                // JP and ordinary notifications can refer to the same SC.
                notification.event = None;
                notification.detail = None;
            }
        }
        Ok(RecordingDecodeResult {
            decoded: DecodeResult::empty(),
            notifications,
        })
    }
}

impl Bilibili {
    fn decode_notifications(
        msg: &[u8],
        save_detail: bool,
        save_raw: bool,
    ) -> Result<Vec<Notification>> {
        let mut notifications = Vec::new();
        for packet in Self::decode_packet(msg) {
            match packet.operation {
                op::NOTIFICATION => {
                    if let Some(notification) =
                        Self::parse_notification(&packet.body, save_detail, save_raw)
                    {
                        notifications.push(notification);
                    }
                }
                op::HEARTBEAT_REPLY => {
                    // Heartbeat reply contains popularity count (4 bytes big-endian)
                    // We ignore it for now
                }
                op::AUTH_REPLY => {
                    let reply: Value = serde_json::from_slice(&packet.body).map_err(|_| {
                        DanmakuError::Decode(
                            "Invalid Bilibili auth response (body redacted)".into(),
                        )
                    })?;
                    let code = reply.get("code").and_then(Value::as_i64).ok_or_else(|| {
                        DanmakuError::Decode("Bilibili auth response missing numeric code".into())
                    })?;
                    if code != 0 {
                        return Err(DanmakuError::Decode(format!(
                            "Bilibili authentication rejected: code={code}"
                        )));
                    }
                    info!("Bilibili danmaku authentication succeeded (code=0)");
                }
                _ => {
                    debug!("Unknown operation: {}", packet.operation);
                }
            }
        }

        Ok(notifications)
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
    packet.extend_from_slice(&16u16.to_be_bytes()); // header length
    packet.extend_from_slice(&1u16.to_be_bytes()); // version
    packet.extend_from_slice(&operation.to_be_bytes()); // operation
    packet.extend_from_slice(&1u32.to_be_bytes()); // sequence

    // Body
    packet.extend_from_slice(body);

    packet
}

/// Decompress zlib data.
fn decompress_zlib(data: &[u8], limit: usize) -> Result<Vec<u8>> {
    decompress_limited(ZlibDecoder::new(data), "zlib", limit)
}

/// Decompress brotli data.
fn decompress_brotli(data: &[u8], limit: usize) -> Result<Vec<u8>> {
    decompress_limited(brotli::Decompressor::new(data, 4096), "brotli", limit)
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
    fn recording_writes_one_xml_row_per_bilibili_notification() {
        use crate::codec::protobuf::ProtoWriter;
        use crate::output::xml::{XmlWriter, XmlWriterConfig};
        use quick_xml::{Reader, events::Event};

        let mut batch = ProtoWriter::new();
        batch.write_varint_field(1, 123);
        batch.write_string(2, "sender");
        for (gift_id, name, count, tid) in [(1, "first", 1, "tid-1"), (2, "second", 3, "tid-2")] {
            let mut item = ProtoWriter::new();
            item.write_varint_field(1, gift_id);
            item.write_string(2, name);
            item.write_varint_field(3, count);
            if gift_id == 2 {
                item.write_varint_field(5, 1000);
                item.write_varint_field(7, 3000);
            }
            item.write_string(9, tid);
            batch.write_bytes(10, &item.into_buffer());
        }
        batch.write_varint_field(11, 1);
        let v2 =
            json!({"cmd": "SEND_GIFT_V2", "data": {"pb": STANDARD.encode(batch.into_buffer())}})
                .to_string();
        let chat = "{\r\n\"cmd\":\"DANMU_MSG:1\",\"info\":[[0,4,18,16777215,1795151935316],\"A & B\\nsecond line\",[123,\"sender\",1,0]]}";
        let sc = r#"{"cmd":"SUPER_CHAT_MESSAGE_JP","data":{"id":42,"user_info":{"uname":"sender"},"uid":123,"message":"hello","price":30,"time":60}}"#;
        let guard =
            r#"{"cmd":"GUARD_BUY","data":{"username":"sender","uid":123,"guard_level":3,"num":1}}"#;
        let follow = r#"{"cmd":"INTERACT_WORD","data":{"msg_type":2,"uid":123,"uname":"sender"}}"#;
        let cutoff = r#"{"cmd":"CUT_OFF","msg":"offline","roomid":456}"#;
        let combo = r#"{"cmd":"COMBO_SEND","data":{"total_num":18}}"#;
        let bad_v2 = r#"{"cmd":"SEND_GIFT_V2","data":{"pb":"not base64"}}"#;
        let duplicate_sc = sc.replace("SUPER_CHAT_MESSAGE_JP", "SUPER_CHAT_MESSAGE");
        let messages = [
            chat,
            &v2,
            sc,
            guard,
            follow,
            cutoff,
            combo,
            bad_v2,
            &duplicate_sc,
        ];
        let mut frame: Vec<u8> = messages
            .iter()
            .flat_map(|body| build_packet(body.as_bytes(), op::NOTIFICATION))
            .collect();
        frame.extend(build_packet(br#"{"code":0}"#, op::AUTH_REPLY));

        let dir = std::env::temp_dir().join(format!(
            "bilibili-notifications-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        for (detail, raw, expected) in [
            (false, false, "d"),
            (false, true, "doooooooo"),
            (true, false, "dsssss"),
            (true, true, "dsssssooo"),
        ] {
            let mut writer = XmlWriter::new(
                dir.join(format!("{detail}-{raw}.xml")),
                XmlWriterConfig {
                    save_detail: detail,
                    save_raw: raw,
                    ..Default::default()
                },
            )
            .unwrap();
            let decoded = Bilibili::new()
                .decode_recording_message(&frame, detail, raw)
                .unwrap();
            for notification in &decoded.notifications {
                writer.write_bilibili_notification(notification).unwrap();
            }
            assert_eq!(writer.message_count() as usize, expected.len());
            let xml = std::fs::read_to_string(writer.finish().unwrap()).unwrap();
            assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?><i "));
            assert_eq!(xml.lines().count(), expected.len() + 2);

            let mut reader = Reader::from_str(&xml);
            let rows: Vec<_> = std::iter::from_fn(|| match reader.read_event().unwrap() {
                Event::Eof => None,
                event => Some(event),
            })
            .filter_map(|event| match event {
                Event::Start(elem) | Event::Empty(elem) if elem.name().as_ref() != b"i" => {
                    let tag = elem.name().as_ref()[0] as char;
                    let attrs: BTreeMap<_, _> = elem
                        .attributes()
                        .map(|attr| {
                            let attr = attr.unwrap();
                            (
                                std::str::from_utf8(attr.key.as_ref()).unwrap().to_owned(),
                                attr.unescape_value().unwrap().into_owned(),
                            )
                        })
                        .collect();
                    Some((tag, attrs))
                }
                _ => None,
            })
            .collect();
            assert_eq!(rows.iter().map(|row| row.0).collect::<String>(), expected);
            let (_, fields) = rows[0].1["p"].split_once(',').unwrap();
            assert_eq!(fields, "4,18,16777215,1795151935316,0,123,0");
            assert!(xml.contains("A &amp; B&#10;second line</d>"));
            for (_, attrs) in &rows {
                assert_eq!(attrs.contains_key("raw"), raw);
            }
            if raw {
                for ((_, attrs), body) in rows.iter().zip(messages) {
                    assert_eq!(attrs["raw"], body);
                }
            }
            if detail {
                assert_eq!(rows[0].1["user"], "sender");
                assert_eq!(rows[0].1["timestamp"], "1795151935316");
                let gift: Value = serde_json::from_str(&rows[1].1["detail"]).unwrap();
                assert_eq!(gift["items"][0]["tid"], "tid-1");
                assert!(gift["items"][0].get("price").is_none());
                assert!(gift["items"][0].get("total_coin").is_none());
                assert_eq!(gift["items"][1]["count"], 3);
                assert_eq!(gift["items"][1]["tid"], "tid-2");
                assert_eq!(gift["items"][1]["price"], 1000);
                assert_eq!(gift["items"][1]["total_coin"], 3000);
                let sc: Value = serde_json::from_str(&rows[2].1["detail"]).unwrap();
                assert_eq!(sc["id"], 42);
                assert_eq!(sc["price"], 30);
                assert_eq!(sc["time"], 60);
                assert_eq!(rows[2].1["price"], "30000");
                assert_eq!(rows[4].1["type"], "interact");
                assert_eq!(rows[5].1["type"], "room_status");
            }
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn chat_without_millisecond_send_time_uses_millisecond_receive_time() {
        for meta in [json!([0, 5, 36, 16777215, 1795151935]), json!([0, 5, 36])] {
            let body = json!({"cmd": "DANMU_MSG", "info": [meta, "text", [123, "sender"]]});
            let notification =
                Bilibili::parse_notification(body.to_string().as_bytes(), false, false).unwrap();
            let Some(DanmakuEvent::Chat(chat)) = notification.event else {
                panic!("expected a chat message");
            };
            let format = notification.chat_metadata.unwrap();
            assert_eq!((format.mode, format.size), (5, 36));
            assert_eq!(format.timestamp_ms, chat.timestamp.timestamp_millis());
        }
    }

    #[test]
    fn auth_identity_uses_the_same_cookie_as_token_headers() {
        // Synthetic identities only; no real account credentials.
        for uid in [12345u64, 67890] {
            let context = PlatformContext::new()
                .with_cookie(format!("SESSDATA=FIXTURE_ONLY; DedeUserID={uid}"));
            let selected_uid = Bilibili::auth_uid(&context).unwrap();
            assert_eq!(selected_uid, uid);
            let headers = Bilibili::default_headers(context.cookie.as_deref());
            assert!(
                headers[COOKIE]
                    .to_str()
                    .unwrap()
                    .ends_with(context.cookie.as_ref().unwrap())
            );
            let packet = Bilibili::build_auth_packet(selected_uid, 42, "FIXTURE_TOKEN");
            assert_eq!(BigEndian::read_u32(&packet[8..12]), op::AUTH);
            let body: Value = serde_json::from_slice(&packet[16..]).unwrap();
            assert_eq!(
                body,
                serde_json::json!({
                    "uid": uid, "roomid": 42, "protover": 3, "platform": "web",
                    "type": 2, "key": "FIXTURE_TOKEN"
                })
            );
        }
        assert_eq!(Bilibili::auth_uid(&PlatformContext::new()).unwrap(), 0);
    }

    #[test]
    fn invalid_or_conflicting_cookie_identity_is_rejected_without_credentials() {
        for cookie in [
            "SESSDATA=FIXTURE_ONLY",
            "DedeUserID=0",
            "DedeUserID=INVALID_FIXTURE",
            "DedeUserID=18446744073709551616",
            "DedeUserID=12345; DedeUserID=67890",
        ] {
            let error = Bilibili::auth_uid(&PlatformContext::new().with_cookie(cookie))
                .unwrap_err()
                .to_string();
            assert!(!error.contains("FIXTURE_ONLY"));
            assert!(!error.contains("INVALID_FIXTURE"));
            assert!(!error.contains("12345"));
            assert!(!error.contains("67890"));
        }
        let mut context = PlatformContext::new().with_cookie("DedeUserID=12345");
        context.uid = Some(67890);
        assert!(Bilibili::auth_uid(&context).is_err());
        context.uid = Some(0);
        assert!(Bilibili::auth_uid(&context).is_err());
        context.uid = Some(12345);
        assert_eq!(Bilibili::auth_uid(&context).unwrap(), 12345);
        context.cookie = Some("SESSDATA=FIXTURE_ONLY".into());
        assert!(Bilibili::auth_uid(&context).is_err());
    }

    #[test]
    fn authentication_requires_a_numeric_success_response() {
        let protocol = Bilibili::new();
        assert!(
            protocol
                .decode_message(&build_packet(br#"{"code":0}"#, op::AUTH_REPLY))
                .is_ok()
        );
        for body in [
            br#"{"code":-101,"message":"SECRET_FIXTURE"}"#.as_slice(),
            br#"{"code":"0"}"#,
            br#"{}"#,
            b"SECRET_FIXTURE",
        ] {
            let error = protocol
                .decode_message(&build_packet(body, op::AUTH_REPLY))
                .unwrap_err()
                .to_string();
            assert!(!error.contains("SECRET_FIXTURE"));
        }
    }

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

    /// 构造指定协议版本的数据包（build_packet 固定 ver=1）。
    fn build_packet_with_version(body: &[u8], operation: u32, version: u16) -> Vec<u8> {
        let mut packet = build_packet(body, operation);
        packet[6..8].copy_from_slice(&version.to_be_bytes());
        packet
    }

    fn zlib_compress(data: &[u8]) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::ZlibEncoder;
        use std::io::Write;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    /// packet_len=0：修复前反向切片 panic（若跳过切片还会因 offset 不前进死循环）。
    #[test]
    fn malformed_zero_length_packet_does_not_panic_or_hang() {
        let mut packet = build_packet(br#"{"cmd":"TEST"}"#, op::NOTIFICATION);
        packet[0..4].copy_from_slice(&0u32.to_be_bytes());

        assert!(Bilibili::decode_packet(&packet).is_empty());
    }

    /// packet_len < 16（小于包头长度）：修复前 &data[offset+16..offset+packet_len] 直接 panic。
    #[test]
    fn malformed_short_length_packet_does_not_panic() {
        let mut packet = build_packet(br#"{"cmd":"TEST"}"#, op::NOTIFICATION);
        packet[0..4].copy_from_slice(&8u32.to_be_bytes());

        assert!(Bilibili::decode_packet(&packet).is_empty());
    }

    /// 合法包之后跟畸形头：保留已解析的包，丢弃不可信的剩余数据。
    #[test]
    fn valid_packet_before_malformed_header_is_preserved() {
        let body = br#"{"cmd":"TEST"}"#;
        let mut data = build_packet(body, op::NOTIFICATION);
        let mut garbage = build_packet(b"x", op::NOTIFICATION);
        garbage[0..4].copy_from_slice(&3u32.to_be_bytes());
        data.extend_from_slice(&garbage);

        let decoded = Bilibili::decode_packet(&data);
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].body, body);
    }

    /// 正常一层压缩嵌套（真实协议形态）仍能解码。
    #[test]
    fn single_level_zlib_packet_still_decodes() {
        let body = br#"{"cmd":"TEST"}"#;
        let inner = build_packet(body, op::NOTIFICATION);
        let packet = build_packet_with_version(&zlib_compress(&inner), op::NOTIFICATION, ver::ZLIB);

        let decoded = Bilibili::decode_packet(&packet);
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].body, body);
    }

    #[test]
    fn compressed_payloads_stop_at_the_decompression_limit() {
        let data = vec![b'x'; 1025];
        assert!(decompress_zlib(&zlib_compress(&data), 1024).is_err());
        let mut compressed = Vec::new();
        brotli::BrotliCompress(
            &mut std::io::Cursor::new(&data),
            &mut compressed,
            &brotli::enc::BrotliEncoderParams::default(),
        )
        .unwrap();
        assert!(decompress_brotli(&compressed, 1024).is_err());
        assert_eq!(decompress_brotli(&compressed, 1025).unwrap(), data);
    }

    #[test]
    fn extended_headers_are_respected_and_invalid_headers_are_dropped() {
        let body = br#"{"cmd":"TEST"}"#;
        let mut packet = build_packet(body, op::NOTIFICATION);
        packet.splice(16..16, [0, 0, 0, 0]);
        packet[..4].copy_from_slice(&((20 + body.len()) as u32).to_be_bytes());
        packet[4..6].copy_from_slice(&20u16.to_be_bytes());
        assert_eq!(Bilibili::decode_packet(&packet)[0].body, body);
        for header_len in [0u16, 15, u16::MAX] {
            packet[4..6].copy_from_slice(&header_len.to_be_bytes());
            assert!(Bilibili::decode_packet(&packet).is_empty());
        }
    }

    /// 超过深度上限的恶意多层压缩嵌套被丢弃，不发生栈溢出。
    #[test]
    fn deeply_nested_compressed_packets_are_dropped() {
        let mut data = build_packet(br#"{"cmd":"TEST"}"#, op::NOTIFICATION);
        for _ in 0..(MAX_DECODE_DEPTH as usize + 8) {
            data = build_packet_with_version(&zlib_compress(&data), op::NOTIFICATION, ver::ZLIB);
        }

        assert!(Bilibili::decode_packet(&data).is_empty());
    }
}
