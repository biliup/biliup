use crate::server::core::live_streamers::{
    AddLiveStreamerDto, LiveStreamerEntity, LiveStreamersRepository,
};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use anyhow::Context;
use async_trait::async_trait;
use sqlx::query_as;

#[derive(Clone)]
pub struct SqliteLiveStreamersRepository {
    pool: ConnectionPool,
}

impl SqliteLiveStreamersRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}
