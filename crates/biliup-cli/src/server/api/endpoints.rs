use crate::server::common::recording_policy::{self, Rejection};
use crate::server::common::upload::{build_studio, submit_to_bilibili, upload};
use crate::server::common::util::Recorder;
use crate::server::config::Config;
use crate::server::core::download_manager::DownloadManager;
use crate::server::errors::{AppError, report_to_response};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::{Stage, Worker, WorkerStatus};
use crate::server::infrastructure::dto::LiveStreamerResponse;
use crate::server::infrastructure::models::live_streamer::{InsertLiveStreamer, LiveStreamer};
use crate::server::infrastructure::models::upload_streamer::{
    InsertUploadStreamer, UploadStreamer,
};
use crate::server::infrastructure::models::{Configuration, FileItem, StreamerInfo};
use crate::server::infrastructure::repositories::{
    del_streamer, delete_bilibili_cookie, get_all_streamer, get_upload_config,
    register_bilibili_cookie,
};
use crate::server::infrastructure::service_register::ServiceRegister;
use crate::{LogHandle, UploadLine};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use biliup::credential::{Credential, save_login_info};
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
use tracing::info;
use tracing_subscriber::EnvFilter;

/// 主播当前被录制策略挡下的原因，供 `/v1/streamers` 覆盖状态字段。
///
/// 探测前就能判定的条件（录制时间范围）按当前时钟实时计算，不受监控循环节奏影响；
/// 需要房间标题才能判定的条件取最近一次探测的结论。
fn rejection_status(streamer: &LiveStreamer, worker: Option<&Worker>) -> Option<&'static str> {
    if let Some(rejection) = recording_policy::reject_before_probe(streamer) {
        return Some(rejection.status());
    }
    worker
        .and_then(Worker::rejection)
        .as_ref()
        .map(Rejection::status)
}

