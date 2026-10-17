//! File location uses existing recording names or render IDs, never arbitrary
//! paths supplied by the browser. GUI actions are limited to direct local peers.
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::router::{StaticPathError, resolve_media_path};
use crate::server::workbench::renders::RenderJobs;
use axum::Json;
use axum::extract::{ConnectInfo, Extension, Query, State};
use axum::http::{HeaderMap, StatusCode, header, uri::Authority};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRequest {
    pub kind: String,
    pub file_name: Option<String>,
    pub id: Option<i64>,
}

#[derive(Serialize)]
struct FileLocation {
    path: String,
    can_open: bool,
}

fn error(status: StatusCode, message: &'static str) -> Response {
    (status, message).into_response()
}

async fn resolve_file(
    pool: &ConnectionPool,
    render_root: &Path,
    cwd: &Path,
    file: &FileRequest,
) -> Result<PathBuf, Response> {
    match file.kind.as_str() {
        "recording" if file.id.is_none() => {
            let name = file
                .file_name
                .as_deref()
                .ok_or_else(|| error(StatusCode::BAD_REQUEST, "缺少录制文件名"))?;
            resolve_media_path(cwd, name).map_err(|e| match e {
                StaticPathError::Invalid => error(StatusCode::BAD_REQUEST, "录制文件名或位置无效"),
                StaticPathError::NotFound => error(StatusCode::NOT_FOUND, "录制文件不存在"),
            })
        }
        "render" if file.file_name.is_none() && file.id.is_some_and(|id| id > 0) => {
            let record: Option<(String, Option<String>)> =
                sqlx::query_as("SELECT state,output_path FROM render_jobs WHERE id=?")
                    .bind(file.id)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| {
                        tracing::warn!(%e,"读取压制文件位置失败");
                        error(StatusCode::INTERNAL_SERVER_ERROR, "读取压制任务失败")
                    })?;
            let (state, path) =
                record.ok_or_else(|| error(StatusCode::NOT_FOUND, "压制任务不存在"))?;
            if state != "ready" {
                return Err(error(StatusCode::CONFLICT, "压制任务尚未完成"));
            }
            let path = path.ok_or_else(|| error(StatusCode::NOT_FOUND, "压制产物不存在"))?;
            let cwd = tokio::fs::canonicalize(cwd)
                .await
                .map_err(|_| error(StatusCode::NOT_FOUND, "工作目录不存在"))?;
            let root = tokio::fs::canonicalize(cwd.join(render_root))
                .await
                .map_err(|_| error(StatusCode::NOT_FOUND, "压制产物目录不存在"))?;
            let path = tokio::fs::canonicalize(cwd.join(path))
                .await
                .map_err(|_| error(StatusCode::NOT_FOUND, "压制产物不存在"))?;
            if !root.starts_with(&cwd)
                || !path.starts_with(&root)
                || !tokio::fs::metadata(&path).await.is_ok_and(|m| m.is_file())
            {
                return Err(error(StatusCode::BAD_REQUEST, "压制产物位置无效"));
            }
            Ok(path)
        }
        _ => Err(error(StatusCode::BAD_REQUEST, "文件类型或标识无效")),
    }
}

fn loopback(ip: IpAddr) -> bool {
    ip.is_loopback()
        || matches!(ip, IpAddr::V6(v6) if v6.to_ipv4_mapped().is_some_and(|v4| v4.is_loopback()))
}

fn direct_local(peer: Option<SocketAddr>, headers: &HeaderMap) -> bool {
    if !peer.is_some_and(|peer| loopback(peer.ip())) {
        return false;
    }
    // A loopback reverse proxy still represents a remote browser. Do not trust
    // forwarded addresses or permit it to open a file manager on the server.
    if [
        "forwarded",
        "x-forwarded-for",
        "x-forwarded-host",
        "x-real-ip",
    ]
    .iter()
    .any(|name| headers.contains_key(*name))
    {
        return false;
    }
    let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    if host.contains('@') {
        return false;
    }
    let Ok(authority) = host.parse::<Authority>() else {
        return false;
    };
    let host = authority
        .host()
        .trim_start_matches('[')
        .trim_end_matches(']');
    host.eq_ignore_ascii_case("localhost") || host.parse::<IpAddr>().is_ok_and(loopback)
}

