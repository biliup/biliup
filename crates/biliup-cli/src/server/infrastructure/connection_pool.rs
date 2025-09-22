use crate::server::errors::{AppError, AppResult};
use error_stack::ResultExt;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite, Row};
use std::path::Path;
use tracing::{info, warn, error};

pub type ConnectionPool = Pool<Sqlite>;

pub struct ConnectionManager;

impl ConnectionManager {
    pub async fn new_pool(path: &str) -> AppResult<ConnectionPool> {
        // 创建所有父级目录（如果不存在）
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .change_context(AppError::Unknown)
                .attach_with(|| path.to_string())?; // 创建 data/ 目录
        }
        let db_url = format!("sqlite://{path}");
        // Start by making a database connection.
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

        // 检查并修复现有表结构（兼容旧版本升级）
        info!("checking and updating existing table structures for compatibility...");
        Self::check_and_update_table_structures(&pool).await?;

        // Query builder syntax closely follows SQL syntax, translated into chained function calls.
        info!("migrations enabled, running...");
        sqlx::migrate!()
            .run(&pool)
            .await
            .change_context(AppError::Custom(
                "error while running database migrations".to_string(),
            ))?;

        Ok(pool)
    }

    /// 检查并更新现有表结构，确保与当前版本兼容
    async fn check_and_update_table_structures(pool: &ConnectionPool) -> AppResult<()> {
        // 定义期望的表结构
        let expected_tables = vec![
            ("uploadstreamers", vec![
                ("id", "INTEGER"),
                ("template_name", "VARCHAR"),
                ("title", "VARCHAR"),
                ("tid", "INTEGER"),
                ("copyright", "INTEGER"),
                ("copyright_source", "VARCHAR"),
                ("cover_path", "VARCHAR"),
                ("description", "TEXT"),
                ("dynamic", "VARCHAR"),
                ("dtime", "INTEGER"),
                ("dolby", "INTEGER"),
                ("hires", "INTEGER"),
                ("charging_pay", "INTEGER"),
                ("no_reprint", "INTEGER"),
                ("uploader", "VARCHAR"),
                ("user_cookie", "VARCHAR"),
                ("tags", "JSON"),
                ("credits", "JSON"),
                ("up_selection_reply", "INTEGER"),
                ("up_close_reply", "INTEGER"),
                ("up_close_danmu", "INTEGER"),
                ("extra_fields", "VARCHAR"),
                ("is_only_self", "INTEGER"),
            ]),
            ("streamerinfo", vec![
                ("id", "INTEGER"),
                ("name", "VARCHAR"),
                ("url", "VARCHAR"),
                ("title", "VARCHAR"),
                ("date", "DATETIME"),
                ("live_cover_path", "VARCHAR"),
            ]),
            ("livestreamers", vec![
                ("id", "INTEGER"),
                ("url", "VARCHAR"),
                ("remark", "VARCHAR"),
                ("filename_prefix", "VARCHAR"),
                ("time_range", "VARCHAR"),
                ("upload_streamers_id", "INTEGER"),
                ("format", "VARCHAR"),
                ("override", "JSON"),
                ("preprocessor", "JSON"),
                ("segment_processor", "JSON"),
                ("downloaded_processor", "JSON"),
                ("postprocessor", "JSON"),
                ("opt_args", "JSON"),
                ("excluded_keywords", "JSON"),
            ]),
            ("filelist", vec![
                ("id", "INTEGER"),
                ("file", "VARCHAR"),
                ("streamer_info_id", "INTEGER"),
            ]),
            ("configuration", vec![
                ("id", "INTEGER"),
                ("key", "VARCHAR"),
                ("value", "TEXT"),
            ]),
        ];

        for (table_name, expected_columns) in expected_tables {
            if let Err(e) = Self::check_and_update_table(pool, table_name, &expected_columns).await {
                warn!("Failed to update table {}: {}", table_name, e);
                // 继续处理其他表，不因为一个表失败而停止整个迁移
            }
        }

        Ok(())
    }

    /// 检查并更新单个表的结构
    async fn check_and_update_table(
        pool: &ConnectionPool,
        table_name: &str,
        expected_columns: &[(&str, &str)],
    ) -> AppResult<()> {
        // 检查表是否存在
        let table_exists = sqlx::query_scalar::<_, i32>(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?"
        )
        .bind(table_name)
        .fetch_one(pool)
        .await
        .change_context(AppError::Unknown)?;

        if table_exists == 0 {
            info!("Table {} does not exist, will be created by migration", table_name);
            return Ok(());
        }

        // 获取现有表的列信息
        let existing_columns = sqlx::query(
            "SELECT name, type FROM pragma_table_info(?)"
        )
        .bind(table_name)
        .fetch_all(pool)
        .await
        .change_context(AppError::Unknown)?;

        let mut existing_column_map = std::collections::HashMap::new();
        for row in existing_columns {
            let name: String = row.get("name");
            let col_type: String = row.get("type");
            existing_column_map.insert(name, col_type);
        }

        // 检查并添加缺失的列
        for (column_name, expected_type) in expected_columns {
            if !existing_column_map.contains_key(*column_name) {
                info!("Adding missing column {} to table {}", column_name, table_name);
                
                // 根据列名和类型确定默认值
                let default_value = Self::get_column_default_value(*column_name, *expected_type);
                
                let alter_sql = format!(
                    "ALTER TABLE {} ADD COLUMN {} {} {}",
                    table_name,
                    column_name,
                    expected_type,
                    default_value
                );
                
                if let Err(e) = sqlx::query(&alter_sql).execute(pool).await {
                    error!("Failed to add column {} to table {}: {}", column_name, table_name, e);
                    return Err(AppError::Custom(format!(
                        "Failed to add column {} to table {}: {}",
                        column_name, table_name, e
                    )));
                }
                
                info!("Successfully added column {} to table {}", column_name, table_name);
            }
        }

        Ok(())
    }

    /// 根据列名和类型获取默认值
    fn get_column_default_value(column_name: &str, column_type: &str) -> String {
        match (column_name, column_type) {
            // JSON 类型字段默认为空数组
            (_, "JSON") => "DEFAULT '[]'".to_string(),
            // 非空字段的默认值
            ("tags", _) => "DEFAULT '[]'".to_string(),
            ("credits", _) => "DEFAULT '[]'".to_string(),
            ("template_name", _) => "DEFAULT ''".to_string(),
            ("remark", _) => "DEFAULT ''".to_string(),
            ("name", _) => "DEFAULT ''".to_string(),
            ("url", _) => "DEFAULT ''".to_string(),
            ("title", _) => "DEFAULT ''".to_string(),
            ("live_cover_path", _) => "DEFAULT ''".to_string(),
            ("file", _) => "DEFAULT ''".to_string(),
            ("key", _) => "DEFAULT ''".to_string(),
            ("value", _) => "DEFAULT ''".to_string(),
            // 整数类型字段默认为0或NULL
            ("id", _) => "".to_string(), // 主键不需要默认值
            ("tid", _) => "DEFAULT 0".to_string(),
            ("copyright", _) => "DEFAULT 0".to_string(),
            ("dtime", _) => "DEFAULT 0".to_string(),
            ("dolby", _) => "DEFAULT 0".to_string(),
            ("hires", _) => "DEFAULT 0".to_string(),
            ("charging_pay", _) => "DEFAULT 0".to_string(),
            ("no_reprint", _) => "DEFAULT 0".to_string(),
            ("up_selection_reply", _) => "DEFAULT 0".to_string(),
            ("up_close_reply", _) => "DEFAULT 0".to_string(),
            ("up_close_danmu", _) => "DEFAULT 0".to_string(),
            ("is_only_self", _) => "DEFAULT 0".to_string(),
            ("upload_streamers_id", _) => "DEFAULT 0".to_string(),
            ("streamer_info_id", _) => "DEFAULT 0".to_string(),
            // 其他字段默认为NULL
            _ => "".to_string(),
        }
    }
}
