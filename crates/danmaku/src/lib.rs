//! Danmaku (live chat) recording client for various streaming platforms.
//!
//! This crate provides a Rust implementation for recording live stream chat
//! messages (danmaku) from various platforms like Twitch, Twitcasting, Bilibili,
//! Douyu, Huya, and Douyin.
//!
//! # Supported Platforms
//!
//! - **Twitch**: IRC over WebSocket
//! - **Twitcasting**: JSON over WebSocket
//! - **Bilibili**: Binary protocol with compression (planned)
//! - **Douyu**: STT protocol (planned)
//! - **Huya**: TARS protocol (planned)
//! - **Douyin**: Protobuf protocol (planned)
//!
//! # Example
//!
//! ```no_run
//! use danmaku::{DanmakuRecorder, RecorderConfig};
//!
//! #[tokio::main]
//! async fn main() -> danmaku::Result<()> {
//!     let config = RecorderConfig::new(
//!         "https://www.twitch.tv/shroud",
//!         "/tmp/danmaku_%Y%m%d_%H%M%S"
//!     );
//!
//!     let recorder = DanmakuRecorder::new(config)?;
//!     let handle = recorder.start();
//!
//!     // ... do other work ...
//!
//!     // Stop recording
//!     handle.stop().await?;
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod codec;
pub mod error;
pub mod message;
pub mod output;
pub mod protocols;

// Re-exports
pub use client::{DanmakuRecorder, RecorderConfig, RecorderHandle};
pub use error::{DanmakuError, Result};
pub use message::{ChatMessage, DanmakuEvent, GiftMessage, GuardBuyMessage, SuperChatMessage};
pub use output::XmlWriter;
pub use protocols::{
    create_platform, ConnectionInfo, DecodeResult, HeartbeatConfig, Platform, PlatformContext,
};