pub async fn get_streamers_endpoint(
    State(pool): State<ConnectionPool>,
    State(managers): State<Arc<DownloadManager>>,
) -> Result<Json<Vec<LiveStreamerResponse>>, Response> {
    let live_streamers = get_all_streamer(&pool).await.map_err(report_to_response)?;
    let mut results = Vec::new();
    let workers = managers.get_rooms().await;
    for x in live_streamers {
        let option = workers
            .clone()
            .into_iter()
            .find(|worker| worker.live_streamer.id == x.id);

        let status = match option.as_ref() {
            Some(t) => format!("{:?}", *t.downloader_status.read().unwrap()),
            None => String::new(),
        };
        // 被录制策略挡下时报告具体原因，否则「不在时间范围内」和「单纯没开播」
        // 在界面上都是「空闲」，用户无从判断配置有没有生效。
        // 时间类条件按当前时钟实时算，不依赖监控循环轮到这个房间，因此不会过期；
        // 依赖房间标题的结论来自最近一次探测。正在录制时不覆盖。
        let status = match rejection_status(&x, option.as_deref()) {
            Some(reason) if status != "Working" => reason.to_string(),
            _ => status,
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
    State(managers): State<Arc<DownloadManager>>,
    State(pool): State<ConnectionPool>,
    Json(payload): Json<InsertLiveStreamer>,
) -> Result<Json<LiveStreamer>, Response> {
    let url = &payload.url.clone();
    // You can insert the model directly.
    let live_streamers = payload
        .insert(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    let upload_config = get_upload_config(&pool, live_streamers.id)
        .await
        .map_err(report_to_response)?;
    let Some(_) = managers
        .add_room(service_register.worker(live_streamers.clone(), upload_config))
        .await
    else {
        info!("not supported url: {}", url);
        return Err((StatusCode::BAD_REQUEST, "Not supported url").into_response());
    };

    info!(url = url, "successfully inserted new live streamers");
    Ok(Json(live_streamers))
}

pub async fn put_streamers_endpoint(
    State(service_register): State<ServiceRegister>,
    State(managers): State<Arc<DownloadManager>>,
    State(pool): State<ConnectionPool>,
    Json(payload): Json<LiveStreamer>,
) -> Result<Json<LiveStreamer>, Response> {
    let streamer = payload
        .update_all_fields(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;

    let id = streamer.id;
    managers.del_room(id).await;

    let upload_config = get_upload_config(&pool, id)
        .await
        .map_err(report_to_response)?;

    managers
        .add_room(service_register.worker(streamer.clone(), upload_config))
        .await
        .ok_or(AppError::Unknown)
        .map_err(report_to_response)?;

    info!(id = id, "successfully update live streamers");
    Ok(Json(streamer))
}

pub async fn delete_streamers_endpoint(
    State(managers): State<Arc<DownloadManager>>,
    State(pool): State<ConnectionPool>,
    Path(id): Path<i64>,
) -> Result<Json<LiveStreamer>, Response> {
    managers.del_room(id).await;

    let live_streamers = del_streamer(&pool, id).await.map_err(report_to_response)?;
    info!(workers=?live_streamers, "successfully inserted new live streamers");
    Ok(Json(live_streamers))
}

// #[axum::debug_handler(state = ServiceRegister)]
pub async fn pause_streamers_endpoint(
    State(managers): State<Arc<DownloadManager>>,
    Path(id): Path<i64>,
) -> Result<Json<()>, Response> {
    let worker = managers.get_room_by_id(id).await;
    if let Some(w) = worker {
        let worker_status = w.downloader_status.read().unwrap().clone();
        match worker_status {
            WorkerStatus::Working(_) => {
                w.change_status(Stage::Download, WorkerStatus::Pause).await;
                info!(url=?&w.live_streamer.url, "successfully pause live streamers");
                managers.make_waker(id).await;
            }
            WorkerStatus::Pause => {
                w.change_status(Stage::Download, WorkerStatus::Idle).await;
                managers.wake_waker(id).await;
                info!(url=?&w.live_streamer.url, "successfully start live streamers");
            }
            WorkerStatus::Pending => {
                w.change_status(Stage::Download, WorkerStatus::Pause).await;
                managers.make_waker(id).await;
                info!(url=?&w.live_streamer.url, "successfully pause live streamers");
            }
            WorkerStatus::Idle => {
                w.change_status(Stage::Download, WorkerStatus::Pause).await;
                managers.make_waker(id).await;
                info!(url=?&w.live_streamer.url, "successfully pause live streamers");
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
    State(log_handle): State<LogHandle>,
    Json(json_data): Json<Config>,
) -> Result<Json<Config>, Response> {
    let mut json_data = json_data;
    json_data.normalize_segment_limits();
    json_data
        .validate_segment_limits()
        .map_err(report_to_response)?;
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
    let mut saved_config: Config = serde_json::from_str(&saved.value)
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    saved_config.normalize_segment_limits();
    saved_config
        .validate_segment_limits()
        .map_err(report_to_response)?;
    *config.write().unwrap() = saved_config;
    let guard = config.read().unwrap();
    if let Some(loggers_level) = &guard.loggers_level {
        let new_filter = EnvFilter::try_new(loggers_level)
            .change_context(AppError::Custom(String::from("Invalid log level format")))
            .map_err(report_to_response)?;

        log_handle
            .modify(|filter| *filter = new_filter)
            .change_context(AppError::Unknown)
            .map_err(report_to_response)?;
    }

    Ok(Json(guard.clone()))
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
    Json(user): Json<AddBilibiliUser>,
) -> Result<Json<Configuration>, Response> {
    let res = register_bilibili_cookie(&pool, &user.value)
        .await
        .map_err(report_to_response)?
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "凭据文件不存在、不是普通文件或无法访问",
            )
                .into_response()
        })?;
    Ok(Json(res))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddBilibiliUser {
    value: PathBuf,
}

#[cfg(test)]
mod user_payload_tests {
    use super::{AddBilibiliUser, PostUploads};
    use std::path::Path;

    #[test]
    fn add_user_payload_rejects_a_client_supplied_configuration_key() {
        assert!(
            serde_json::from_value::<AddBilibiliUser>(serde_json::json!({
                "key": "config",
                "value": "cookies.json"
            }))
            .is_err()
        );
        let payload: AddBilibiliUser = serde_json::from_value(serde_json::json!({
            "value": "cookies.json"
        }))
        .unwrap();
        assert_eq!(payload.value, Path::new("cookies.json"));
    }

    #[test]
    fn page_upload_requires_a_server_side_template_id() {
        assert!(
            serde_json::from_value::<PostUploads>(serde_json::json!({
                "files": ["video.mp4"],
                "params": {
                    "id": 99,
                    "user_cookie": "/tmp/client-controlled.json"
                }
            }))
            .is_err()
        );

        let payload: PostUploads = serde_json::from_value(serde_json::json!({
            "files": ["video.mp4"],
            "template_id": 7
        }))
        .unwrap();
        assert_eq!(payload.template_id, 7);
        assert_eq!(payload.files, ["video.mp4"]);
    }
}

pub async fn delete_user_endpoint(
    Path(id): Path<i64>,
    State(pool): State<ConnectionPool>,
) -> Result<Json<crate::server::infrastructure::repositories::DeletedBilibiliCookie>, Response> {
    let deleted = delete_bilibili_cookie(&pool, id)
        .await
        .map_err(report_to_response)?;
    info!(
        user_id = deleted.id,
        file_deleted = deleted.file_deleted,
        references_remaining = deleted.references_remaining,
        "deleted Bilibili user registration"
    );
    Ok(Json(deleted))
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

    save_login_info(&filename, &info)
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
    State(_service_register): State<ServiceRegister>,
    State(managers): State<Arc<DownloadManager>>,
    State(config): State<Arc<RwLock<Config>>>,
) -> Result<Json<serde_json::Value>, Response> {
    let workers = managers.get_rooms().await;

    let mut sw = Vec::new();
    for worker in &workers {
        sw.push(serde_json::json!({
            "downloader_status": format!("{:?}", worker.downloader_status.read()),
            "uploader_status": format!("{:?}", worker.uploader_status.read().unwrap()),
            "live_streamer": worker.live_streamer,
            "upload_streamer": worker.upload_streamer,
        }));
    }

    Ok(Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "rooms": sw,
        "download_semaphore": managers.download_semaphore,
        "update_semaphore": managers.u_kills.len(),
        "config": config,
    })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostUploads {
    files: Vec<String>,
    template_id: i64,
}

// #[debug_handler]
pub async fn post_uploads(
    State(config): State<Arc<RwLock<Config>>>,
    State(pool): State<ConnectionPool>,
    Json(json_data): Json<PostUploads>,
) -> Result<Json<serde_json::Value>, Response> {
    let upload_config = UploadStreamer::select()
        .where_("id = ?")
        .bind(json_data.template_id)
        .fetch_optional(&pool)
        .await
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "上传模板不存在").into_response())?;
    if upload_config.is_noop_uploader() {
        info!(
            uploader = ?upload_config.uploader,
            "Skipping page upload because uploader is Noop"
        );
        return Ok(Json(json!({})));
    }

    let (line, limit, submit_api) = {
        let config = config.read().unwrap();
        let line = UploadLine::from_str(&config.lines, true).ok();
        let limit = config.threads;
        let submit_api = config.submit_api.clone();
        (line, limit, submit_api)
    };
    let root = std::env::current_dir()
        .change_context(AppError::Unknown)
        .map_err(report_to_response)?;
    let files = json_data
        .files
        .iter()
        .map(|file| {
            crate::server::router::resolve_media_path(&root, file).map_err(|_| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("无效或越界的媒体文件名: {file}"),
                )
                    .into_response()
            })
        })
        .collect::<Result<Vec<_>, Response>>()?;
    if files.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "至少选择一个媒体文件").into_response());
    }
    info!("通过页面开始上传");
    tokio::spawn(async move {
        let result = async {
            let (bilibili, videos) = upload(
                upload_config
                    .user_cookie
                    .as_deref()
                    .unwrap_or("cookies.json"),
                None,
                line,
                &files,
                limit as usize,
            )
            .await?;
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
                );
                let studio = build_studio(&upload_config, &bilibili, videos, &recorder).await?;
                submit_to_bilibili(&bilibili, &studio, submit_api.as_deref()).await?;
                info!(template_id = upload_config.id, "通过页面上传成功");
            }
            Ok::<_, Report<AppError>>(())
        }
        .await;
        if result.is_err() {
            // Upload failures can wrap upstream response bodies containing
            // short-lived upload authorization. Keep persistent Web logs free
            // of those response bodies.
            tracing::error!(template_id = upload_config.id, "页面上传失败");
        }
    });

    Ok(Json(serde_json::json!({})))
}

