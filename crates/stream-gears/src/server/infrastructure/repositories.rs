use crate::server::config::Config;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::models::{Configuration, InsertConfiguration, LiveStreamer, UploadStreamer};
use error_stack::ResultExt;
use ormlite::{Insert, Model};

pub async fn get_streamer(pool: &ConnectionPool, id: i64) -> AppResult<LiveStreamer> {
    LiveStreamer::select()
        .where_("id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .change_context(AppError::Unknown)
}

pub async fn del_streamer(pool: &ConnectionPool, id: i64) -> AppResult<LiveStreamer> {
    let streamer = get_streamer(pool, id).await?;
    streamer
        .clone()
        .delete(pool)
        .await
        .change_context(AppError::Unknown)?;
    Ok(streamer)
}
pub async fn get_all_streamer(pool: &ConnectionPool) -> AppResult<Vec<LiveStreamer>> {
    LiveStreamer::select()
        .fetch_all(pool)
        .await
        .change_context(AppError::Unknown)
}

pub async fn get_config(pool: &ConnectionPool) -> AppResult<Config> {
    let configuration = Configuration::select()
        .where_("key = 'config'")
        .fetch_optional(pool)
        .await
        .change_context(AppError::Unknown)?;
    if let Some(configuration) = configuration {
        let json: Config =
            serde_json::from_str(&configuration.value).change_context(AppError::Unknown)?;
        Ok(json)
    } else {
        let config = Config::builder()
            .streamers(Default::default())
            .build();
        let configuration = InsertConfiguration {
            key: "config".to_string(),
            value: serde_json::to_string(&config).unwrap(),
        }.insert(pool).await.change_context(AppError::Unknown)?;
        let json: Config =
            serde_json::from_str(&configuration.value).change_context(AppError::Unknown)?;
        Ok(json)
    }

}

pub async fn get_all_uploader(pool: &ConnectionPool) -> AppResult<Vec<UploadStreamer>> {
    UploadStreamer::select()
        .fetch_all(pool)
        .await
        .change_context(AppError::Unknown)
}
