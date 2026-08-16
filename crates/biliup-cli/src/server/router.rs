use crate::server::api::bilibili_endpoints::{
    archive_pre_endpoint, get_user_archives_endpoint, get_user_profile_endpoint,
};
use crate::server::api::endpoints::{
    add_upload_streamer_endpoint, add_user_endpoint, delete_streamers_endpoint,
    delete_template_endpoint, delete_user_endpoint, get_configuration, get_qrcode, get_status,
    get_streamer_info, get_streamer_info_files, get_streamers_endpoint,
    get_upload_streamer_endpoint, get_upload_streamers_endpoint, get_users_endpoint, get_videos,
    login_by_qrcode, pause_streamers_endpoint, post_streamers_endpoint, post_uploads,
    put_configuration, put_streamers_endpoint,
};
use crate::server::infrastructure::service_register::ServiceRegister;
use axum::Router;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use tower::ServiceExt;
use tower_http::services::ServeFile;

const ALLOWED_MEDIA_EXTENSIONS: &[&str] = &["mp4", "flv", "3gp", "webm", "mkv", "ts"];
const ALLOWED_LOG_FILES: &[&str] = &["ds_update.log", "download.log", "upload.log"];
/// 创建应用程序路由
pub fn router(service_register: ServiceRegister) -> Router<()> {
    Router::new()
        // 主播管理相关路由
        .route(
            "/v1/streamers",
            get(get_streamers_endpoint) // 获取主播列表
                .post(post_streamers_endpoint) // 添加主播
                .put(put_streamers_endpoint), // 更新主播
        )
        .route("/v1/streamers/{id}", delete(delete_streamers_endpoint)) // 删除主播
        .route("/v1/streamers/{id}/pause", put(pause_streamers_endpoint))
        // 配置管理路由
        .route(
            "/v1/configuration",
            get(get_configuration).put(put_configuration), // 获取/更新配置
        )
        // 主播信息路由
        .route("/v1/streamer-info", get(get_streamer_info)) // 获取主播信息
        .route("/v1/streamer-info/files/{id}", get(get_streamer_info_files)) // 获取主播信息
        // 上传模板管理路由
        .route("/v1/upload/streamers", get(get_upload_streamers_endpoint)) // 获取上传模板列表
        .route(
            "/v1/upload/streamers/{id}",
            delete(delete_template_endpoint) // 删除上传模板
                .get(get_upload_streamer_endpoint), // 获取单个上传模板
        )
        .route("/v1/upload/streamers", post(add_upload_streamer_endpoint)) // 添加上传模板
        // 用户管理路由
        .route("/v1/users", get(get_users_endpoint).post(add_user_endpoint)) // 获取用户列表/添加用户
        .route(
            "/v1/users/{id}",
            get(get_user_profile_endpoint).delete(delete_user_endpoint),
        )
        .route("/v1/users/{id}/archives", get(get_user_archives_endpoint))
        // B站API代理路由
        .route("/bili/archive/pre", get(archive_pre_endpoint)) // 投稿预处理
        // 认证相关路由
        .route("/v1/get_qrcode", get(get_qrcode)) // 获取二维码
        .route("/v1/login_by_qrcode", post(login_by_qrcode)) // 二维码登录
        // 视频文件管理路由
        .route("/v1/videos", get(get_videos)) // 获取视频列表
        .route("/v1/status", get(get_status))
        .route("/v1/uploads", post(post_uploads))
        .route("/static/{path}", get(using_serve_file_from_a_route))
        .with_state(service_register) // 注入服务注册器状态
}