#[cfg(test)]
mod recording_policy_status_tests {
    use super::*;
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use chrono::{Duration as ChronoDuration, SecondsFormat};
    use serde_json::Value;

    /// 以当前时刻为基准造窗口，形态与前端 `Date.toISOString()` 写出的一致
    fn window(starts_in: i64, ends_in: i64) -> String {
        let now = Utc::now();
        let iso = |offset: i64| {
            (now + ChronoDuration::seconds(offset)).to_rfc3339_opts(SecondsFormat::Millis, true)
        };
        format!(r#"["{}","{}"]"#, iso(starts_in), iso(ends_in))
    }

    fn insert(url: &str, time_range: Option<String>) -> InsertLiveStreamer {
        InsertLiveStreamer {
            url: url.to_string(),
            remark: url.to_string(),
            filename_prefix: None,
            time_range,
            upload_streamers_id: None,
            format: None,
            override_cfg: None,
            preprocessor: None,
            segment_processor: None,
            downloaded_processor: None,
            postprocessor: None,
            opt_args: None,
            excluded_keywords: None,
        }
    }

    fn worker_for(streamer: LiveStreamer) -> Worker {
        Worker::new(
            streamer,
            None,
            Arc::new(RwLock::new(Config::default())),
            Default::default(),
        )
    }

    /// 走完整的 数据库 -> /v1/streamers 处理函数 -> JSON 这条路
    #[tokio::test]
    async fn the_streamers_endpoint_reports_why_a_streamer_is_not_recording() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();

        // 窗口尚未开始 -> 应报告 OutOfSchedule
        insert("https://live.example.com/closed", Some(window(3600, 7200)))
            .insert(&pool)
            .await
            .unwrap();
        // 窗口正开着 -> 不该被覆盖
        insert("https://live.example.com/open", Some(window(-60, 3600)))
            .insert(&pool)
            .await
            .unwrap();
        // 没配时间范围 -> 不该被覆盖
        insert("https://live.example.com/always", None)
            .insert(&pool)
            .await
            .unwrap();

        let managers = Arc::new(DownloadManager::new(1, 0, pool.clone()));
        let Json(responses) = get_streamers_endpoint(State(pool), State(managers))
            .await
            .expect("接口应返回成功");

        let status_of = |url: &str| {
            responses
                .iter()
                .find(|r| r.inner.url == url)
                .unwrap_or_else(|| panic!("响应里应有 {url}"))
                .status
                .clone()
        };

        assert_eq!(
            status_of("https://live.example.com/closed"),
            "OutOfSchedule"
        );
        assert_eq!(status_of("https://live.example.com/open"), "");
        assert_eq!(status_of("https://live.example.com/always"), "");
    }

