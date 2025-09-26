use axum::extract::ws::{Message, Utf8Bytes, WebSocket};
use axum::extract::{Query, WebSocketUpgrade};
use serde::Deserialize;
use std::collections::VecDeque;
use std::io;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, BufReader};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{debug, error, info};

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
