use crate::server::core::users::{User, UsersRepository};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use anyhow::Context;
use async_trait::async_trait;
use sqlx::{query, query_as};

#[derive(Clone)]
pub struct SqliteUsersStreamersRepository {
    pool: ConnectionPool,
}

impl SqliteUsersStreamersRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}
