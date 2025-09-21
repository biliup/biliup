use crate::server::errors::{AppError, AppResult};
use error_stack::ResultExt;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::path::Path;
use tracing::info;

pub type ConnectionPool = Pool<Sqlite>;

pub struct ConnectionManager;

impl ConnectionManager {
    pub async fn new_pool(path: &str) -> AppResult<ConnectionPool> {
        // 创建所有父级目录（如果不存在）
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .change_context(AppError::Unknown)
                .attach_lazy(|| path.to_string())?; // 创建 data/ 目录
        }
        let db_url = format!("sqlite://{path}");
        /// Start by making a database connection.
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)
            .change_context(AppError::Unknown)?;
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect(&db_url)
            .await
            .change_context(AppError::Custom(
                "error while initializing the database connection pool".to_string(),
            ))?;

        /// Query builder syntax closely follows SQL syntax, translated into chained function calls.

        info!("migrations enabled, running...");
        sqlx::migrate!()
            .run(&pool)
            .await
            .change_context(AppError::Custom(
                "error while running database migrations".to_string(),
            ))?;

        Ok(pool)
    }
}
