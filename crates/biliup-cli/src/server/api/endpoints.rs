use crate::server::config::Config;
use crate::server::core::download_manager::ActorHandle;
use crate::server::errors::{AppError, report_to_response};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::Worker;
use crate::server::infrastructure::dto::LiveStreamerResponse;
use crate::server::infrastructure::models::{
    Configuration, FileItem, InsertConfiguration, InsertLiveStreamer, InsertUploadStreamer,
    LiveStreamer, StreamerInfo, UploadStreamer,
};
use crate::server::infrastructure::repositories::{
    del_streamer, get_all_streamer, get_upload_config,
};
use crate::server::infrastructure::service_register::ServiceRegister;
use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, debug_handler};
use biliup::credential::Credential;
use error_stack::{Report, ResultExt};
use ormlite::{Insert, Model};
use serde::Deserialize;
use serde_json::json;
use std::collections::VecDeque;
use std::io;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, UNIX_EPOCH};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{debug, error, info};

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
                        if worker.live_streamer.id == x.id {
                            Some(format!("{:?}", *worker.downloader_status.read().unwrap()))
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
        .add_room(&manager, live_streamers.clone(), upload_config)
        .await
        .map_err(report_to_response)?;

    info!(workers=?service_register.workers, "successfully inserted new live streamers");
    Ok(Json(live_streamers))
}
#[debug_handler(state = ServiceRegister)]
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
        .add_room(&manager, streamer.clone(), upload_config)
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
    info!(workers=?service_register.workers, "successfully inserted new live streamers");
    Ok(Json(live_streamers))
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

pub async fn get_status(
    State(service_register): State<ServiceRegister>,
    State(workers): State<Arc<RwLock<Vec<Arc<Worker>>>>>,
    State(config): State<Arc<RwLock<Config>>>,
    State(actor_handle): State<Arc<ActorHandle>>,
) -> Result<Json<serde_json::Value>, Response> {
    let mut sw = Vec::new();
    for worker in workers.read().unwrap().iter() {
        sw.push(serde_json::json!({
        "downloader_status": format!("{:?}", worker.downloader_status.read().unwrap()),
        "uploader_status": format!("{:?}", worker.uploader_status.read().unwrap()),
        "live_streamer": worker.live_streamer,
        "upload_streamer": worker.upload_streamer,
        }))
    }

    Ok(Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "rooms": sw,
        "download_semaphore": actor_handle.d_kills.len(),
        "update_semaphore": actor_handle.u_kills.len(),
        "config": config,
    })))
}

static ALLOWED_FILES: &[&str] = &["ds_update.log", "download.log", "upload.log"];

#[derive(Debug, Deserialize, Clone)]
pub struct LogsQuery {
    file: Option<String>,
}
pub async fn ws_logs(
    ws: WebSocketUpgrade,
    Query(query): Query<LogsQuery>,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| websocket_logs(socket, query))
}

async fn websocket_logs(mut ws: WebSocket, query: LogsQuery) {
    // 参数获取与校验
    let file_param = query.file.unwrap_or_else(|| "ds_update.log".to_string());
    if !ALLOWED_FILES.contains(&file_param.as_str()) {
        let _ = ws
            .send(Message::Text(
                format!("不允许访问请求的文件: {}", file_param).into(),
            ))
            .await;
        let _ = ws.send(Message::Close(None)).await;
        return;
    }

    let log_file = PathBuf::from(&file_param);

    // 发送初始内容（最后50行）并获取当前大小
    let mut file_size = match send_last_lines(&mut ws, &log_file, 50).await {
        Ok(size) => size,
        Err(e) => {
            match e.kind() {
                ErrorKind::NotFound => {
                    let _ = ws
                        .send(Message::Text(
                            format!("日志文件 {} 不存在", log_file.display()).into(),
                        ))
                        .await;
                }
                _ => {
                    let _ = ws
                        .send(Message::Text(format!("读取日志文件错误: {}", e).into()))
                        .await;
                    error!("读取日志文件错误: {}", e);
                }
            }
            let _ = ws.send(Message::Close(None)).await;
            return;
        }
    };

    // 心跳/轮询间隔
    let mut tick = interval(Duration::from_millis(500));
    tick.set_missed_tick_behavior(MissedTickBehavior::Skip);

    // 主循环：同时处理客户端消息和文件更新
    loop {
        tokio::select! {
            maybe_msg = ws.recv() => {
                match maybe_msg {
                    Some(Ok(Message::Close(_))) => {
                        let _ = ws.send(Message::Close(None)).await;
                        break;
                    }
                    Some(Ok(Message::Ping(payload))) => {
                        // 回应 PONG
                        let _ = ws.send(Message::Pong(payload)).await;
                    }
                    Some(Ok(_)) => {
                        // 其他消息不处理（Text/Binary等）
                    }
                    Some(Err(e)) => {
                        error!("WebSocket连接错误: {}", e);
                        break;
                    }
                    None => {
                        info!("WebSocket连接已关闭");
                        break;
                    }
                }
            }

            _ = tick.tick() => {
                // 文件是否存在
                let meta = match fs::metadata(&log_file).await {
                    Ok(m) => m,
                    Err(e) if e.kind() == ErrorKind::NotFound => {
                        let _ = ws.send(Message::Text(format!(
                            "日志文件 {} 不再存在",
                            log_file.display()
                        ).into())).await;
                        break;
                    }
                    Err(e) => {
                        let _ = ws.send(Message::Text(format!("监控日志文件错误: {}", e).into())).await;
                        error!("websocket_logs错误: {}", e);
                        break;
                    }
                };

                let current_size = meta.len();

                // 文件被截断
                if current_size < file_size {
                    let _ = ws.send(Message::Text(Utf8Bytes::from("日志文件被截断，重新加载...".to_string()))).await;
                    match send_last_lines(&mut ws, &log_file, 50).await {
                        Ok(size) => file_size = size,
                        Err(e) => {
                            let _ = ws.send(Message::Text(format!("读取日志文件错误: {}", e).into())).await;
                            error!("读取日志文件错误: {}", e);
                            break;
                        }
                    }
                    continue;
                }

                // 文件新增内容
                if current_size > file_size {
                    if let Err(e) = send_new_lines_from_offset(&mut ws, &log_file, file_size).await {
                        let _ = ws.send(Message::Text(format!("监控日志文件错误: {}", e).into())).await;
                        error!("websocket_logs错误: {}", e);
                        break;
                    }
                    file_size = current_size;
                }
            }
        }
    }

    let _ = ws.send(Message::Close(None)).await;
    debug!("WebSocket日志会话结束: {}", file_param);
}

