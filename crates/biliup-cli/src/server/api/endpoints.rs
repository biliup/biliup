use crate::server::core::download_actor::DownloadActorHandle;
use crate::server::core::live_streamers::{
    AddLiveStreamerDto, DynLiveStreamersRepository, DynLiveStreamersService, LiveStreamerDto,
    LiveStreamerEntity,
};
use crate::server::core::upload_streamers::{DynUploadStreamersRepository, StudioEntity};
use crate::server::core::users::{DynUsersRepository, User};
use crate::server::errors::{AppError, AppResult};

use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::repositories::models::{
    Configuration, FileItem, LiveStreamer, StreamerInfo, UploadStreamer,
};
use crate::server::infrastructure::service_register::ServiceRegister;
use axum::extract::{Path, State};
use axum::{Extension, Json};
use biliup::credential::login_by_cookies;
use ormlite::Model;
use serde_json::json;

pub async fn get_streamers_endpoint(
    State(pool): State<ConnectionPool>,
) -> AppResult<Json<Vec<LiveStreamer>>> {
    let live_streamers = LiveStreamer::select().fetch_all(&pool).await?;

    Ok(Json(live_streamers))
}

pub async fn get_configuration(
    State(pool): State<ConnectionPool>,
) -> AppResult<Json<serde_json::Value>> {
    let configurations = Configuration::select()
        .where_("key = 'config'")
        .fetch_one(&pool)
        .await?;

    Ok(Json(serde_json::from_str(&configurations.value)?))
}

pub async fn get_streamer_info(
    // Extension(streamers_service): Extension<DynUploadStreamersRepository>,
    State(pool): State<ConnectionPool>,
) -> AppResult<Json<Vec<StreamerInfo>>> {
    let streamer_infos = StreamerInfo::select().fetch_all(&pool).await?;
    let file_items = FileItem::select().fetch_all(&pool).await?;
    println!("{:?}", file_items);

    Ok(Json(streamer_infos))
}

pub async fn get_upload_streamers_endpoint(
    // Extension(streamers_service): Extension<DynUploadStreamersRepository>,
    State(pool): State<ConnectionPool>,
) -> AppResult<Json<Vec<UploadStreamer>>> {
    let uploader_streamers = UploadStreamer::select().fetch_all(&pool).await?;
    Ok(Json(uploader_streamers))
}

pub async fn get_upload_streamer_endpoint(
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
) -> AppResult<Json<UploadStreamer>> {
    let uploader_streamers = UploadStreamer::select()
        .where_("id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await?;
    Ok(Json(uploader_streamers))
}

pub async fn get_streamer_endpoint(
    Extension(streamers_service): Extension<DynLiveStreamersService>,
    Extension(download_actor_handle): Extension<DownloadActorHandle>,
    Path(id): Path<i64>,
) -> AppResult<Json<LiveStreamerDto>> {
    Ok(Json(streamers_service.get_streamer_by_id(id).await?))
}

pub async fn add_streamer_endpoint(
    Extension(streamers_service): Extension<DynLiveStreamersService>,
    Extension(download_actor_handle): Extension<DownloadActorHandle>,
    Json(request): Json<AddLiveStreamerDto>,
) -> AppResult<Json<LiveStreamerDto>> {
    download_actor_handle.add_streamer(&request.url);
    Ok(Json(streamers_service.add_streamer(request).await?))
}

pub async fn delete_streamer_endpoint(
    Extension(download_actor_handle): Extension<DownloadActorHandle>,
    Path(id): Path<i64>,
) -> AppResult<Json<()>> {
    Ok(Json(()))
}

pub async fn update_streamer_endpoint(
    Extension(download_actor_handle): Extension<DownloadActorHandle>,
    Json(request): Json<LiveStreamerEntity>,
) -> AppResult<Json<LiveStreamerDto>> {
    download_actor_handle.update_streamer(&request.url);
    Err(AppError::InvalidLoginAttempt)
}

pub async fn add_upload_streamer_endpoint(
    Extension(streamers_service): Extension<DynUploadStreamersRepository>,
    Json(request): Json<StudioEntity>,
) -> AppResult<Json<StudioEntity>> {
    Ok(Json(streamers_service.create_streamer(request).await?))
}

pub async fn delete_template_endpoint(
    Extension(streamers_service): Extension<DynUploadStreamersRepository>,
    Path(id): Path<i64>,
) -> AppResult<Json<()>> {
    Ok(Json(streamers_service.delete_streamer(id).await?))
}

pub async fn update_template_endpoint(
    Extension(streamers_service): Extension<DynUploadStreamersRepository>,
    Json(request): Json<StudioEntity>,
) -> AppResult<Json<StudioEntity>> {
    Ok(Json(streamers_service.update_streamer(request).await?))
}

pub async fn get_users_endpoint(
    State(pool): State<ConnectionPool>,
) -> AppResult<Json<Vec<serde_json::Value>>> {
    let configurations = Configuration::select()
        .where_("key = 'bilibili-cookies'")
        .fetch_all(&pool)
        .await?;
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

pub async fn add_user_endpoint(Json(request): Json<User>) -> AppResult<Json<User>> {
    Err(AppError::InternalServerError)
}

pub async fn delete_user_endpoint(Path(id): Path<i64>) -> AppResult<Json<()>> {
    Ok(Json(()))
}
