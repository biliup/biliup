use crate::server::config::Config;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::{Configuration, LiveStreamer, UploadStreamer};
use crate::server::infrastructure::repositories::{get_config, get_streamer};
use biliup::client::StatelessClient;
use error_stack::ResultExt;
use ormlite::Model;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use tracing::info;

#[derive(Debug)]
pub struct Context {
    pool: ConnectionPool,
    pub client: StatelessClient,
}

impl Context {
    pub fn new(pool: ConnectionPool, client: StatelessClient) -> Self {
        Self { pool, client }
    }

    pub async fn get_config(&self) -> AppResult<Config> {
        get_config(&self.pool).await
    }
}

#[derive(Debug)]
pub struct Worker {
    pub(crate) id: i64,
    pub url: String,
    pub context: Context,
    pub downloader_status: RwLock<WorkerStatus>,
    pub uploader_status: RwLock<WorkerStatus>,
}

impl Worker {
    pub fn new(id: i64, url: &str, context: Context) -> Self {
        Self {
            id,
            context,
            downloader_status: RwLock::new(Default::default()),
            uploader_status: Default::default(),
            url: url.to_string(),
        }
    }

    pub async fn get_streamer(&self) -> AppResult<LiveStreamer> {
        get_streamer(&self.context.pool, self.id).await
    }

    pub async fn get_config(&self) -> AppResult<Config> {
        self.context.get_config().await
    }

    pub async fn get_upload_config(&self) -> AppResult<Option<UploadStreamer>> {
        let Some(id) = self.get_streamer().await?.upload_streamers_id else {
            return Ok(None);
        };

        UploadStreamer::select()
            .where_("id = ?")
            .bind(id)
            .fetch_optional(&self.context.pool)
            .await
            .change_context(AppError::Unknown)
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