    #[tokio::test]
    async fn the_endpoint_response_serialises_the_status_frontend_switches_on() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        insert("https://live.example.com/closed", Some(window(3600, 7200)))
            .insert(&pool)
            .await
            .unwrap();

        let managers = Arc::new(DownloadManager::new(1, 0, pool.clone()));
        let Json(responses) = get_streamers_endpoint(State(pool), State(managers))
            .await
            .unwrap();

        // 前端 app/(app)/streamers/page.tsx 就是对这个字符串做 switch
        let json: Value = serde_json::to_value(&responses[0]).unwrap();
        assert_eq!(json["status"], "OutOfSchedule");
    }

    #[test]
    fn a_title_excluded_worker_reports_title_excluded() {
        let streamer = LiveStreamer {
            id: 1,
            url: "https://live.example.com/x".to_string(),
            remark: "x".to_string(),
            filename_prefix: None,
            time_range: None,
            upload_streamers_id: None,
            format: None,
            override_cfg: None,
            preprocessor: None,
            segment_processor: None,
            downloaded_processor: None,
            postprocessor: None,
            opt_args: None,
            excluded_keywords: None,
        };
        let worker = worker_for(streamer.clone());

        // 没有任何拦截时不覆盖状态
        assert_eq!(rejection_status(&streamer, Some(&worker)), None);

        // 探测后记录下的标题拦截会被报告出来
        worker.set_rejection(Some(Rejection::ExcludedKeyword("录像".to_string())));
        assert_eq!(
            rejection_status(&streamer, Some(&worker)),
            Some("TitleExcluded")
        );

        // 探测结果推翻后清空
        worker.set_rejection(None);
        assert_eq!(rejection_status(&streamer, Some(&worker)), None);
    }

    #[test]
    fn the_time_window_outranks_a_stale_title_rejection() {
        let streamer = LiveStreamer {
            id: 1,
            url: "https://live.example.com/x".to_string(),
            remark: "x".to_string(),
            filename_prefix: None,
            time_range: Some(window(3600, 7200)),
            upload_streamers_id: None,
            format: None,
            override_cfg: None,
            preprocessor: None,
            segment_processor: None,
            downloaded_processor: None,
            postprocessor: None,
            opt_args: None,
            excluded_keywords: None,
        };
        let worker = worker_for(streamer.clone());
        worker.set_rejection(Some(Rejection::ExcludedKeyword("录像".to_string())));

        // 时间范围按当前时钟实时算、不会过期，优先于上一次探测留下的结论
        assert_eq!(
            rejection_status(&streamer, Some(&worker)),
            Some("OutOfSchedule")
        );
    }
}
