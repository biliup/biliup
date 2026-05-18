//! Danmaku message types.
//!
//! These types represent the various kinds of messages that can be received
//! from live streaming platforms.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Default color (white) for danmaku messages.
pub const DEFAULT_COLOR: u32 = 16777215;

/// A chat message (danmaku) from a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// The message content.
    pub content: String,
    /// The sender's display name.
    pub name: Option<String>,
    /// The sender's user ID.
    pub uid: Option<u64>,
    /// The message color as RGB integer (default: 16777215 = white).
    pub color: u32,
    /// When the message was received.
    pub timestamp: DateTime<Utc>,
}

impl ChatMessage {
    /// Create a new chat message with default color.
    pub fn new(content: String) -> Self {
        Self {
            content,
            name: None,
            uid: None,
            color: DEFAULT_COLOR,
            timestamp: Utc::now(),
        }
    }

    /// Set the sender's name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the sender's user ID.
    pub fn with_uid(mut self, uid: u64) -> Self {
        self.uid = Some(uid);
        self
    }

    /// Set the message color.
    pub fn with_color(mut self, color: u32) -> Self {
        self.color = color;
        self
    }
}

/// A gift message from a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftMessage {
    /// The sender's display name.
    pub name: String,
    /// The sender's user ID.
    pub uid: u64,
    /// The gift name.
    pub gift_name: String,
    /// The gift price (in platform-specific units).
    pub price: u64,
    /// Number of gifts sent.
    pub num: u32,
    /// Formatted content string.
    pub content: String,
    /// When the gift was sent.
    pub timestamp: DateTime<Utc>,
}

/// A Super Chat (paid highlighted message).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuperChatMessage {
    /// The sender's display name.
    pub name: String,
    /// The sender's user ID.
    pub uid: u64,
    /// The message content.
    pub content: String,
    /// The amount paid (in platform-specific units).
    pub price: u64,
    /// When the Super Chat was sent.
    pub timestamp: DateTime<Utc>,
}

/// A guard/membership purchase message (Bilibili-specific).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardBuyMessage {
    /// The buyer's display name.
    pub name: String,
    /// The buyer's user ID.
    pub uid: u64,
    /// The guard/membership level name.
    pub gift_name: String,
    /// The price paid.
    pub price: u64,
    /// Number of months purchased.
    pub num: u32,
    /// When the purchase was made.
    pub timestamp: DateTime<Utc>,
}

/// A user entering the room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnterMessage {
    /// The user's display name.
    pub name: String,
    /// The user's ID.
    pub uid: Option<u64>,
    /// When the user entered.
    pub timestamp: DateTime<Utc>,
}

/// Events that can be received from a live stream.
#[derive(Debug, Clone)]
pub enum DanmakuEvent {
    /// A chat message (danmaku).
    Chat(ChatMessage),
    /// A gift was sent.
    Gift(GiftMessage),
    /// A Super Chat was sent.
    SuperChat(SuperChatMessage),
    /// A guard/membership was purchased.
    GuardBuy(GuardBuyMessage),
    /// A user entered the room.
    Enter(EnterMessage),
    /// Other unrecognized message types.
    Other {
        /// Raw data for debugging.
        raw_data: String,
    },
}

impl DanmakuEvent {
    /// Get the message type as a string.
    pub fn msg_type(&self) -> &'static str {
        match self {
            DanmakuEvent::Chat(_) => "danmaku",
            DanmakuEvent::Gift(_) => "gift",
            DanmakuEvent::SuperChat(_) => "super_chat",
            DanmakuEvent::GuardBuy(_) => "guard_buy",
            DanmakuEvent::Enter(_) => "enter",
            DanmakuEvent::Other { .. } => "other",
        }
    }
}