// 发送最后 n 行，并返回当前文件大小
async fn send_last_lines(
    ws: &mut WebSocket,
    path: &std::path::Path,
    n: usize,
) -> std::io::Result<u64> {
    let meta = fs::metadata(path).await?;
    let file_size = meta.len();

    let file = fs::File::open(path).await?;
    let mut reader = BufReader::new(file);
    let mut lines = reader.lines();

    let mut buf: VecDeque<String> = VecDeque::with_capacity(n);
    while let Some(line) = lines.next_line().await? {
        if buf.len() == n {
            buf.pop_front();
        }
        buf.push_back(line);
    }
    for line in buf {
        ws.send(Message::Text(Utf8Bytes::from(line)))
            .await
            .map_err(|e| {
                std::io::Error::new(
                    ErrorKind::ConnectionAborted,
                    format!("发送WebSocket消息失败: {}", e),
                )
            })?;
    }
    Ok(file_size)
}

// 从偏移量开始读取新增内容，并逐行发送
async fn send_new_lines_from_offset(
    ws: &mut WebSocket,
    path: &std::path::Path,
    offset: u64,
) -> std::io::Result<()> {
    let mut file = fs::File::open(path).await?;
    file.seek(std::io::SeekFrom::Start(offset)).await?;

    // 直接读到字符串（UTF-8），若遇到非UTF-8可换成读bytes+lossy
    let mut s = String::new();
    if let Err(e) = file.read_to_string(&mut s).await {
        // 如果遇到非UTF-8数据，降级为 lossy
        let mut bytes = Vec::new();
        file.seek(std::io::SeekFrom::Start(offset)).await?;
        file.read_to_end(&mut bytes).await?;
        s = String::from_utf8_lossy(&bytes).into_owned();
        if e.kind() != ErrorKind::InvalidData {
            // 非编码错误也要汇报
            error!("读取日志文件新内容失败: {}", e);
        }
    }

    for line in s.lines() {
        ws.send(Message::Text(Utf8Bytes::from(line.to_string())))
            .await
            .map_err(|e| {
                std::io::Error::new(
                    ErrorKind::ConnectionAborted,
                    format!("发送WebSocket消息失败: {}", e),
                )
            })?;
    }
    Ok(())
}

async fn resolve_latest_log_path(
    dir: &std::path::Path,
    prefix: &str,
    suffix: &str,
) -> io::Result<PathBuf> {
    // 1) 活跃文件 prefix.log
    let active = dir.join(format!("{}.{}", prefix, suffix));
    if fs::metadata(&active).await.is_ok() {
        return Ok(active);
    }

    // 2) 回退到归档文件 prefix.*.log 中最新的一个
    let mut rd = fs::read_dir(dir).await?;
    let pre = format!("{}.", prefix);
    let suf = format!(".{}", suffix);

    let mut candidates: Vec<(String, PathBuf)> = Vec::new();
    while let Some(ent) = rd.next_entry().await? {
        let path = ent.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name.starts_with(&pre) && name.ends_with(&suf) {
            candidates.push((name.to_string(), path));
        }
    }

    if candidates.is_empty() {
        return Err(io::Error::new(ErrorKind::NotFound, "no log file found"));
    }

    // tracing-appender 的归档名使用 YYYY-MM-DD，中间按字符串排序即时间顺序
    candidates.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(candidates.last().unwrap().1.clone())
}
