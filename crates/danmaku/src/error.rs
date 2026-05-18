//! Error types for the danmaku crate.

use thiserror::Error;

/// Result type alias for danmaku operations.
pub type Result<T> = std::result::Result<T, DanmakuError>;

/// Errors that can occur during danmaku recording.
#[derive(Error, Debug)]
pub enum DanmakuError {
    /// The URL does not match any supported platform.
    #[error("Unsupported platform for URL: {0}")]
    UnsupportedPlatform(String),

    /// WebSocket connection or communication error.
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    /// HTTP request error (e.g., when fetching WebSocket info).
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Error decoding protocol-specific message format.
    #[error("Protocol decode error: {0}")]
    Decode(String),

    /// Error writing XML output.
    #[error("XML write error: {0}")]
    Xml(String),

    /// General I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// The WebSocket connection was closed.
    #[error("Connection closed")]
    ConnectionClosed,

    /// Error during compression/decompression.
    #[error("Compression error: {0}")]
    Compression(String),

    /// Channel send error.
    #[error("Channel send error")]
    ChannelSend,

    /// The client has been stopped.
    #[error("Client stopped")]
    Stopped,
}

impl From<quick_xml::Error> for DanmakuError {
    fn from(err: quick_xml::Error) -> Self {
        DanmakuError::Xml(err.to_string())
    }
}

impl From<quick_xml::DeError> for DanmakuError {
    fn from(err: quick_xml::DeError) -> Self {
        DanmakuError::Xml(err.to_string())
    }
}
