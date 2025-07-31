use crate::server::core::download_actor::DownloadActorHandle;
use crate::server::core::live_streamers::{
    AddLiveStreamerDto, DynLiveStreamersRepository, DynLiveStreamersService, LiveStreamerDto,
    LiveStreamerEntity,
};
use crate::server::core::upload_streamers::{DynUploadStreamersRepository, StudioEntity};
use crate::server::core::users::{DynUsersRepository, User};
use crate::server::errors::AppResult;

use axum::extract::{Path, State};
use axum::{Extension, Json};

pub async fn get_streamers_endpoint(
    Extension(streamers_service): Extension<DynLiveStreamersService>,
    Extension(download_actor_handle): Extension<DownloadActorHandle>,
) -> AppResult<Json<Vec<LiveStreamerDto>>> {
    let map = download_actor_handle.get_streamers();
    let mut vec = streamers_service.get_streamers().await?;
    for live in vec.iter_mut() {
        live.status = map.get(&live.url).copied().unwrap_or_default()
    }
    Ok(Json(vec))
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
    State(state): State<DynLiveStreamersRepository>,
    Extension(download_actor_handle): Extension<DownloadActorHandle>,
    Path(id): Path<i64>,
) -> AppResult<Json<()>> {
    download_actor_handle.remove_streamer(&state.get_streamer_by_id(id).await?.url);
    Ok(Json(state.delete_streamer(id).await?))
}

pub async fn update_streamer_endpoint(
    State(state): State<DynLiveStreamersRepository>,
    Extension(download_actor_handle): Extension<DownloadActorHandle>,
    Json(request): Json<LiveStreamerEntity>,
) -> AppResult<Json<LiveStreamerDto>> {
    download_actor_handle.update_streamer(&request.url);
    Ok(Json(state.update_streamer(request).await?.into_dto()))
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

pub async fn get_upload_streamers_endpoint(
    Extension(streamers_service): Extension<DynUploadStreamersRepository>,
) -> AppResult<Json<Vec<StudioEntity>>> {
    Ok(Json(streamers_service.get_streamers().await?))
}

pub async fn get_upload_streamer_endpoint(
    Extension(streamers_service): Extension<DynUploadStreamersRepository>,
    Path(id): Path<i64>,
) -> AppResult<Json<StudioEntity>> {
    Ok(Json(streamers_service.get_streamer_by_id(id).await?))
}

pub async fn get_users_endpoint(
    State(state): State<DynUsersRepository>,
) -> AppResult<Json<Vec<User>>> {
    Ok(Json(state.get_users().await?))
}

pub async fn add_user_endpoint(
    State(state): State<DynUsersRepository>,
    Json(request): Json<User>,
) -> AppResult<Json<User>> {
    Ok(Json(state.create_user(request).await?))
}

pub async fn delete_user_endpoint(
    State(state): State<DynUsersRepository>,
    Path(id): Path<i64>,
) -> AppResult<Json<()>> {
    Ok(Json(state.delete_user(id).await?))
}
