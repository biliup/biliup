use crate::UploadLine;
use crate::server::common::upload::{build_studio, submit_to_bilibili, upload};
use crate::server::common::util::Recorder;
use crate::server::config::Config;
use crate::server::core::download_manager::ActorHandle;
use crate::server::errors::{AppError, report_to_response};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::{Worker, WorkerStatus};
use crate::server::infrastructure::dto::LiveStreamerResponse;
use crate::server::infrastructure::models::live_streamer::{InsertLiveStreamer, LiveStreamer};
use crate::server::infrastructure::models::upload_streamer::{
    InsertUploadStreamer, UploadStreamer,
};
use crate::server::infrastructure::models::{
    Configuration, FileItem, InsertConfiguration, StreamerInfo,
};
use crate::server::infrastructure::repositories::{
    del_streamer, get_all_streamer, get_upload_config,
};
use crate::server::infrastructure::service_register::ServiceRegister;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use biliup::credential::Credential;
use chrono::Utc;
use clap::ValueEnum;
use error_stack::{Report, ResultExt};
use ormlite::{Insert, Model};
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, UNIX_EPOCH};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::info;

pub async fn get_streamers_endpoint(
    State(pool): State<ConnectionPool>,
    State(workers): State<Arc<RwLock<Vec<Arc<Worker>>>>>,
) -> Result<Json<Vec<LiveStreamerResponse>>, Response> {
    let live_streamers = get_all_streamer(&pool).await.map_err(report_to_response)?;
    let mut results = Vec::new();
    for x in live_streamers {
        let option = workers
            .read()
            .unwrap()
            .clone()
            .into_iter()
            .find(|worker| worker.live_streamer.id == x.id);

        let status = match option.as_ref() {
            Some(t) => format!("{:?}", *t.downloader_status.read().await),
            None => String::new(),
        };

        results.push(LiveStreamerResponse {
            status,
            inner: x,
            upload_status: option
                .map(|t| format!("{:?}", *t.uploader_status.read().unwrap()))
                .unwrap_or_default(),
        });
    }
    Ok(Json(results))
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

    // You can insert the model directly.
    let live_streamers = payload
        .insert(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    let upload_config = get_upload_config(&pool, live_streamers.id)
        .await
        .map_err(report_to_response)?;
    service_register
        .add_room(manager, live_streamers.clone(), upload_config)
        .await
        .map_err(report_to_response)?;

    info!(workers=?live_streamers, "successfully inserted new live streamers");
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

    service_register
        .del_room(streamer.id)
        .await
        .map_err(report_to_response)?;

    let upload_config = get_upload_config(&pool, streamer.id)
        .await
        .map_err(report_to_response)?;

    service_register
        .add_room(manager, streamer.clone(), upload_config)
        .await
        .map_err(report_to_response)?;

    info!(workers=?streamer, "successfully update live streamers");
    Ok(Json(streamer))
}

pub async fn delete_streamers_endpoint(
    State(service_register): State<ServiceRegister>,
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
) -> Result<Json<LiveStreamer>, Response> {
    service_register
        .del_room(id)
        .await
        .map_err(report_to_response)?;
    let live_streamers = del_streamer(&pool, id).await.map_err(report_to_response)?;
    info!(workers=?live_streamers, "successfully inserted new live streamers");
    Ok(Json(live_streamers))
}

// #[axum::debug_handler(state = ServiceRegister)]
pub async fn pause_streamers_endpoint(
    State(service_register): State<ServiceRegister>,
    State(pool): State<ConnectionPool>,
    State(workers): State<Arc<RwLock<Vec<Arc<Worker>>>>>,
    Path(id): Path<i64>,
) -> Result<Json<()>, Response> {
    let option = workers
        .read()
        .unwrap()
        .clone()
        .into_iter()
        .find(|worker| worker.live_streamer.id == id);
    if let Some(w) = option {
        let manager = service_register
            .get_manager(&w.live_streamer.url)
            .ok_or(AppError::Unknown)
            .map_err(report_to_response)?;
        let monitor = manager.ensure_monitor(pool.clone());
        let worker_status = w.downloader_status.read().await.clone();
        match worker_status {
            WorkerStatus::Working(d) => {
                d.stop().await.map_err(report_to_response)?;
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                monitor
                    .rooms_handle
                    .toggle(w.clone(), WorkerStatus::Pause)
                    .await;
                info!(workers=?&w.live_streamer.url, "successfully pause live streamers");
            }
            WorkerStatus::Pause => {
                monitor
                    .rooms_handle
                    .toggle(w.clone(), WorkerStatus::Idle)
                    .await;
                info!(workers=?&w.live_streamer.url, "successfully start live streamers");
            }
            _ => {
                monitor
                    .rooms_handle
                    .toggle(w.clone(), WorkerStatus::Pause)
                    .await;
                info!("非预期状态")
            }
        };
    }

    Ok(Json(()))
}

pub async fn get_configuration(
    State(config): State<Arc<RwLock<Config>>>,
) -> Result<Json<Config>, Response> {
    Ok(Json(config.read().unwrap().clone()))
}

// #[axum_macros::debug_handler(state = ServiceRegister)]
pub async fn put_configuration(
    State(config): State<Arc<RwLock<Config>>>,
    State(pool): State<ConnectionPool>,
    Json(json_data): Json<Config>,
) -> Result<Json<Config>, Response> {
    // 将 JSON 序列化为 TEXT 存库
    let value_txt = serde_json::to_string(&json_data)
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;

    let mut tx = pool
        .begin()
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;

    // 最多取 2 条判断是否多行
    let ids: Vec<i64> =
        sqlx::query_scalar::<_, i64>("SELECT id FROM configuration WHERE key = ?1 LIMIT 2")
            .bind("config")
            .fetch_all(&mut *tx)
            .await
            .change_context(AppError::Unknown)
            .map_err(report_to_response)?;

    let saved: Configuration = if ids.is_empty() {
        // 插入
        sqlx::query("INSERT INTO configuration (key, value) VALUES (?1, ?2)")
            .bind("config")
            .bind(&value_txt)
            .execute(&mut *tx)
            .await
            .change_context(AppError::Unknown)
            .map_err(report_to_response)?;

        // 取 last_insert_rowid 并读回整行
        let id: i64 = sqlx::query_scalar::<_, i64>("SELECT last_insert_rowid()")
            .fetch_one(&mut *tx)
            .await
            .change_context(AppError::Unknown)
            .map_err(report_to_response)?;

        sqlx::query_as::<_, Configuration>("SELECT id, key, value FROM configuration WHERE id = ?1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .change_context(AppError::Unknown)
            .map_err(report_to_response)?
    } else if ids.len() == 1 {
        // 更新
        let id = ids[0];
        sqlx::query("UPDATE configuration SET value = ?1 WHERE id = ?2")
            .bind(&value_txt)
            .bind(id)
            .execute(&mut *tx)
            .await
            .change_context(AppError::Unknown)
            .map_err(report_to_response)?;

        sqlx::query_as::<_, Configuration>("SELECT id, key, value FROM configuration WHERE id = ?1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .change_context(AppError::Unknown)
            .map_err(report_to_response)?
    } else {
        // 多行报错
        return Err(report_to_response(Report::new(AppError::Custom(
            format!("有多个空间配置同时存在 (key='config'): {} 行", ids.len()).to_string(),
        ))));
    };

    tx.commit()
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    // 提交后从 DB 重新加载配置
    let saved_config: Config = serde_json::from_str(&saved.value)
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    *config.write().unwrap() = saved_config;
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

    Ok(Json(streamer_infos))
}

pub async fn get_streamer_info_files(
    // Extension(streamers_service): Extension<DynUploadStreamersRepository>,
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<FileItem>>, Response> {
    let file_items = FileItem::select()
        .where_("streamer_info_id = ?")
        .bind(id)
        .fetch_all(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;

    Ok(Json(file_items))
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

pub async fn add_user_endpoint(
    State(pool): State<ConnectionPool>,
    Json(user): Json<InsertConfiguration>,
) -> Result<Json<Configuration>, Response> {
    let res = user
        .insert(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    Ok(Json(res))
}

pub async fn delete_user_endpoint(
    Path(id): Path<i64>,
    State(pool): State<ConnectionPool>,
) -> Result<Json<()>, Response> {
    let x = sqlx::query("DELETE FROM configuration WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    info!("{:?}", x);
    Ok(Json(()))
}

pub async fn get_qrcode() -> Result<Json<serde_json::Value>, Response> {
    let qrcode = Credential::new(None)
        .get_qrcode()
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    Ok(Json(qrcode))
}

pub async fn login_by_qrcode(
    Json(value): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, Response> {
    let info = tokio::time::timeout(
        Duration::from_secs(300),
        Credential::new(None).login_by_qrcode(value),
        // std::future::pending::<AppResult<LoginInfo>>(),
    )
    .await
    .change_context(AppError::Custom("deadline has elapsed".to_string()))
    .map_err(report_to_response)?
    .change_context(AppError::Unknown)
    .map_err(report_to_response)?;

    // extract mid
    let mid = info.token_info.mid;
    let filename = format!("data/{}.json", mid);

    let mut file = fs::File::create(&filename)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    file.write_all(&serde_json::to_vec_pretty(&info).unwrap())
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;

    Ok(Json(json!({ "filename": filename })))
}

pub async fn get_videos() -> Result<Json<Vec<serde_json::Value>>, Response> {
    let media_extensions = [".mp4", ".flv", ".3gp", ".webm", ".mkv", ".ts"];
    let blacklist = ["next-env.d.ts"];

    let mut file_list = Vec::new();
    let mut index = 1;

    // **use tokio::fs::read_dir**
    if let Ok(mut entries) = fs::read_dir(".").await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().into_owned();

            if blacklist.contains(&file_name.as_str()) {
                continue;
            }

            if let Some(ext) = path.extension().and_then(|e| e.to_str())
                && media_extensions
                    .iter()
                    .any(|allowed| ext == allowed.trim_start_matches('.'))
                && let Ok(metadata) = entry.metadata().await
            {
                let mtime = metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                file_list.push(serde_json::json!({
                    "key": index,
                    "name": file_name,
                    "updateTime": mtime,
                    "size": metadata.len(),
                }));
                index += 1;
            }
        }
    }
    Ok(Json(file_list))
}

// #[axum::debug_handler(state = ServiceRegister)]
pub async fn get_status(
    State(service_register): State<ServiceRegister>,
    State(workers): State<Arc<RwLock<Vec<Arc<Worker>>>>>,
    State(config): State<Arc<RwLock<Config>>>,
    State(actor_handle): State<Arc<ActorHandle>>,
) -> Result<Json<serde_json::Value>, Response> {
    let workers_clone = workers.read().unwrap().clone(); // 如果 Vec 本身可以克隆

    let mut sw = Vec::new();
    for worker in &workers_clone {
        sw.push(serde_json::json!({
            "downloader_status": format!("{:?}", worker.downloader_status.read().await),
            "uploader_status": format!("{:?}", worker.uploader_status.read().unwrap()),
            "live_streamer": worker.live_streamer,
            "upload_streamer": worker.upload_streamer,
        }));
    }

    Ok(Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "rooms": sw,
        "download_semaphore": actor_handle.d_kills.len(),
        "update_semaphore": actor_handle.u_kills.len(),
        "config": config,
    })))
}

#[derive(Deserialize)]
pub struct PostUploads {
    files: Vec<PathBuf>,
    params: UploadStreamer,
}

// #[debug_handler]
pub async fn post_uploads(
    State(config): State<Arc<RwLock<Config>>>,
    Json(json_data): Json<PostUploads>,
) -> Result<Json<serde_json::Value>, Response> {
    let upload_config = json_data.params;
    let (line, limit, submit_api) = {
        let config = config.read().unwrap();
        let line = UploadLine::from_str(&config.lines, true).ok();
        let limit = config.threads;
        let submit_api = config.submit_api.clone();
        (line, limit, submit_api)
    };
    info!("通过页面开始上传");
    let (bilibili, videos) = upload(
        upload_config
            .user_cookie
            .as_deref()
            .unwrap_or("cookies.json"),
        None,
        line,
        &json_data.files,
        limit as usize,
    )
    .await
    .map_err(report_to_response)?;
    if !videos.is_empty() {
        let recorder = Recorder::new(
            upload_config.title.clone(),
            StreamerInfo::new(
                &upload_config.template_name,
                "stream_title",
                "",
                Utc::now(),
                "",
            ),
            "",
        );
        let studio = build_studio(&upload_config, &bilibili, videos, recorder)
            .await
            .map_err(report_to_response)?;
        let response_data = submit_to_bilibili(&bilibili, &studio, submit_api.as_deref())
            .await
            .map_err(report_to_response)?;
        info!("通过页面上传成功 {:?}", response_data);
    }
    Ok(Json(serde_json::json!({})))
}
