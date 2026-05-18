//! Twitch IRC protocol implementation.
//!
//! Twitch uses IRC over WebSocket for chat. The protocol is text-based
//! and relatively simple:
//!
//! - Connect to `wss://irc-ws.chat.twitch.tv`
//! - Send CAP, PASS, NICK, USER, and JOIN commands
//! - Send "PING" heartbeat every 40 seconds
//! - Parse PRIVMSG for chat messages

use std::time::Duration;

use async_trait::async_trait;
use regex::Regex;

use crate::error::Result;
use crate::message::{ChatMessage, DanmakuEvent};
use crate::protocols::{
    ConnectionInfo, DecodeResult, HeartbeatConfig, Platform, PlatformContext, RegistrationData,
};

/// Twitch IRC protocol implementation.
pub struct Twitch {
    /// Regex for parsing PRIVMSG messages.
    privmsg_regex: Regex,
    /// Regex for extracting display name.
    display_name_regex: Regex,
    /// Regex for extracting color.
    color_regex: Regex,
}

impl Twitch {
    /// Create a new Twitch protocol handler.
    pub fn new() -> Self {
        Self {
            privmsg_regex: Regex::new(r"PRIVMSG [^:]+:(.+)").unwrap(),
            display_name_regex: Regex::new(r"display-name=([^;]+);").unwrap(),
            color_regex: Regex::new(r"color=#([a-fA-F0-9]{6});").unwrap(),
        }
    }

    /// Extract room ID from URL.
    fn extract_room_id(url: &str) -> Option<String> {
        // Match patterns like:
        // https://www.twitch.tv/channel_name
        // twitch.tv/channel_name
        let re = Regex::new(r"twitch\.tv/([^/?]+)").ok()?;
        re.captures(url)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_lowercase())
    }

    /// Generate a random anonymous nick.
    fn random_nick() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let random_part = (timestamp % 80000) + 1000;
        format!("justinfan{}", random_part)
    }
}

impl Default for Twitch {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Platform for Twitch {
    fn name(&self) -> &'static str {
        "Twitch"
    }

    async fn get_connection_info(
        &self,
        url: &str,
        _context: &PlatformContext,
    ) -> Result<ConnectionInfo> {
        let room_id = Self::extract_room_id(url)
            .ok_or_else(|| crate::error::DanmakuError::Decode("Invalid Twitch URL".to_string()))?;

        let nick = Self::random_nick();

        // IRC registration commands
        let reg_data = vec![
            RegistrationData::Text(
                "CAP REQ :twitch.tv/tags twitch.tv/commands twitch.tv/membership".to_string(),
            ),
            RegistrationData::Text("PASS SCHMOOPIIE".to_string()),
            RegistrationData::Text(format!("NICK {}", nick)),
            RegistrationData::Text(format!("USER {} 8 * :{}", nick, nick)),
            RegistrationData::Text(format!("JOIN #{}", room_id)),
        ];

        Ok(ConnectionInfo::new("wss://irc-ws.chat.twitch.tv").with_registration(reg_data))
    }

    fn heartbeat_config(&self) -> HeartbeatConfig {
        HeartbeatConfig::text("PING", Duration::from_secs(40))
    }

    fn decode_message(&self, msg: &[u8]) -> Result<DecodeResult> {
        // Twitch uses text protocol
        let text = std::str::from_utf8(msg)
            .map_err(|e| crate::error::DanmakuError::Decode(e.to_string()))?;

        let mut events = Vec::new();

        for line in text.lines() {
            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Try to parse PRIVMSG
            if let Some(content_cap) = self.privmsg_regex.captures(line) {
                if let Some(content) = content_cap.get(1) {
                    let mut chat = ChatMessage::new(content.as_str().to_string());

                    // Extract display name
                    if let Some(name_cap) = self.display_name_regex.captures(line) {
                        if let Some(name) = name_cap.get(1) {
                            chat = chat.with_name(name.as_str());
                        }
                    }

                    // Extract color
                    if let Some(color_cap) = self.color_regex.captures(line) {
                        if let Some(color_hex) = color_cap.get(1) {
                            if let Ok(color) = u32::from_str_radix(color_hex.as_str(), 16) {
                                chat = chat.with_color(color);
                            }
                        }
                    }

                    events.push(DanmakuEvent::Chat(chat));
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
    fn test_extract_room_id() {
        assert_eq!(
            Twitch::extract_room_id("https://www.twitch.tv/shroud"),
            Some("shroud".to_string())
        );
        assert_eq!(
            Twitch::extract_room_id("twitch.tv/Channel_Name"),
            Some("channel_name".to_string())
        );
        assert_eq!(
            Twitch::extract_room_id("https://twitch.tv/xqc?ref=abc"),
            Some("xqc".to_string())
        );
    }

    #[test]
    fn test_decode_privmsg() {
        let twitch = Twitch::new();
        let msg = b"@badge-info=;badges=;color=#FF0000;display-name=TestUser;emotes=;flags=;id=abc;mod=0;room-id=123;subscriber=0;tmi-sent-ts=1234567890;turbo=0;user-id=456;user-type= :testuser!testuser@testuser.tmi.twitch.tv PRIVMSG #channel :Hello World!";

        let result = twitch.decode_message(msg).unwrap();
        assert_eq!(result.events.len(), 1);

        if let DanmakuEvent::Chat(chat) = &result.events[0] {
            assert_eq!(chat.content, "Hello World!");
            assert_eq!(chat.name, Some("TestUser".to_string()));
            assert_eq!(chat.color, 0xFF0000);
        } else {
            panic!("Expected Chat event");
        }
    }
}
