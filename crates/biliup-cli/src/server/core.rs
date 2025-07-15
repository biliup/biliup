use serde::{Deserialize, Serialize};

pub mod download_actor;
pub mod live_streamers;
pub mod main_loop;
pub mod upload_actor;
pub mod upload_streamers;
pub mod users;
pub mod util;

/// Status of the live stream
pub enum LiveStatus {
    /// Stream is online.
    Online,
    /// Stream is offline.
    Offline,
    /// The status of the stream could not be determined.
    Unknown,
}

/// Status of the live stream
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default, PartialEq)]
pub enum StreamStatus {
    /// Stream is online.
    Working,
    /// Stream is offline.
    Inspecting,
    /// The status of the stream could not be determined.
    #[default]
    Pending,
    Idle,
}
