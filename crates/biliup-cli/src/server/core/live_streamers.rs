use crate::server::core::StreamStatus;
use async_trait::async_trait;
use biliup::uploader::bilibili::Studio;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;

/// Similar to above, we want to keep a reference count across threads so we can manage our connection pool.
pub type DynLiveStreamersRepository = Arc<dyn LiveStreamersRepository + Send + Sync>;
pub type DynDownloadRecordsRepository = Arc<dyn DownloadRecordsRepository + Send + Sync>;
pub type DynVideosRepository = Arc<dyn VideosRepository + Send + Sync>;

#[async_trait]
pub trait LiveStreamersRepository {
    async fn create_streamer(
        &self,
        entity: AddLiveStreamerDto,
    ) -> anyhow::Result<LiveStreamerEntity>;
    async fn delete_streamer(&self, id: i64) -> anyhow::Result<()>;
    async fn update_streamer(
        &self,
        entity: LiveStreamerEntity,
    ) -> anyhow::Result<LiveStreamerEntity>;
    async fn get_streamers(&self) -> anyhow::Result<Vec<LiveStreamerEntity>>;
    async fn get_streamer_by_url(&self, url: &str) -> anyhow::Result<LiveStreamerEntity>;
    async fn get_streamer_by_id(&self, id: i64) -> anyhow::Result<LiveStreamerEntity>;
}

#[async_trait]
pub trait DownloadRecordsRepository {
    async fn create(&self, entity: DownloadRecords) -> anyhow::Result<DownloadRecords>;
    // async fn delete(&self, id: i64) -> anyhow::Result<()>;
    // async fn update(&self, entity: DownloadRecords) -> anyhow::Result<DownloadRecords>;
    async fn get_all(&self) -> anyhow::Result<Vec<DownloadRecords>>;
    // async fn get_by_id(&self, id: i64) -> anyhow::Result<DownloadRecords>;
}

#[async_trait]
pub trait VideosRepository {
    async fn create(&self, entity: Videos) -> anyhow::Result<Videos>;
    // async fn delete(&self, id: i64) -> anyhow::Result<()>;
    async fn update(&self, entity: Videos) -> anyhow::Result<Videos>;
    // async fn get_all(&self) -> anyhow::Result<Vec<Videos>>;
    async fn get_by_id(&self, id: i64) -> anyhow::Result<Videos>;
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct LiveStreamerEntity {
    pub id: i64,
    pub url: String,
    pub remark: String,
    pub filename: String,
    pub split_time: Option<i64>,
    pub split_size: Option<i64>,
    pub upload_id: Option<i64>,
}

#[derive(FromRow)]
pub struct DownloadRecords {
    pub id: i64,
    pub title: String,
    pub live_id: i64,
    pub upload_id: Option<i64>,
    pub status: String,
}

#[derive(FromRow)]
pub struct Videos {
    pub id: i64,
    pub start_time: chrono::DateTime<Utc>,
    pub end_time: chrono::DateTime<Utc>,
    pub path: String,
    pub record_id: i64,
    pub status: String,
}

impl LiveStreamerEntity {
    pub fn into_dto(self) -> LiveStreamerDto {
        LiveStreamerDto {
            id: self.id,
            url: self.url,
            remark: self.remark,
            filename: self.filename,
            split_time: self.split_time.map(|t| t as u64),
            split_size: self.split_size.map(|s| s as u64),
            status: Default::default(),
        }
    }
}

/// A reference counter for our user service allows us safely pass instances user utils
/// around which themselves depend on the user repostiory, and ultimately, our Posgres connection pool.
pub type DynLiveStreamersService = Arc<dyn LiveStreamersService + Send + Sync>;

#[async_trait]
pub trait LiveStreamersService {
    async fn add_streamer(&self, request: AddLiveStreamerDto) -> anyhow::Result<LiveStreamerDto>;
    async fn get_streamer_by_url(&self, url: &str) -> anyhow::Result<LiveStreamerDto>;
    async fn get_streamer_by_id(&self, id: i64) -> anyhow::Result<LiveStreamerDto>;
    async fn get_streamers(&self) -> anyhow::Result<Vec<LiveStreamerDto>>;
    async fn get_studio_by_url(&self, url: &str) -> anyhow::Result<Option<Studio>>;
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct AddLiveStreamerDto {
    pub url: String,
    pub remark: String,
    pub filename: String,
    pub split_time: Option<u64>,
    pub split_size: Option<u64>,
    pub upload_id: Option<i64>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct LiveStreamerDto {
    pub id: i64,
    pub url: String,
    pub remark: String,
    pub filename: String,
    pub split_time: Option<u64>,
    pub split_size: Option<u64>,
    pub status: StreamStatus,
}
