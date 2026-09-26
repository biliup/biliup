use crate::LogHandle;
use crate::server::auto_clip::settings::{AutoClipConfig, restore_masked_keys};
use crate::server::config::Config;
use crate::server::core::download_manager::DownloadManager;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::Configuration;
use error_stack::{Report, ResultExt};
use std::sync::RwLock;
use tracing_subscriber::EnvFilter;

/// 保存配置失败的原因
#[derive(Debug)]
pub enum ApplyConfigError {
    /// 配置本身不合法，原样告诉用户；此时库、内存与运行状态都没有改动
    Invalid(String),
    /// 落库或生效时出错
    Internal(Report<AppError>),
}

impl From<Report<AppError>> for ApplyConfigError {
    fn from(report: Report<AppError>) -> Self {
        ApplyConfigError::Internal(report)
    }
}

/// 校验并保存全局配置，然后让它立即生效：ffmpeg 路径、下载池 / 上传池容量、日志级别，
/// 最后换掉内存里的配置。返回从库里读回的配置。
pub async fn apply_config(
    config: &RwLock<Config>,
    pool: &ConnectionPool,
    managers: &DownloadManager,
    log_handle: &LogHandle,
    new_config: Config,
) -> Result<Config, ApplyConfigError> {
    let mut new_config = new_config;
    new_config.normalize_segment_limits();
    new_config.validate_segment_limits()?;
    new_config
        .validate_pool_sizes()
        .map_err(ApplyConfigError::Invalid)?;
    // 界面拿到的 key 是掩码，交回来时换回原值；整块都没填时去掉，不多出 `auto_clip` 这个键
    new_config.auto_clip = match new_config
        .auto_clip
        .take()
        .and_then(AutoClipConfig::normalized)
    {
        Some(submitted) => {
            let stored = config.read().unwrap().auto_clip.clone();
            Some(
                restore_masked_keys(stored.as_ref(), submitted)
                    .map_err(ApplyConfigError::Invalid)?,
            )
        }
        None => None,
    };

    let saved = save_config(pool, &new_config).await?;
    // 提交后从 DB 重新加载配置
    let mut saved_config: Config =
        serde_json::from_str(&saved.value).change_context(AppError::Unknown)?;
    saved_config.normalize_segment_limits();
    saved_config.validate_segment_limits()?;
    crate::tools::set_configured_ffmpeg(saved_config.ffmpeg_path.as_deref());
    // 下载池 / 上传池的容量不是每次从配置里读的，要在这里同步过去才能不重启就生效
    managers.resize_pools(saved_config.pool1_size, saved_config.pool2_size);
    *config.write().unwrap() = saved_config;
    crate::server::auto_clip::runner::kick();
    let guard = config.read().unwrap();
    if let Some(loggers_level) = &guard.loggers_level {
        let new_filter = EnvFilter::try_new(loggers_level)
            .change_context(AppError::Custom(String::from("Invalid log level format")))?;

        log_handle
            .modify(|filter| *filter = new_filter)
            .change_context(AppError::Unknown)?;
    }

    Ok(guard.clone())
}

