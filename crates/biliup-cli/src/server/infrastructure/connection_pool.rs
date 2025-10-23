use crate::server::errors::{AppError, AppResult};
use error_stack::ResultExt;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::path::Path;
use tracing::info;

/// SQLite连接池类型别名
pub type ConnectionPool = Pool<Sqlite>;

/// 连接管理器
/// 负责管理SQLite数据库连接池的创建和配置
pub struct ConnectionManager;

impl ConnectionManager {
    /// 创建新的数据库连接池
    ///
    /// # 参数
    /// * `path` - 数据库文件路径
    ///
    /// # 返回
    /// 返回配置好的SQLite连接池
    pub async fn new_pool(path: &str) -> AppResult<ConnectionPool> {
        // 创建所有父级目录（如果不存在）
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .change_context(AppError::Unknown)
                .attach_with(|| path.to_string())?; // 创建 data/ 目录
        }

        let db_url = format!("sqlite://{path}");

        // 创建数据库文件（如果不存在）
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .change_context(AppError::Unknown)?;

        // 创建连接池，最大连接数设为2
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect(&db_url)
            .await
            .change_context(AppError::Custom(
                "error while initializing the database connection pool".to_string(),
            ))?;

        // 运行数据库迁移，确保数据库结构是最新的
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
