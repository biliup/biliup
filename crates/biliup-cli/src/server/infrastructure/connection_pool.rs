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

#[cfg(test)]
mod tests {
    use super::ConnectionManager;

    /// 旧版本落库的覆写里 `file_size: null` 只是占位，迁移后必须消失（跟随全局），
    /// 其它显式设置的字段与真正的数值原样保留。
    #[tokio::test]
    async fn migration_strips_placeholder_null_file_size_from_overrides() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 4")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO livestreamers (url, remark, override) VALUES \
             ('https://live.bilibili.com/1', 'placeholder', '{\"downloader\":\"sync-downloader\",\"file_size\":null,\"bili_qn\":null}'), \
             ('https://live.bilibili.com/2', 'explicit-size', '{\"file_size\":52428800}'), \
             ('https://live.bilibili.com/3', 'no-override', NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let rows: Vec<(String, Option<String>)> =
            sqlx::query_as("SELECT remark, override FROM livestreamers ORDER BY id")
                .fetch_all(&pool)
                .await
                .unwrap();

        let placeholder: serde_json::Value =
            serde_json::from_str(rows[0].1.as_deref().unwrap()).unwrap();
        assert!(
            placeholder.get("file_size").is_none(),
            "占位 null 必须被移除: {placeholder}"
        );
        assert_eq!(placeholder["downloader"], "sync-downloader");
        assert!(
            placeholder["bili_qn"].is_null(),
            "其它字段的 null 无害，保持原样"
        );

        let explicit: serde_json::Value =
            serde_json::from_str(rows[1].1.as_deref().unwrap()).unwrap();
        assert_eq!(explicit["file_size"], 52_428_800);
        assert!(rows[2].1.is_none());
    }

    #[tokio::test]
    async fn identity_migration_fails_closed_on_multiple_existing_administrators() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        sqlx::query("DROP INDEX uq_configuration_biliup_identity")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM _sqlx_migrations WHERE version = 3")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO configuration (key, value) VALUES ('biliup', 'first'), ('biliup', 'second')",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool.close().await;

        assert!(
            ConnectionManager::new_pool(db.to_str().unwrap())
                .await
                .is_err()
        );

        let options = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&format!("sqlite://{}", db.display()))
            .await
            .unwrap();
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM configuration WHERE key = 'biliup'")
                .fetch_one(&options)
                .await
                .unwrap();
        assert_eq!(
            count, 2,
            "migration must not pick an administrator implicitly"
        );
    }
}
