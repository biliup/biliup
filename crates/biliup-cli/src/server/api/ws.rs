use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use axum::extract::{Query, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use std::collections::VecDeque;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, BufReader};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{debug, error, info};

static ALLOWED_FILES: &[&str] = &["ds_update.log", "download.log", "upload.log"];
const MAX_LOG_CONNECTIONS: usize = 8;
static LOG_CONNECTIONS: OnceLock<Arc<Semaphore>> = OnceLock::new();

#[derive(Debug, Deserialize, Clone)]
pub struct LogsQuery {
    file: Option<String>,
}

pub async fn ws_logs(
    ws: WebSocketUpgrade,
    Query(query): Query<LogsQuery>,
    headers: HeaderMap,
) -> Response {
    if !websocket_origin_allowed(&headers) {
        return (StatusCode::FORBIDDEN, "WebSocket Origin 不受信任").into_response();
    }
    let limiter = LOG_CONNECTIONS
        .get_or_init(|| Arc::new(Semaphore::new(MAX_LOG_CONNECTIONS)))
        .clone();
    let Some(permit) = acquire_log_permit(limiter) else {
        return (StatusCode::TOO_MANY_REQUESTS, "日志连接数已达上限").into_response();
    };

    ws.on_upgrade(move |socket| async move {
        let _permit = permit;
        websocket_logs(socket, query).await;
    })
}

fn acquire_log_permit(limiter: Arc<Semaphore>) -> Option<OwnedSemaphorePermit> {
    limiter.try_acquire_owned().ok()
}

fn websocket_origin_allowed(headers: &HeaderMap) -> bool {
    let Some(origin) = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let Ok(origin) = url::Url::parse(origin) else {
        return false;
    };
    if !matches!(origin.scheme(), "http" | "https") {
        return false;
    }
    if origin.username() != ""
        || origin.password().is_some()
        || origin.path() != "/"
        || origin.query().is_some()
        || origin.fragment().is_some()
    {
        return false;
    }
    let origin_authority = &origin[url::Position::BeforeHost..url::Position::AfterPort];
    let host_matches = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|host| host.eq_ignore_ascii_case(origin_authority));
    let trusted_dev_origin = matches!(
        origin.as_str().trim_end_matches('/'),
        "http://localhost:3000" | "http://127.0.0.1:3000" | "http://[::1]:3000"
    );
    host_matches || trusted_dev_origin
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
    let reader = BufReader::new(file);
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

#[cfg(test)]
mod tests {
    use super::{MAX_LOG_CONNECTIONS, acquire_log_permit, websocket_origin_allowed};
    use axum::http::{HeaderMap, HeaderValue, header};
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    fn headers(host: &str, origin: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_str(host).unwrap());
        if let Some(origin) = origin {
            headers.insert(header::ORIGIN, HeaderValue::from_str(origin).unwrap());
        }
        headers
    }

    #[test]
    fn websocket_origin_must_match_host_or_trusted_dev_frontend() {
        assert!(websocket_origin_allowed(&headers(
            "example.test",
            Some("https://example.test")
        )));
        assert!(websocket_origin_allowed(&headers(
            "127.0.0.1:19159",
            Some("http://localhost:3000")
        )));
        assert!(!websocket_origin_allowed(&headers(
            "127.0.0.1:19159",
            Some("https://attacker.example")
        )));
        assert!(!websocket_origin_allowed(&headers(
            "example.test",
            Some("https://example.test/not-an-origin")
        )));
        assert!(!websocket_origin_allowed(&headers(
            "example.test",
            Some("https://user@example.test")
        )));
        assert!(!websocket_origin_allowed(&headers("127.0.0.1:19159", None)));
    }

    #[test]
    fn websocket_log_connections_are_bounded() {
        let limiter = Arc::new(Semaphore::new(MAX_LOG_CONNECTIONS));
        let permits: Vec<_> = (0..MAX_LOG_CONNECTIONS)
            .map(|_| acquire_log_permit(limiter.clone()).unwrap())
            .collect();
        assert!(acquire_log_permit(limiter.clone()).is_none());
        drop(permits);
        assert!(acquire_log_permit(limiter).is_some());
    }
}
