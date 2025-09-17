use crate::server::config::Config;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::{Configuration, LiveStreamer, UploadStreamer};
use crate::server::infrastructure::repositories::{get_config, get_streamer};
use error_stack::ResultExt;
use ormlite::Model;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tracing::info;

pub struct Context {
    uploads: Vec<UploadStreamer>,
    configuration: Configuration,
    streamers: Vec<LiveStreamer>,
}

impl Context {
    pub fn new(
        uploads: Vec<UploadStreamer>,
        configuration: Configuration,
        streamers: Vec<LiveStreamer>,
    ) -> Self {
        Self {
            uploads,
            configuration,
            streamers,
        }
    }
}

#[derive(Debug)]
pub struct Worker {
    pub(crate) id: i64,
    pub url: String,
    pool: ConnectionPool,
    pub downloader_status: RwLock<WorkerStatus>,
    pub uploader_status: RwLock<WorkerStatus>,
}

impl Worker {
    pub fn new(id: i64, url: &str, pool: ConnectionPool) -> Self {
        Self {
            id,
            pool,
            downloader_status: RwLock::new(Default::default()),
            uploader_status: Default::default(),
            url: url.to_string(),
        }
    }

    pub async fn get_streamer(&self) -> AppResult<LiveStreamer> {
        get_streamer(&self.pool, self.id).await
    }

    pub async fn get_config(&self) -> AppResult<Config> {
        get_config(&self.pool).await
    }

    pub async fn get_upload_config(&self) -> AppResult<Option<UploadStreamer>> {
        let Some(id) = self.get_streamer().await?.upload_streamers_id else {
            return Ok(None);
        };

        Ok(UploadStreamer::select()
            .where_("id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .change_context(AppError::Unknown)?)
    }

    pub fn change_status(&self, stage: Stage, status: WorkerStatus) {
        match stage {
            Stage::Download => {
                *self.downloader_status.write().unwrap() = status;
            }
            Stage::Upload => {
                *self.uploader_status.write().unwrap() = status;
            }
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        info!("Dropping worker {}", self.id);
    }
}

impl PartialEq for Worker {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Worker {}

#[derive(Debug)]
pub enum Stage {
    Download,
    Upload,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default, PartialEq)]
pub enum WorkerStatus {
    /// Stream is online.
    Working,
    /// The status of the stream could not be determined.
    Pending,
    #[default]
    Idle,
}
