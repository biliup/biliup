//! Manual export task snapshots and API views.
pub use crate::server::common::processing::model::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderAsset {
    pub id: i64,
    pub path: PathBuf,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderSpec {
    pub session_id: i64,
    pub clip_id: Option<i64>,
    /// Name captured at enqueue, so retries do not depend on later title edits.
    #[serde(default)]
    pub output_name: Option<String>,
    pub in_ms: i64,
    pub out_ms: i64,
    pub recipe: RenderRecipe,
    pub sources: Vec<RenderSource>,
    pub assets: Vec<RenderAsset>,
    #[serde(default)]
    pub font_path: Option<PathBuf>,
}
#[derive(Debug, Clone, Serialize)]
pub struct AssetView {
    pub id: i64,
    pub width: u32,
    pub height: u32,
    pub sha256: String,
    pub url: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct JobView {
    pub id: i64,
    pub session_id: i64,
    pub clip_id: Option<i64>,
    pub state: String,
    pub phase: String,
    pub ratio: Option<f64>,
    pub error: Option<String>,
    pub output_bytes: Option<i64>,
    pub duration_ms: Option<i64>,
    pub created_at: i64,
    pub download_url: Option<String>,
    pub file_name: Option<String>,
    pub session_name: String,
    pub session_title: String,
    pub requested_encoder: String,
    pub requested_device: Option<String>,
    pub encoder: Option<String>,
    pub encoder_device: Option<String>,
    pub fallback_reason: Option<String>,
    pub fps: Option<f64>,
    pub speed: Option<f64>,
    pub processed_ms: Option<i64>,
    pub total_ms: Option<i64>,
    pub started_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub updated_at: i64,
    pub elapsed_ms: i64,
    pub eta_ms: Option<i64>,
}
#[derive(Debug, Clone)]
pub struct JobRecord {
    pub view: JobView,
    pub spec: RenderSpec,
    pub output_path: Option<PathBuf>,
}
