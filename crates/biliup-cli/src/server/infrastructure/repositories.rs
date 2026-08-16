use crate::server::config::Config;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::live_streamer::{InsertLiveStreamer, LiveStreamer};
use crate::server::infrastructure::models::upload_streamer::{
    InsertUploadStreamer, UploadStreamer,
};
use crate::server::infrastructure::models::{Configuration, InsertConfiguration};
use biliup::uploader::credential::{LoginInfo, lock_credential_file, normalize_credential_path};
use error_stack::{ResultExt, bail};
use ormlite::{Insert, Model};
use serde::Serialize;
use std::io::Write;
use std::path::{Path, PathBuf};

/// 根据ID获取直播主播信息
///
/// # 参数
/// * `pool` - 数据库连接池
/// * `id` - 主播ID
pub async fn get_streamer(pool: &ConnectionPool, id: i64) -> AppResult<LiveStreamer> {
    LiveStreamer::select()
        .where_("id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .change_context(AppError::Unknown)
}

/// 获取主播的上传配置
///
/// # 参数
/// * `pool` - 数据库连接池
/// * `id` - 主播ID
pub async fn get_upload_config(
    pool: &ConnectionPool,
    id: i64,
) -> AppResult<Option<UploadStreamer>> {
    let Some(id) = get_streamer(pool, id).await?.upload_streamers_id else {
        return Ok(None);
    };

    UploadStreamer::select()
        .where_("id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .change_context(AppError::Unknown)
}

/// 删除指定的直播主播
///
/// # 参数
/// * `pool` - 数据库连接池
/// * `id` - 主播ID
///
/// # 返回
/// 返回被删除的主播信息
pub async fn del_streamer(pool: &ConnectionPool, id: i64) -> AppResult<LiveStreamer> {
    let streamer = get_streamer(pool, id).await?;
    streamer
        .clone()
        .delete(pool)
        .await
        .change_context(AppError::Unknown)?;
    Ok(streamer)
}
/// 获取所有直播主播信息
///
/// # 参数
/// * `pool` - 数据库连接池
pub async fn get_all_streamer(pool: &ConnectionPool) -> AppResult<Vec<LiveStreamer>> {
    LiveStreamer::select()
        .fetch_all(pool)
        .await
        .change_context(AppError::Unknown)
}

/// 从数据库获取全局配置
///
/// # 参数
/// * `pool` - 数据库连接池
///
/// # 返回
/// 返回全局配置，如果不存在则返回默认配置
pub async fn get_config(pool: &ConnectionPool) -> AppResult<Config> {
    let configuration = Configuration::select()
        .where_("key = 'config'")
        .fetch_optional(pool)
        .await
        .change_context(AppError::Unknown)?;
    if let Some(configuration) = configuration {
        // 从数据库中解析配置JSON
        let mut json: Config =
            serde_json::from_str(&configuration.value).change_context(AppError::Unknown)?;
        json.normalize_segment_limits();
        json.validate_segment_limits()?;
        Ok(json)
    } else {
        // 如果数据库中没有配置，返回默认配置
        let config = Config::default();
        Ok(config)
    }
}

/// 插入或更新全局配置到数据库
///
/// # 参数
/// * `pool` - 数据库连接池
/// * `config` - 要保存的配置
pub async fn upsert_config(pool: &ConnectionPool, config: &Config) -> AppResult<Configuration> {
    let mut config = config.clone();
    config.normalize_segment_limits();
    config.validate_segment_limits()?;
    let value_txt = serde_json::to_string(&config).change_context(AppError::Unknown)?;
    let mut tx = pool.begin().await.change_context(AppError::Unknown)?;

    let ids: Vec<i64> = sqlx::query_scalar("SELECT id FROM configuration WHERE key = ?1 LIMIT 2")
        .bind("config")
        .fetch_all(&mut *tx)
        .await
        .change_context(AppError::Unknown)?;

    let saved = if ids.is_empty() {
        sqlx::query("INSERT INTO configuration (key, value) VALUES (?1, ?2)")
            .bind("config")
            .bind(&value_txt)
            .execute(&mut *tx)
            .await
            .change_context(AppError::Unknown)?;
        let id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
            .fetch_one(&mut *tx)
            .await
            .change_context(AppError::Unknown)?;
        sqlx::query_as::<_, Configuration>("SELECT id, key, value FROM configuration WHERE id = ?1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .change_context(AppError::Unknown)?
    } else if ids.len() == 1 {
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
        bail!(AppError::Custom(format!(
            "有多个空间配置同时存在 (key='config'): {} 行",
            ids.len()
        )));
    };

    tx.commit().await.change_context(AppError::Unknown)?;
    Ok(saved)
}

/// 插入或更新上传模板，按模板名保持配置文件导入幂等。
pub async fn upsert_upload_streamer_by_template_name(
    pool: &ConnectionPool,
    mut payload: InsertUploadStreamer,
) -> AppResult<UploadStreamer> {
    let ids: Vec<i64> =
        sqlx::query_scalar("SELECT id FROM uploadstreamers WHERE template_name = ?1 LIMIT 2")
            .bind(&payload.template_name)
            .fetch_all(pool)
            .await
            .change_context(AppError::Unknown)?;

    if ids.is_empty() {
        ormlite::Insert::insert(payload, pool)
            .await
            .change_context(AppError::Unknown)
    } else if ids.len() == 1 {
        let id = ids[0];
        payload.id = Some(id);
        payload
            .update_all_fields(pool)
            .await
            .change_context(AppError::Unknown)?;
        UploadStreamer::select()
            .where_("id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .change_context(AppError::Unknown)
    } else {
        bail!(AppError::Custom(format!(
            "有多个同名上传模板同时存在 (template_name='{}'): {} 行",
            payload.template_name,
            ids.len()
        )));
    }
}

/// 插入或更新直播间配置，按 URL 保持幂等。
pub async fn upsert_live_streamer_by_url(
    pool: &ConnectionPool,
    payload: InsertLiveStreamer,
) -> AppResult<LiveStreamer> {
    if let Some(mut streamer) = LiveStreamer::select()
        .where_("url = ?")
        .bind(&payload.url)
        .fetch_optional(pool)
        .await
        .change_context(AppError::Unknown)?
    {
        streamer.remark = payload.remark;
        streamer.filename_prefix = payload.filename_prefix;
        streamer.time_range = payload.time_range;
        streamer.upload_streamers_id = payload.upload_streamers_id;
        streamer.format = payload.format;
        streamer.override_cfg = payload.override_cfg;
        streamer.preprocessor = payload.preprocessor;
        streamer.segment_processor = payload.segment_processor;
        streamer.downloaded_processor = payload.downloaded_processor;
        streamer.postprocessor = payload.postprocessor;
        streamer.opt_args = payload.opt_args;
        streamer.excluded_keywords = payload.excluded_keywords;
        streamer
            .update_all_fields(pool)
            .await
            .change_context(AppError::Unknown)
    } else {
        payload.insert(pool).await.change_context(AppError::Unknown)
    }
}

/// 插入全局配置到数据库
///
/// # 参数
/// * `pool` - 数据库连接池
/// * `config` - 要保存的配置
pub async fn insert_config(pool: &ConnectionPool, config: &Config) -> AppResult<Configuration> {
    let mut config = config.clone();
    config.normalize_segment_limits();
    config.validate_segment_limits()?;
    let configuration = InsertConfiguration {
        key: "config".to_string(),
        value: serde_json::to_string(&config).unwrap(),
    }
    .insert(pool)
    .await
    .change_context(AppError::Unknown)?;
    Ok(configuration)
}

/// 获取所有上传配置
///
/// # 参数
/// * `pool` - 数据库连接池
pub async fn get_all_uploader(pool: &ConnectionPool) -> AppResult<Vec<UploadStreamer>> {
    UploadStreamer::select()
        .fetch_all(pool)
        .await
        .change_context(AppError::Unknown)
}

/// Register a CLI cookie file in the Web UI's user registry.
///
/// The CLI stores Bilibili credentials in a JSON file, while the Web UI stores
/// references to those files in the `configuration` table.  Keep the two
/// representations linked by registering an existing file at server startup.
/// The canonical path makes the entry usable even when callers use different
/// relative-path spellings, and the upsert is idempotent so restarting the
/// server does not create duplicate accounts.
pub async fn register_bilibili_cookie(
    pool: &ConnectionPool,
    cookie_file: impl AsRef<Path>,
) -> AppResult<Option<Configuration>> {
    let requested_cookie_file = cookie_file.as_ref();
    let cookie_file = normalize_credential_path(requested_cookie_file)
        .change_context(AppError::Unknown)
        .attach_with(|| format!("normalize cookie file {}", requested_cookie_file.display()))?;
    let _guard = lock_credential_file(&cookie_file)
        .await
        .change_context(AppError::Unknown)
        .attach_with(|| format!("lock cookie file {}", cookie_file.display()))?;
    if !cookie_file.is_file() {
        return Ok(None);
    }

    // Registration confers permission to read and eventually delete this file,
    // so require it to actually be a supported credential document.
    let file = std::fs::File::open(&cookie_file)
        .change_context(AppError::Unknown)
        .attach_with(|| format!("open cookie file {}", cookie_file.display()))?;
    serde_json::from_reader::<_, LoginInfo>(std::io::BufReader::new(file))
        .change_context(AppError::Custom("无效的 B 站凭据文件".into()))?;

    let cookie_path = cookie_file.to_string_lossy().into_owned();

    let inserted = sqlx::query_as::<_, Configuration>(
        r#"
        INSERT INTO configuration (key, value)
        VALUES ('bilibili-cookies', ?1)
        ON CONFLICT DO NOTHING
        RETURNING id, key, value
        "#,
    )
    .bind(&cookie_path)
    .fetch_optional(pool)
    .await
    .change_context(AppError::Unknown)?;
    if inserted.is_some() {
        return Ok(inserted);
    }

    Configuration::select()
        .where_("key = ? AND value = ?")
        .bind("bilibili-cookies")
        .bind(&cookie_path)
        .fetch_optional(pool)
        .await
        .change_context(AppError::Unknown)
}

#[derive(Debug, Serialize)]
pub struct DeletedBilibiliCookie {
    pub id: i64,
    pub file_deleted: bool,
    pub references_remaining: usize,
}

fn normalize_cookie_reference(
    value: impl AsRef<Path>,
    server_root: &Path,
) -> std::io::Result<PathBuf> {
    let value = value.as_ref();
    if value.is_absolute() {
        normalize_credential_path(value)
    } else {
        normalize_credential_path(server_root.join(value))
    }
}

fn references_path(value: &str, target: &Path, server_root: &Path) -> bool {
    normalize_cookie_reference(value, server_root)
        .map(|path| path == target)
        .unwrap_or(false)
}

fn restore_file(
    path: &Path,
    contents: &[u8],
    permissions: std::fs::Permissions,
) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("credentials");
    let temporary = parent.join(format!(
        ".{file_name}.biliup-restore-{}-{:016x}",
        std::process::id(),
        rand::random::<u64>()
    ));
    let result = (|| {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        let mut file = options.open(&temporary)?;
        file.set_permissions(permissions)?;
        file.write_all(contents)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// Delete one Web user registration and, if no other configuration or upload
/// template references it, remove the credential file as well.
pub async fn delete_bilibili_cookie(
    pool: &ConnectionPool,
    id: i64,
) -> AppResult<DeletedBilibiliCookie> {
    let server_root = std::env::current_dir().change_context(AppError::Unknown)?;
    delete_bilibili_cookie_from(pool, id, &server_root).await
}

async fn delete_bilibili_cookie_from(
    pool: &ConnectionPool,
    id: i64,
    server_root: &Path,
) -> AppResult<DeletedBilibiliCookie> {
    let initial = sqlx::query_as::<_, Configuration>(
        "SELECT id, key, value FROM configuration WHERE id = ?1 AND key = 'bilibili-cookies'",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .change_context(AppError::Unknown)?
    .ok_or_else(|| AppError::Custom("B 站用户不存在".into()))?;

    let target_path = normalize_cookie_reference(&initial.value, server_root)
        .change_context(AppError::Unknown)
        .attach_with(|| format!("normalize cookie file registered as user {id}"))?;
    let _guard = lock_credential_file(&target_path)
        .await
        .change_context(AppError::Unknown)?;

    let mut tx = pool.begin().await.change_context(AppError::Unknown)?;
    // Upgrade the SQLite transaction to a writer before checking references,
    // preventing a template or configuration insert from racing the decision.
    sqlx::query("UPDATE configuration SET id = id WHERE 0")
        .execute(&mut *tx)
        .await
        .change_context(AppError::Unknown)?;

    let target = sqlx::query_as::<_, Configuration>(
        "SELECT id, key, value FROM configuration WHERE id = ?1 AND key = 'bilibili-cookies'",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .change_context(AppError::Unknown)?
    .ok_or_else(|| AppError::Custom("B 站用户不存在".into()))?;
    if normalize_cookie_reference(&target.value, server_root).change_context(AppError::Unknown)?
        != target_path
    {
        bail!(AppError::Custom("用户凭据路径在删除期间发生变化".into()));
    }

    let configurations = sqlx::query_as::<_, Configuration>(
        "SELECT id, key, value FROM configuration WHERE id <> ?1",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .change_context(AppError::Unknown)?;
    let mut references_remaining = configurations
        .iter()
        .filter(|configuration| references_path(&configuration.value, &target_path, server_root))
        .count();

    let upload_cookie_paths: Vec<Option<String>> =
        sqlx::query_scalar("SELECT user_cookie FROM uploadstreamers")
            .fetch_all(&mut *tx)
            .await
            .change_context(AppError::Unknown)?;
    let default_cookie = normalize_cookie_reference("cookies.json", server_root)
        .change_context(AppError::Unknown)?;
    for reference in upload_cookie_paths {
        match reference {
            Some(reference) if references_path(&reference, &target_path, server_root) => {
                references_remaining += 1;
            }
            None if default_cookie == target_path => references_remaining += 1,
            _ => {}
        }
    }

    let mut quarantine: Option<PathBuf> = None;
    let mut backup = None;
    if references_remaining == 0 && target_path.exists() {
        let metadata = std::fs::metadata(&target_path)
            .change_context(AppError::Unknown)
            .attach("read credential metadata before deletion")?;
        if !metadata.is_file() {
            bail!(AppError::Custom("登记的凭据路径不是普通文件".into()));
        }
        const MAX_CREDENTIAL_SIZE: u64 = 16 * 1024 * 1024;
        if metadata.len() > MAX_CREDENTIAL_SIZE {
            bail!(AppError::Custom("凭据文件异常过大，拒绝自动删除".into()));
        }
        let contents = std::fs::read(&target_path)
            .change_context(AppError::Unknown)
            .attach("back up credential before deletion")?;
        let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = target_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("credentials");
        let quarantined = parent.join(format!(
            ".{file_name}.biliup-delete-{}-{:016x}",
            std::process::id(),
            rand::random::<u64>()
        ));
        std::fs::rename(&target_path, &quarantined)
            .change_context(AppError::Unknown)
            .attach("quarantine credential before deletion")?;
        quarantine = Some(quarantined);
        backup = Some((contents, metadata.permissions()));
    }

    let delete_result =
        sqlx::query("DELETE FROM configuration WHERE id = ?1 AND key = 'bilibili-cookies'")
            .bind(id)
            .execute(&mut *tx)
            .await;
    let delete_result = match delete_result {
        Ok(result) if result.rows_affected() == 1 => result,
        Ok(_) => {
            if let Some(quarantine) = quarantine.as_ref() {
                std::fs::rename(quarantine, &target_path)
                    .change_context(AppError::Unknown)
                    .attach("restore quarantined credential after an empty database delete")?;
            }
            bail!(AppError::Custom("B 站用户不存在".into()));
        }
        Err(error) => {
            if let Some(quarantine) = quarantine.as_ref() {
                if let Err(restore_error) = std::fs::rename(quarantine, &target_path) {
                    bail!(AppError::Custom(format!(
                        "数据库删除失败且无法恢复已隔离的凭据文件: {error}; {restore_error}"
                    )));
                }
            }
            return Err(error).change_context(AppError::Unknown);
        }
    };
    debug_assert_eq!(delete_result.rows_affected(), 1);

    if let Some(quarantine) = quarantine.as_ref()
        && let Err(error) = std::fs::remove_file(quarantine)
    {
        let restore_result = std::fs::rename(quarantine, &target_path);
        tx.rollback().await.change_context(AppError::Unknown)?;
        return match restore_result {
            Ok(()) => Err(error)
                .change_context(AppError::Custom("删除凭据文件失败，用户记录已保留".into())),
            Err(restore_error) => bail!(AppError::Custom(format!(
                "删除凭据文件失败且无法从隔离路径恢复: {error}; {restore_error}"
            ))),
        };
    }

    if let Err(commit_error) = tx.commit().await {
        if let Some((contents, permissions)) = backup {
            restore_file(&target_path, &contents, permissions)
                .change_context(AppError::Unknown)
                .attach("restore credential after database commit failure")?;
        }
        return Err(commit_error).change_context(AppError::Unknown);
    }

    Ok(DeletedBilibiliCookie {
        id,
        file_deleted: quarantine.is_some(),
        references_remaining,
    })
}

#[cfg(test)]
mod cookie_tests {
    use super::{delete_bilibili_cookie_from, register_bilibili_cookie};
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use std::fs;
    use std::path::Path;

    fn write_cookie(path: &Path) {
        fs::write(
            path,
            r#"{
                "cookie_info": {"cookies": []},
                "sso": [],
                "token_info": {
                    "access_token": "test-access-token",
                    "expires_in": 3600,
                    "mid": 1,
                    "refresh_token": "test-refresh-token"
                },
                "platform": "Android"
            }"#,
        )
        .unwrap();
    }

    async fn setup() -> (
        tempfile::TempDir,
        crate::server::infrastructure::connection_pool::ConnectionPool,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        (dir, pool)
    }

    #[tokio::test]
    async fn registers_existing_cookie_file_once_using_canonical_path() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let cookie = dir.path().join("cookies.json");
        write_cookie(&cookie);
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();

        let first = register_bilibili_cookie(&pool, &cookie).await.unwrap();
        let second = register_bilibili_cookie(&pool, &cookie).await.unwrap();

        assert_eq!(
            first.as_ref().map(|value| value.value.as_str()),
            Some(cookie.canonicalize().unwrap().to_str().unwrap())
        );
        assert_eq!(
            second.as_ref().map(|value| value.id),
            first.as_ref().map(|value| value.id)
        );
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM configuration WHERE key = 'bilibili-cookies'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn ignores_missing_cookie_file() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();

        assert!(
            register_bilibili_cookie(&pool, dir.path().join("missing.json"))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn concurrent_registration_is_atomic_and_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let cookie = dir.path().join("cookies.json");
        write_cookie(&cookie);
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();

        let mut tasks = Vec::new();
        for _ in 0..12 {
            let pool = pool.clone();
            let cookie = cookie.clone();
            tasks.push(tokio::spawn(async move {
                register_bilibili_cookie(&pool, cookie).await.unwrap()
            }));
        }
        for task in tasks {
            assert!(task.await.unwrap().is_some());
        }

        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM configuration WHERE key = 'bilibili-cookies'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn rejects_non_credential_json() {
        let (dir, pool) = setup().await;
        let cookie = dir.path().join("not-a-cookie.json");
        fs::write(&cookie, "{}").unwrap();

        assert!(register_bilibili_cookie(&pool, cookie).await.is_err());
    }

    #[tokio::test]
    async fn deletes_an_unreferenced_cookie_file_and_only_its_user_row() {
        let (dir, pool) = setup().await;
        let cookie = dir.path().join("private.json");
        write_cookie(&cookie);
        let user = register_bilibili_cookie(&pool, &cookie)
            .await
            .unwrap()
            .unwrap();
        sqlx::query("INSERT INTO configuration (key, value) VALUES ('config', '{}')")
            .execute(&pool)
            .await
            .unwrap();

        let deleted = delete_bilibili_cookie_from(&pool, user.id, dir.path())
            .await
            .unwrap();

        assert!(deleted.file_deleted);
        assert_eq!(deleted.references_remaining, 0);
        assert!(!cookie.exists());
        let config_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM configuration WHERE key = 'config'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(config_count, 1);
    }

    #[tokio::test]
    async fn shared_relative_and_symlink_references_preserve_the_file() {
        let (dir, pool) = setup().await;
        let cookie = dir.path().join("shared.json");
        write_cookie(&cookie);
        let user = register_bilibili_cookie(&pool, &cookie)
            .await
            .unwrap()
            .unwrap();

        #[cfg(unix)]
        let reference = {
            use std::os::unix::fs::symlink;
            let link = dir.path().join("linked.json");
            symlink(&cookie, &link).unwrap();
            "linked.json"
        };
        #[cfg(not(unix))]
        let reference = "shared.json";
        sqlx::query("INSERT INTO configuration (key, value) VALUES ('bilibili-cookies', ?1)")
            .bind(reference)
            .execute(&pool)
            .await
            .unwrap();

        let deleted = delete_bilibili_cookie_from(&pool, user.id, dir.path())
            .await
            .unwrap();
        assert!(!deleted.file_deleted);
        assert_eq!(deleted.references_remaining, 1);
        assert!(cookie.exists());
    }

    #[tokio::test]
    async fn explicit_upload_template_reference_preserves_the_file() {
        let (dir, pool) = setup().await;
        let cookie = dir.path().join("template.json");
        write_cookie(&cookie);
        let user = register_bilibili_cookie(&pool, &cookie)
            .await
            .unwrap()
            .unwrap();
        sqlx::query(
            "INSERT INTO uploadstreamers (template_name, tags, user_cookie) VALUES ('template', '[]', 'template.json')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let deleted = delete_bilibili_cookie_from(&pool, user.id, dir.path())
            .await
            .unwrap();
        assert!(!deleted.file_deleted);
        assert_eq!(deleted.references_remaining, 1);
        assert!(cookie.exists());
    }

    #[tokio::test]
    async fn null_upload_template_reference_preserves_the_default_cookie() {
        let (dir, pool) = setup().await;
        let cookie = dir.path().join("cookies.json");
        write_cookie(&cookie);
        let user = register_bilibili_cookie(&pool, &cookie)
            .await
            .unwrap()
            .unwrap();
        sqlx::query(
            "INSERT INTO uploadstreamers (template_name, tags, user_cookie) VALUES ('default-template', '[]', NULL)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let deleted = delete_bilibili_cookie_from(&pool, user.id, dir.path())
            .await
            .unwrap();
        assert!(!deleted.file_deleted);
        assert_eq!(deleted.references_remaining, 1);
        assert!(cookie.exists());
    }

    #[tokio::test]
    async fn refuses_to_delete_non_cookie_configuration_rows() {
        let (dir, pool) = setup().await;
        let result = sqlx::query("INSERT INTO configuration (key, value) VALUES ('config', '{}')")
            .execute(&pool)
            .await
            .unwrap();

        assert!(
            delete_bilibili_cookie_from(&pool, result.last_insert_rowid(), dir.path())
                .await
                .is_err()
        );
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM configuration WHERE key = 'config'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 1);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn filesystem_failure_keeps_the_user_row_and_file() {
        use std::os::unix::fs::PermissionsExt;

        let (dir, pool) = setup().await;
        let credential_dir = dir.path().join("credentials");
        fs::create_dir(&credential_dir).unwrap();
        let cookie = credential_dir.join("protected.json");
        write_cookie(&cookie);
        let user = register_bilibili_cookie(&pool, &cookie)
            .await
            .unwrap()
            .unwrap();
        fs::set_permissions(&credential_dir, fs::Permissions::from_mode(0o500)).unwrap();

        let result = delete_bilibili_cookie_from(&pool, user.id, dir.path()).await;
        fs::set_permissions(&credential_dir, fs::Permissions::from_mode(0o700)).unwrap();

        assert!(result.is_err());
        assert!(cookie.exists());
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM configuration WHERE id = ?1 AND key = 'bilibili-cookies'",
        )
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn database_delete_failure_restores_the_quarantined_file_and_row() {
        let (dir, pool) = setup().await;
        let cookie = dir.path().join("rollback.json");
        write_cookie(&cookie);
        let original = fs::read(&cookie).unwrap();
        let user = register_bilibili_cookie(&pool, &cookie)
            .await
            .unwrap()
            .unwrap();
        sqlx::query(&format!(
            "CREATE TRIGGER prevent_cookie_delete BEFORE DELETE ON configuration WHEN OLD.id = {} BEGIN SELECT RAISE(ABORT, 'blocked'); END",
            user.id
        ))
        .execute(&pool)
        .await
        .unwrap();

        assert!(
            delete_bilibili_cookie_from(&pool, user.id, dir.path())
                .await
                .is_err()
        );
        assert_eq!(fs::read(&cookie).unwrap(), original);
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM configuration WHERE id = ?1 AND key = 'bilibili-cookies'",
        )
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn qr_style_relative_cookie_path_is_deleted_from_the_server_root() {
        let (dir, pool) = setup().await;
        let data = dir.path().join("data");
        fs::create_dir(&data).unwrap();
        let cookie = data.join("42.json");
        write_cookie(&cookie);
        let inserted = sqlx::query(
            "INSERT INTO configuration (key, value) VALUES ('bilibili-cookies', 'data/42.json')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let deleted = delete_bilibili_cookie_from(&pool, inserted.last_insert_rowid(), dir.path())
            .await
            .unwrap();
        assert!(deleted.file_deleted);
        assert!(!cookie.exists());
    }

    #[tokio::test]
    async fn concurrent_deletes_remove_exactly_one_row_and_file() {
        let (dir, pool) = setup().await;
        let cookie = dir.path().join("concurrent-delete.json");
        write_cookie(&cookie);
        let user = register_bilibili_cookie(&pool, &cookie)
            .await
            .unwrap()
            .unwrap();

        let first = tokio::spawn({
            let pool = pool.clone();
            let root = dir.path().to_path_buf();
            async move { delete_bilibili_cookie_from(&pool, user.id, &root).await }
        });
        let second = tokio::spawn({
            let pool = pool.clone();
            let root = dir.path().to_path_buf();
            async move { delete_bilibili_cookie_from(&pool, user.id, &root).await }
        });
        let results = [first.await.unwrap(), second.await.unwrap()];

        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
        assert!(!cookie.exists());
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM configuration WHERE id = ?1 AND key = 'bilibili-cookies'",
        )
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 0);
    }
}