/// 把配置写进 `configuration` 表 `key = 'config'` 的那一行（没有就插入），返回写入后的整行
async fn save_config(pool: &ConnectionPool, config: &Config) -> AppResult<Configuration> {
    // 将 JSON 序列化为 TEXT 存库
    let value_txt = serde_json::to_string(config).change_context(AppError::Unknown)?;

    let mut tx = pool.begin().await.change_context(AppError::Unknown)?;

    // 最多取 2 条判断是否多行
    let ids: Vec<i64> =
        sqlx::query_scalar::<_, i64>("SELECT id FROM configuration WHERE key = ?1 LIMIT 2")
            .bind("config")
            .fetch_all(&mut *tx)
            .await
            .change_context(AppError::Unknown)?;

    let saved: Configuration = if ids.is_empty() {
        // 插入
        sqlx::query("INSERT INTO configuration (key, value) VALUES (?1, ?2)")
            .bind("config")
            .bind(&value_txt)
            .execute(&mut *tx)
            .await
            .change_context(AppError::Unknown)?;

        // 取 last_insert_rowid 并读回整行
        let id: i64 = sqlx::query_scalar::<_, i64>("SELECT last_insert_rowid()")
            .fetch_one(&mut *tx)
            .await
            .change_context(AppError::Unknown)?;

        sqlx::query_as::<_, Configuration>("SELECT id, key, value FROM configuration WHERE id = ?1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .change_context(AppError::Unknown)?
    } else if ids.len() == 1 {
        // 更新
        let id = ids[0];
        sqlx::query("UPDATE configuration SET value = ?1 WHERE id = ?2")
            .bind(&value_txt)
            .bind(id)
            .execute(&mut *tx)
            .await
            .change_context(AppError::Unknown)?;

        sqlx::query_as::<_, Configuration>("SELECT id, key, value FROM configuration WHERE id = ?1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .change_context(AppError::Unknown)?
    } else {
        // 多行报错
        return Err(Report::new(AppError::Custom(format!(
            "有多个空间配置同时存在 (key='config'): {} 行",
            ids.len()
        ))));
    };

    tx.commit().await.change_context(AppError::Unknown)?;
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use tracing_subscriber::{Registry, reload};

    struct Fixture {
        _dir: tempfile::TempDir,
        pool: ConnectionPool,
        config: RwLock<Config>,
        managers: DownloadManager,
        log_handle: LogHandle,
        /// 句柄只持有弱引用，过滤层被丢弃后换不了日志级别
        _filter: reload::Layer<EnvFilter, Registry>,
    }

    async fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let config = Config::default();
        let managers = DownloadManager::new(config.pool1_size, config.pool2_size, pool.clone());
        let (filter, log_handle) = reload::Layer::new(EnvFilter::new("info"));
        Fixture {
            _dir: dir,
            pool,
            config: RwLock::new(config),
            managers,
            log_handle,
            _filter: filter,
        }
    }

    impl Fixture {
        async fn apply(&self, new_config: Config) -> Result<Config, ApplyConfigError> {
            apply_config(
                &self.config,
                &self.pool,
                &self.managers,
                &self.log_handle,
                new_config,
            )
            .await
        }

        async fn saved_rows(&self) -> Vec<Config> {
            let values: Vec<String> =
                sqlx::query_scalar("SELECT value FROM configuration WHERE key = 'config'")
                    .fetch_all(&self.pool)
                    .await
                    .unwrap();
            values
                .iter()
                .map(|value| serde_json::from_str(value).unwrap())
                .collect()
        }

        fn pool_sizes(&self) -> (usize, usize) {
            (
                self.managers.download_pool_size(),
                self.managers.upload_pool_size(),
            )
        }
    }

    /// 保存后库里、内存里、池容量与日志级别都换成新值
    #[tokio::test]
    async fn applying_a_config_saves_it_and_takes_effect() {
        let f = fixture().await;
        let applied = f
            .apply(Config {
                pool1_size: 5,
                pool2_size: 2,
                segment_time: Some("  ".to_string()),
                loggers_level: Some("debug".to_string()),
                ..Config::default()
            })
            .await
            .expect("合法配置应保存成功");

        assert_eq!(applied.pool1_size, 5);
        assert_eq!(applied.segment_time, None, "空白的分段时长按未设置保存");
        let rows = f.saved_rows().await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pool1_size, 5);
        assert_eq!(rows[0].segment_time, None);
        assert_eq!(f.config.read().unwrap().pool2_size, 2);
        assert_eq!(f.pool_sizes(), (5, 2));
        assert_eq!(
            f.log_handle
                .with_current(|filter| filter.to_string())
                .unwrap(),
            "debug"
        );
    }

    /// 再次保存更新同一行，不会插入第二行
    #[tokio::test]
    async fn applying_again_updates_the_same_row() {
        let f = fixture().await;
        for pool1_size in [3, 7] {
            f.apply(Config {
                pool1_size,
                ..Config::default()
            })
            .await
            .expect("合法配置应保存成功");
        }

        let rows = f.saved_rows().await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pool1_size, 7);
        assert_eq!(f.pool_sizes().0, 7);
    }

    /// 池大小为 0 时给出可以直接展示的原因，库、内存与池容量都不变
    #[tokio::test]
    async fn an_invalid_pool_size_changes_nothing() {
        let f = fixture().await;
        let before = f.pool_sizes();
        let result = f
            .apply(Config {
                pool1_size: 0,
                loggers_level: Some("debug".to_string()),
                ..Config::default()
            })
            .await;

        let Err(ApplyConfigError::Invalid(message)) = result else {
            panic!("池大小为 0 应判为非法配置：{result:?}");
        };
        assert_eq!(message, "下载线程池大小（pool1_size）至少为 1");
        assert!(f.saved_rows().await.is_empty());
        assert_eq!(f.pool_sizes(), before);
        assert_eq!(f.config.read().unwrap().loggers_level, None);
        assert_eq!(
            f.log_handle
                .with_current(|filter| filter.to_string())
                .unwrap(),
            "info"
        );
    }

    /// 库里有多行配置时报错，已有的行、内存与池容量都不动
    #[tokio::test]
    async fn duplicate_config_rows_are_reported_without_applying() {
        let f = fixture().await;
        for _ in 0..2 {
            sqlx::query("INSERT INTO configuration (key, value) VALUES ('config', '{}')")
                .execute(&f.pool)
                .await
                .unwrap();
        }
        let before = f.pool_sizes();

        let result = f
            .apply(Config {
                pool1_size: 9,
                ..Config::default()
            })
            .await;

        let Err(ApplyConfigError::Internal(report)) = result else {
            panic!("多行配置应报内部错误：{result:?}");
        };
        assert!(matches!(
            report.current_context(),
            AppError::Custom(message) if message.contains("2 行")
        ));
        assert_eq!(f.pool_sizes(), before);
        assert_ne!(f.config.read().unwrap().pool1_size, 9);
    }

    /// 日志级别写错时报错；此前的步骤已经完成，配置已落库并生效
    #[tokio::test]
    async fn an_invalid_log_level_is_reported_after_the_config_takes_effect() {
        let f = fixture().await;
        let result = f
            .apply(Config {
                pool1_size: 4,
                loggers_level: Some("biliup=notalevel".to_string()),
                ..Config::default()
            })
            .await;

        let Err(ApplyConfigError::Internal(report)) = result else {
            panic!("非法的日志级别应报内部错误：{result:?}");
        };
        assert!(matches!(
            report.current_context(),
            AppError::Custom(message) if message == "Invalid log level format"
        ));
        assert_eq!(f.saved_rows().await[0].pool1_size, 4);
        assert_eq!(f.config.read().unwrap().pool1_size, 4);
        assert_eq!(f.pool_sizes().0, 4);
        assert_eq!(
            f.log_handle
                .with_current(|filter| filter.to_string())
                .unwrap(),
            "info"
        );
    }
}
