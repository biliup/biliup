//! Platform protocol implementations.
//!
//! Each supported platform has its own module implementing the [`Platform`] trait.

pub mod bilibili;
pub mod douyin;
pub mod douyu;
pub mod huya;
pub mod twitch;
pub mod twitcasting;
pub mod wbi;

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::header::HeaderMap;

use crate::error::{DanmakuError, Result};
use crate::message::DanmakuEvent;

/// Registration data sent to WebSocket after connection.
#[derive(Debug, Clone)]
pub enum RegistrationData {
    /// Text message.
    Text(String),
    /// Binary message.
    Binary(Vec<u8>),
}

/// Connection information for a platform.
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// WebSocket URL to connect to.
    pub ws_url: String,
    /// Registration packets to send after connecting.
    pub registration_data: Vec<RegistrationData>,
    /// HTTP headers for the WebSocket connection.
    pub headers: HeaderMap,
}

impl ConnectionInfo {
    /// Create a new connection info with just a URL.
    pub fn new(ws_url: impl Into<String>) -> Self {
        Self {
            ws_url: ws_url.into(),
            registration_data: Vec::new(),
            headers: HeaderMap::new(),
        }
    }

    /// Add registration data.
    pub fn with_registration(mut self, data: Vec<RegistrationData>) -> Self {
        self.registration_data = data;
        self
    }

    /// Add headers.
    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }
}

/// Heartbeat configuration for a platform.
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// Heartbeat data to send (None if no heartbeat needed).
    pub data: Option<HeartbeatData>,
    /// Interval between heartbeats.
    pub interval: Duration,
}

impl HeartbeatConfig {
    /// Create a config with no heartbeat.
    pub fn none() -> Self {
        Self {
            data: None,
            interval: Duration::from_secs(30),
        }
    }

    /// Create a config with text heartbeat.
    pub fn text(msg: impl Into<String>, interval: Duration) -> Self {
        Self {
            data: Some(HeartbeatData::Text(msg.into())),
            interval,
        }
    }

    /// Create a config with binary heartbeat.
    pub fn binary(data: Vec<u8>, interval: Duration) -> Self {
        Self {
            data: Some(HeartbeatData::Binary(data)),
            interval,
        }
    }
}

/// Heartbeat data format.
#[derive(Debug, Clone)]
pub enum HeartbeatData {
    /// Text heartbeat message.
    Text(String),
    /// Binary heartbeat message.
    Binary(Vec<u8>),
}

/// Result of decoding a WebSocket message.
#[derive(Debug, Default)]
pub struct DecodeResult {
    /// Decoded events.
    pub events: Vec<DanmakuEvent>,
    /// Optional acknowledgment packet to send back (e.g., for Douyin).
    pub ack: Option<Vec<u8>>,
}

impl DecodeResult {
    /// Create an empty result.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a result with events.
    pub fn with_events(events: Vec<DanmakuEvent>) -> Self {
        Self { events, ack: None }
    }

    /// Add an acknowledgment packet.
    pub fn with_ack(mut self, ack: Vec<u8>) -> Self {
        self.ack = Some(ack);
        self
    }
}

/// Platform-specific context for connecting.
#[derive(Debug, Clone, Default)]
pub struct PlatformContext {
    /// Room ID (platform-specific format).
    pub room_id: Option<String>,
    /// User ID for authenticated connections.
    pub uid: Option<u64>,
    /// Cookie string for authenticated requests.
    pub cookie: Option<String>,
    /// Movie ID (Twitcasting-specific).
    pub movie_id: Option<String>,
    /// Password (Twitcasting-specific).
    pub password: Option<String>,
    /// Additional platform-specific configuration.
    pub extra: HashMap<String, String>,
}

impl PlatformContext {
    /// Create an empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the room ID.
    pub fn with_room_id(mut self, room_id: impl Into<String>) -> Self {
        self.room_id = Some(room_id.into());
        self
    }

    /// Set the user ID.
    pub fn with_uid(mut self, uid: u64) -> Self {
        self.uid = Some(uid);
        self
    }

    /// Set the cookie.
    pub fn with_cookie(mut self, cookie: impl Into<String>) -> Self {
        self.cookie = Some(cookie.into());
        self
    }
}

/// Trait for platform-specific protocol implementations.
#[async_trait]
pub trait Platform: Send + Sync {
    /// Get the platform name for logging.
    fn name(&self) -> &'static str;

    /// Get WebSocket connection info (URL, registration packets).
    async fn get_connection_info(
        &self,
        url: &str,
        context: &PlatformContext,
    ) -> Result<ConnectionInfo>;

    /// Get heartbeat configuration.
    fn heartbeat_config(&self) -> HeartbeatConfig;

    /// Decode a WebSocket message into danmaku events.
    ///
    /// For text-based protocols, `msg` contains UTF-8 text.
    /// For binary protocols, `msg` contains raw bytes.
    fn decode_message(&self, msg: &[u8]) -> Result<DecodeResult>;

    /// Whether this platform uses text-based WebSocket messages.
    ///
    /// If true, messages are expected to be valid UTF-8 text.
    /// If false, messages are treated as binary.
    fn is_text_protocol(&self) -> bool {
        false
    }
}

/// Create a platform instance based on URL.
pub fn create_platform(url: &str) -> Result<Box<dyn Platform>> {
    // Check each platform's URL pattern
    if url.contains("live.bilibili.com") {
        return Ok(Box::new(bilibili::Bilibili::new()));
    }

    if url.contains("twitch.tv") {
        return Ok(Box::new(twitch::Twitch::new()));
    }

    if url.contains("twitcasting.tv") {
        return Ok(Box::new(twitcasting::Twitcasting::new()));
    }

    if url.contains("douyu.com") {
        return Ok(Box::new(douyu::Douyu::new()));
    }

    if url.contains("huya.com") {
        return Ok(Box::new(huya::Huya::new()));
    }

    if url.contains("live.douyin.com") {
        return Ok(Box::new(douyin::Douyin::new()));
    }

    Err(DanmakuError::UnsupportedPlatform(url.to_string()))
}
