use crate::server::config::Config;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::{
    Configuration, InsertConfiguration, LiveStreamer, UploadStreamer,
};
use error_stack::ResultExt;
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
        let json: Config =
            serde_json::from_str(&configuration.value).change_context(AppError::Unknown)?;
        Ok(json)
    } else {
        // 如果数据库中没有配置，返回默认配置
        let config = Config::builder().streamers(Default::default()).build();
        Ok(config)
    }
}

/// 插入或更新全局配置到数据库
///
/// # 参数
/// * `pool` - 数据库连接池
/// * `config` - 要保存的配置
pub async fn insert_config(pool: &ConnectionPool, config: &Config) -> AppResult<Configuration> {
    let configuration = InsertConfiguration {
        key: "config".to_string(),
        value: serde_json::to_string(config).unwrap(),
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
