use crate::server::core::upload_streamers::{StudioEntity, UploadStreamersRepository};
use crate::server::infrastructure::connection_pool::ConnectionPool;

use anyhow::Context;
use async_trait::async_trait;
use sqlx::query_as;

#[derive(Clone)]
pub struct SqliteUploadStreamersRepository {
    pool: ConnectionPool,
}

impl SqliteUploadStreamersRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}
