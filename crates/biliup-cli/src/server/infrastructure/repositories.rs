use crate::server::config::Config;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::live_streamer::{InsertLiveStreamer, LiveStreamer};
use crate::server::infrastructure::models::upload_streamer::{
    InsertUploadStreamer, UploadStreamer,
};
use crate::server::infrastructure::models::{Configuration, InsertConfiguration};
use error_stack::{ResultExt, bail};
use ormlite::{Insert, Model};

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
