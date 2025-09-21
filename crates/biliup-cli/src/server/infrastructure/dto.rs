use crate::server::infrastructure::context::WorkerStatus;
use crate::server::infrastructure::models::LiveStreamer;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LiveStreamerResponse {
    #[serde(flatten)]
    pub inner: LiveStreamer,

    pub status: WorkerStatus,
}
