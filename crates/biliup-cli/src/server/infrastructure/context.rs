use crate::server::config::Config;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::{LiveStreamer, UploadStreamer};
use crate::server::infrastructure::repositories::{get_config, get_streamer};
use axum::http::Extensions;
use biliup::client::StatelessClient;
use error_stack::ResultExt;
use ormlite::Model;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tracing::info;

#[derive(Debug, Clone)]
pub struct Context {
    pub worker: Arc<Worker>,
    pub extension: Extensions,
}

impl Context {
    pub fn new(worker: Arc<Worker>) -> Self {
        Self {
            worker,
            extension: Default::default(),
        }
    }

    // pub async fn get_config(&self) -> AppResult<Config> {
    //     get_config(&self.pool).await
    // }
}

#[derive(Debug)]
pub struct Worker {
    pub downloader_status: RwLock<WorkerStatus>,
    pub uploader_status: RwLock<WorkerStatus>,
    pub live_streamer: LiveStreamer,
    pub upload_streamer: Option<UploadStreamer>,
    pub config: Arc<RwLock<Config>>,
    pub client: StatelessClient,
}

impl Worker {
    pub fn new(
        live_streamer: LiveStreamer,
        upload_streamer: Option<UploadStreamer>,
        config: Arc<RwLock<Config>>,
        client: StatelessClient,
    ) -> Self {
        Self {
            downloader_status: RwLock::new(Default::default()),
            uploader_status: Default::default(),
            live_streamer,
            upload_streamer,
            config,
            client,
        }
    }

    pub fn get_streamer(&self) -> LiveStreamer {
        // get_streamer(&self.context.pool, self.id).await
        self.live_streamer.clone()
    }

    pub fn get_upload_config(&self) -> Option<UploadStreamer> {
        // let Some(id) = self.get_streamer().await?.upload_streamers_id else {
        //     return Ok(None);
        // };
        //
        // UploadStreamer::select()
        //     .where_("id = ?")
        //     .bind(id)
        //     .fetch_optional(&self.context.pool)
        //     .await
        //     .change_context(AppError::Unknown)
        self.upload_streamer.clone()
    }

    pub fn get_config(&self) -> Config {
        self.config.read().unwrap().clone()
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
        info!("Dropping worker {}", self.live_streamer.id);
    }
}

impl PartialEq for Worker {
    fn eq(&self, other: &Self) -> bool {
        self.live_streamer.id == other.live_streamer.id
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