async fn using_serve_file_from_a_route(
    axum::extract::Path(path): axum::extract::Path<String>,
    request: Request<Body>,
) -> Response {
    let root = match std::env::current_dir() {
        Ok(root) => root,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let path = match resolve_static_path(&root, &path) {
        Ok(path) => path,
        Err(StaticPathError::Invalid) => return StatusCode::BAD_REQUEST.into_response(),
        Err(StaticPathError::NotFound) => return StatusCode::NOT_FOUND.into_response(),
    };

    ServeFile::new(path).oneshot(request).await.into_response()
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StaticPathError {
    Invalid,
    NotFound,
}

fn resolve_static_path(
    root: &std::path::Path,
    requested: &str,
) -> Result<std::path::PathBuf, StaticPathError> {
    use std::path::{Component, Path};

    // The endpoint intentionally accepts an opaque basename, not a filesystem
    // path.  Reject both separator styles so the same rule holds on every OS.
    if requested.is_empty() || requested.contains('/') || requested.contains('\\') {
        return Err(StaticPathError::Invalid);
    }
    let requested_path = Path::new(requested);
    let mut components = requested_path.components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err(StaticPathError::Invalid);
    }

    if !ALLOWED_LOG_FILES.contains(&requested) {
        return resolve_media_path(root, requested);
    }

    let root = root.canonicalize().map_err(|_| StaticPathError::NotFound)?;
    let candidate = root.join(requested);
    let canonical = candidate
        .canonicalize()
        .map_err(|_| StaticPathError::NotFound)?;
    if !canonical.starts_with(&root) || !canonical.is_file() {
        return Err(StaticPathError::Invalid);
    }
    Ok(canonical)
}

pub(crate) fn resolve_media_path(
    root: &std::path::Path,
    requested: &str,
) -> Result<std::path::PathBuf, StaticPathError> {
    use std::path::{Component, Path};

    if requested.is_empty() || requested.contains('/') || requested.contains('\\') {
        return Err(StaticPathError::Invalid);
    }
    let requested_path = Path::new(requested);
    let mut components = requested_path.components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err(StaticPathError::Invalid);
    }
    let allowed = requested_path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ALLOWED_MEDIA_EXTENSIONS
                .iter()
                .any(|allowed| extension.eq_ignore_ascii_case(allowed))
        });
    if !allowed {
        return Err(StaticPathError::Invalid);
    }

    let root = root.canonicalize().map_err(|_| StaticPathError::NotFound)?;
    let canonical = root
        .join(requested)
        .canonicalize()
        .map_err(|_| StaticPathError::NotFound)?;
    if !canonical.starts_with(&root) || !canonical.is_file() {
        return Err(StaticPathError::Invalid);
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::{StaticPathError, resolve_static_path};
    use std::fs;

    #[test]
    fn static_files_are_limited_to_media_and_known_logs() {
        let root = tempfile::tempdir().unwrap();
        let video = root.path().join("recording.mp4");
        let log = root.path().join("ds_update.log");
        let cookie = root.path().join("cookies.json");
        fs::write(&video, b"video").unwrap();
        fs::write(&log, b"log").unwrap();
        fs::write(&cookie, b"secret").unwrap();

        assert_eq!(
            resolve_static_path(root.path(), "recording.mp4").unwrap(),
            video
        );
        assert_eq!(
            resolve_static_path(root.path(), "ds_update.log").unwrap(),
            log
        );
        assert_eq!(
            resolve_static_path(root.path(), "cookies.json"),
            Err(StaticPathError::Invalid)
        );
    }

    #[test]
    fn static_files_reject_absolute_and_traversal_paths() {
        let root = tempfile::tempdir().unwrap();

        for path in [
            "../secret.mp4",
            "/etc/passwd.mp4",
            "nested/video.mp4",
            "..\\secret.mp4",
        ] {
            assert_eq!(
                resolve_static_path(root.path(), path),
                Err(StaticPathError::Invalid)
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn static_files_reject_symlinks_that_escape_the_root() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        let link = root.path().join("escape.mp4");
        symlink(outside.path(), &link).unwrap();

        assert_eq!(
            resolve_static_path(root.path(), "escape.mp4"),
            Err(StaticPathError::Invalid)
        );
    }
}
