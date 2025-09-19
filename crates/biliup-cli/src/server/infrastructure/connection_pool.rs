use anyhow::{Context, Result};
use ormlite::model::*;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use tracing::info;

pub type ConnectionPool = Pool<Sqlite>;

pub struct ConnectionManager;

impl ConnectionManager {
    pub async fn new_pool(path: &str) -> Result<ConnectionPool> {
        let db_url = format!("sqlite://{path}");
        /// Start by making a database connection.
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)?;
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect(&db_url)
            .await
            .context("error while initializing the database connection pool")?;

        /// Query builder syntax closely follows SQL syntax, translated into chained function calls.

        info!("migrations enabled, running...");
        sqlx::migrate!()
            .run(&pool)
            .await
            .context("error while running database migrations")?;

        Ok(pool)
    }
}