#[cfg(target_os = "linux")]
fn command_available(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(dir.join(name))
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
    })
}

fn desktop_available() -> bool {
    #[cfg(target_os = "linux")]
    {
        let display = ["DISPLAY", "WAYLAND_DISPLAY"]
            .iter()
            .any(|key| std::env::var_os(key).is_some_and(|v| !v.is_empty()));
        return display
            && (command_available("xdg-open")
                || (std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some_and(|v| !v.is_empty())
                    && command_available("gdbus")));
    }
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("SSH_CONNECTION").is_none();
    }
    #[cfg(target_os = "windows")]
    {
        // Services run in session 0 and cannot display a file manager in the
        // signed-in user's desktop, even when their client connects locally.
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetCurrentProcessId() -> u32;
            fn ProcessIdToSessionId(process_id: u32, session_id: *mut u32) -> i32;
        }
        let mut session_id = 0;
        return unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session_id) != 0 }
            && session_id > 0;
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

async fn run(command: &mut tokio::process::Command) -> Result<(), String> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let status = tokio::time::timeout(Duration::from_secs(3), command.status())
        .await
        .map_err(|_| "文件管理器未响应".to_owned())?
        .map_err(|_| "无法启动文件管理器".to_owned())?;
    if !status.success() {
        return Err("文件管理器未能打开该文件".to_owned());
    }
    Ok(())
}

async fn open_native(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        if command_available("gdbus") && std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some() {
            let uri = url::Url::from_file_path(path).map_err(|_| "文件地址无效".to_owned())?;
            let items =
                serde_json::to_string(&[uri.as_str()]).map_err(|_| "文件地址无效".to_owned())?;
            let result = run(tokio::process::Command::new("gdbus").args([
                "call",
                "--session",
                "--dest",
                "org.freedesktop.FileManager1",
                "--object-path",
                "/org/freedesktop/FileManager1",
                "--method",
                "org.freedesktop.FileManager1.ShowItems",
                &items,
                "",
            ]))
            .await;
            if result.is_ok() {
                return result;
            }
        }
        return run(tokio::process::Command::new("xdg-open")
            .arg(path.parent().ok_or_else(|| "文件目录无效".to_owned())?))
        .await;
    }
    #[cfg(target_os = "macos")]
    {
        return run(tokio::process::Command::new("open").arg("-R").arg(path)).await;
    }
    #[cfg(target_os = "windows")]
    {
        return run(
            tokio::process::Command::new("explorer.exe").arg(format!("/select,{}", path.display()))
        )
        .await;
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = path;
        Err("当前系统没有可用的文件管理器".to_owned())
    }
}

pub async fn location(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    Query(file): Query<FileRequest>,
) -> Response {
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(_) => return error(StatusCode::INTERNAL_SERVER_ERROR, "工作目录不可用"),
    };
    match resolve_file(&pool, jobs.root(), &cwd, &file).await {
        Ok(path) => Json(FileLocation {
            path: path.to_string_lossy().into_owned(),
            can_open: direct_local(peer.map(|p| p.0.0), &headers) && desktop_available(),
        })
        .into_response(),
        Err(response) => response,
    }
}

pub async fn reveal(
    State(pool): State<ConnectionPool>,
    State(jobs): State<Arc<RenderJobs>>,
    peer: Option<Extension<ConnectInfo<SocketAddr>>>,
    headers: HeaderMap,
    Json(file): Json<FileRequest>,
) -> Response {
    if !direct_local(peer.map(|p| p.0.0), &headers) {
        return error(StatusCode::FORBIDDEN, "远程访问请复制服务器文件路径");
    }
    if !desktop_available() {
        return error(StatusCode::CONFLICT, "服务没有可用桌面，请复制文件路径");
    }
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(_) => return error(StatusCode::INTERNAL_SERVER_ERROR, "工作目录不可用"),
    };
    let path = match resolve_file(&pool, jobs.root(), &cwd, &file).await {
        Ok(path) => path,
        Err(response) => return response,
    };
    match open_native(&path).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(message) => {
            (StatusCode::CONFLICT, format!("{message}，请复制文件路径")).into_response()
        }
    }
}

#[cfg(test)]
mod tests;
