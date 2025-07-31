use crate::server::core::live_streamers::{Videos, VideosRepository};

use crate::server::infrastructure::connection_pool::ConnectionPool;
use async_trait::async_trait;

#[derive(Clone)]
pub struct SqliteVideosRepository {
    pool: ConnectionPool,
}

impl SqliteVideosRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl VideosRepository for SqliteVideosRepository {
    async fn create(&self, entity: Videos) -> anyhow::Result<Videos> {
        todo!()
    }

    async fn update(&self, entity: Videos) -> anyhow::Result<Videos> {
        todo!()
    }

    async fn get_by_id(&self, id: i64) -> anyhow::Result<Videos> {
        todo!()
    }
}
