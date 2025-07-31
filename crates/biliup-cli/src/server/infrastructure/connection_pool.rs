use anyhow::{Context, Result};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use tracing::info;

pub type ConnectionPool = Pool<Sqlite>;

pub struct ConnectionManager;

impl ConnectionManager {
    pub async fn new_pool() -> Result<ConnectionPool> {
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open("data.db")?;
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect("sqlite://data.db")
            .await
            .context("error while initializing the database connection pool")?;

        info!("migrations enabled, running...");
        sqlx::migrate!()
            .run(&pool)
            .await
            .context("error while running database migrations")?;

        Ok(pool)
    }
}
