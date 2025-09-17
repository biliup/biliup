use crate::server::errors::{AppError, AppResult, report_to_response};
use std::sync::{Arc, RwLock};

use crate::server::config::Config;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::Worker;
use crate::server::infrastructure::dto::LiveStreamerResponse;
use crate::server::infrastructure::models::{
    Configuration, FileItem, InsertLiveStreamer, InsertUploadStreamer, LiveStreamer, StreamerInfo,
    UploadStreamer,
};
use crate::server::infrastructure::repositories::{del_streamer, get_all_streamer, get_config};
use crate::server::infrastructure::service_register::ServiceRegister;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use error_stack::{ResultExt, bail};
use ormlite::query_builder::OnConflict;
use ormlite::{Insert, Model};
use serde_json::json;
use tracing::info;

pub async fn get_streamers_endpoint(
    State(pool): State<ConnectionPool>,
    State(workers): State<Arc<RwLock<Vec<Arc<Worker>>>>>,
) -> Result<Json<Vec<LiveStreamerResponse>>, Response> {
    let live_streamers = get_all_streamer(&pool).await.map_err(report_to_response)?;
    info!(
        "get_streamers_endpoint found {} live streamers",
        live_streamers.len()
    );
    Ok(Json(
        live_streamers
            .into_iter()
            .map(|x| LiveStreamerResponse {
                status: workers
                    .read()
                    .unwrap()
                    .iter()
                    .find_map(|worker| {
                        if worker.id == x.id {
                            Some(worker.downloader_status.read().unwrap().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default(),
                inner: x,
            })
            .collect(),
    ))
}

pub async fn post_streamers_endpoint(
    State(service_register): State<ServiceRegister>,
    State(pool): State<ConnectionPool>,
    Json(payload): Json<InsertLiveStreamer>,
) -> Result<Json<LiveStreamer>, Response> {
    let Some(manager) = service_register.get_manager(&payload.url) else {
        info!("not supported url: {}", &payload.url);
        return Err((StatusCode::BAD_REQUEST, "Not supported url").into_response());
    };
    let monitor = manager.ensure_monitor();

    /// You can insert the model directly.
    let live_streamers = payload
        .insert(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    service_register
        .add_room(live_streamers.id, &live_streamers.url, monitor)
        .await
        .map_err(report_to_response)?;

    info!(workers=?service_register.workers, "successfully inserted new live streamers");
    Ok(Json(live_streamers))
}

pub async fn put_streamers_endpoint(
    State(service_register): State<ServiceRegister>,
    State(pool): State<ConnectionPool>,
    Json(payload): Json<LiveStreamer>,
) -> Result<Json<LiveStreamer>, Response> {
    let streamer = payload
        .update_all_fields(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    let Some(manager) = service_register.get_manager(&streamer.url) else {
        info!("not supported url: {}", &streamer.url);
        return Err((StatusCode::BAD_REQUEST, "Not supported url").into_response());
    };
    let monitor = manager.ensure_monitor();

    service_register
        .del_room(streamer.id)
        .await
        .map_err(report_to_response)?;

    service_register
        .add_room(streamer.id, &streamer.url, monitor)
        .await
        .map_err(report_to_response)?;

    info!(workers=?streamer, "successfully update live streamers");
    Ok(Json(streamer))
}

pub async fn delete_streamers_endpoint(
    State(service_register): State<ServiceRegister>,
    State(pool): State<ConnectionPool>,
    State(workers): State<Arc<RwLock<Vec<Arc<Worker>>>>>,
    Path(id): Path<i64>,
) -> Result<Json<LiveStreamer>, Response> {
    let _ = service_register
        .del_room(id)
        .await
        .map_err(report_to_response)?;
    let live_streamers = del_streamer(&pool, id).await.map_err(report_to_response)?;
    info!(workers=?service_register.workers, "successfully inserted new live streamers");
    Ok(Json(live_streamers))
}

pub async fn get_configuration(
    State(config): State<Arc<RwLock<Config>>>,
) -> Result<Json<Config>, Response> {
    Ok(Json(config.read().unwrap().clone()))
}

pub async fn get_streamer_info(
    // Extension(streamers_service): Extension<DynUploadStreamersRepository>,
    State(pool): State<ConnectionPool>,
) -> Result<Json<Vec<StreamerInfo>>, Response> {
    let streamer_infos = StreamerInfo::select()
        .fetch_all(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    let file_items = FileItem::select()
        .fetch_all(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    println!("{:?}", file_items);

    Ok(Json(streamer_infos))
}

pub async fn get_upload_streamers_endpoint(
    // Extension(streamers_service): Extension<DynUploadStreamersRepository>,
    State(pool): State<ConnectionPool>,
) -> Result<Json<Vec<UploadStreamer>>, Response> {
    let uploader_streamers = UploadStreamer::select()
        .fetch_all(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    Ok(Json(uploader_streamers))
}

pub async fn add_upload_streamer_endpoint(
    // Extension(streamers_service): Extension<DynUploadStreamersRepository>,
    State(pool): State<ConnectionPool>,
    Json(upload_streamer): Json<InsertUploadStreamer>,
) -> Result<Json<serde_json::Value>, Response> {
    if upload_streamer.id.is_none() {
        Ok(Json(
            serde_json::to_value(
                ormlite::Insert::insert(upload_streamer, &pool)
                    .await
                    .change_context(AppError::Unknown)
                    .map_err(report_to_response)?,
            )
            .change_context(AppError::Unknown)
            .map_err(report_to_response)?,
        ))
    } else {
        Ok(Json(
            serde_json::to_value(
                upload_streamer
                    .update_all_fields(&pool)
                    .await
                    .change_context(AppError::Unknown)
                    .map_err(report_to_response)?,
            )
            .change_context(AppError::Unknown)
            .map_err(report_to_response)?,
        ))
    }
}

pub async fn get_upload_streamer_endpoint(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
) -> Result<Json<UploadStreamer>, Response> {
    let uploader_streamers = UploadStreamer::select()
        .where_("id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    Ok(Json(uploader_streamers))
}
pub async fn delete_template_endpoint(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
) -> Result<Json<()>, Response> {
    let uploader_streamers = UploadStreamer::select()
        .where_("id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    Ok(Json(
        uploader_streamers
            .delete(&pool)
            .await
            .change_context(AppError::Unknown)
            .map_err(report_to_response)?,
    ))
}

pub async fn get_users_endpoint(
    State(pool): State<ConnectionPool>,
) -> Result<Json<Vec<serde_json::Value>>, Response> {
    let configurations = Configuration::select()
        .where_("key = 'bilibili-cookies'")
        .fetch_all(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    let mut res = Vec::new();
    for cookies in configurations {
        res.push(json!({
            "id": cookies.id,
            "name": cookies.value,
            "value": cookies.value,
            "platform": cookies.key,
        }))
    }
    Ok(Json(res))
}

pub async fn delete_user_endpoint(Path(id): Path<i64>) -> Result<Json<()>, Response> {
    Ok(Json(()))
}
