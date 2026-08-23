use axum::http;

use crate::server;
use crate::server::api::auth;
use crate::server::api::spa::static_handler;
use crate::server::api::ws::ws_logs;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::service_register::ServiceRegister;
use crate::server::infrastructure::users::Backend;
use axum::http::HeaderValue;
use axum::routing::get;
use axum_login::{AuthManagerLayerBuilder, login_required};
use error_stack::ResultExt;
use std::net::SocketAddr;
use time::Duration;
use tokio::signal;
use tokio::task::AbortHandle;
use tower_http::cors::{AllowMethods, CorsLayer};
use tower_sessions::{ExpiredDeletion, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;
use tracing::{error, info};

/// 应用程序控制器，负责启动和管理Web服务器
pub struct ApplicationController;

impl ApplicationController {
    /// 启动Web服务器
    pub async fn serve(
        addr: &SocketAddr,
        enable_login_guard: bool,
        secure_session_cookie: bool,
        service_register: ServiceRegister,
    ) -> AppResult<()> {
        // 会话层配置
        // 使用 tower-sessions 建立会话层，将会话作为请求扩展提供
        let session_store = SqliteStore::new(service_register.pool.clone());
        session_store
            .migrate()
            .await
            .change_context(AppError::Unknown)?;

        // 启动定期清理过期会话的任务
        let deletion_task = tokio::task::spawn(
            session_store
                .clone()
                .continuously_delete_expired(tokio::time::Duration::from_secs(60)),
        );

        // 生成用于签名会话cookie的加密密钥
        // let key = Key::generate();

        // 配置会话管理层
        let session_layer = SessionManagerLayer::new(session_store)
            // The server itself only speaks plain HTTP, and browsers refuse to
            // store `Secure` cookies delivered over insecure remote origins, so
            // a forced `Secure` attribute on non-loopback binds silently broke
            // every direct-HTTP login. Default to a non-`Secure` cookie and let
            // deployments behind an HTTPS reverse proxy opt in explicitly via
            // `--secure-session-cookie`.
            .with_secure(secure_session_cookie)
            .with_name("biliup.sid")
            .with_expiry(Expiry::OnInactivity(Duration::days(7)));
        // .with_signed(key);

        // 认证服务配置
        // 将会话层与后端结合，建立认证服务，将认证会话作为请求扩展提供
        let backend = Backend::new(service_register.pool.clone());
        let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

        // 构建应用程序路由
        // 是否启用登录保护
        let protected_routes =
            server::router::router(service_register.clone()).route("/v1/ws/logs", get(ws_logs));
        let mut app = with_optional_auth(protected_routes, enable_login_guard);
        app = app
            .layer(auth_layer) // 添加认证层
            .layer(
                // CORS配置 - 跨域资源共享
                // 详见 https://docs.rs/tower-http/latest/tower_http/cors/index.html
                // 注意：对于某些请求类型（如POST application/json），
                // 需要添加 ".allow_headers([http::header::CONTENT_TYPE])"
                // 参考：https://github.com/tokio-rs/axum/issues/849
                CorsLayer::new()
                    .allow_headers([http::header::CONTENT_TYPE])
                    .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
                    .allow_methods(AllowMethods::any()),
            )
            .fallback(static_handler); // 静态文件处理回退

        // 启动HTTP服务器
        info!("routes initialized, listening on {}", addr);
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .change_context(AppError::Unknown)?;

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal(
                deletion_task.abort_handle(),
                service_register,
            ))
            .await
            .change_context(AppError::Unknown)
            .attach("error while starting API server")?;

        // 等待会话清理任务完成
        match deletion_task.await {
            Ok(Ok(())) => { /* 正常完成 */ }
            Ok(Err(e)) => {
                // 真正业务错误
                return Err(e).change_context(AppError::Unknown);
            }
            Err(join_err) if join_err.is_cancelled() => {
                info!("Deletion task cancelled on shutdown");
            }
            Err(join_err) if join_err.is_panic() => {
                error!("Deletion task panicked: {join_err}");
                return Err(AppError::Unknown.into());
            }
            Err(join_err) => {
                error!("Join error: {join_err}");
                return Err(AppError::Unknown.into());
            }
        }

        Ok(())
    }
}

fn with_optional_auth(app: axum::Router<()>, enable_login_guard: bool) -> axum::Router<()> {
    if enable_login_guard {
        app.route_layer(login_required!(Backend))
            .merge(auth::router())
    } else {
        app
    }
}

/// 优雅关闭信号处理
async fn shutdown_signal(
    deletion_task_abort_handle: AbortHandle,
    service_register: ServiceRegister,
) {
    // 监听Ctrl+C信号
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    // Unix系统下监听SIGTERM信号
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    // 非Unix系统下使用pending future
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    // 等待任一信号触发，然后中止清理任务
    tokio::select! {
        _ = ctrl_c => { deletion_task_abort_handle.abort() },
        _ = terminate => { deletion_task_abort_handle.abort() },
    }
    service_register.cleanup().await;
}

#[cfg(test)]
mod tests {
    use super::with_optional_auth;
    use crate::server::api::auth;
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use crate::server::infrastructure::users::Backend;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use axum::routing::get;
    use axum_login::AuthManagerLayerBuilder;
    use tower::ServiceExt;
    use tower_sessions::SessionManagerLayer;
    use tower_sessions_sqlx_store::SqliteStore;

    async fn request_log_route(enable_login_guard: bool) -> StatusCode {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let session_store = SqliteStore::new(pool.clone());
        session_store.migrate().await.unwrap();
        let auth_layer = AuthManagerLayerBuilder::new(
            Backend::new(pool),
            SessionManagerLayer::new(session_store).with_secure(false),
        )
        .build();
        let protected = Router::new().route("/v1/ws/logs", get(|| async { StatusCode::OK }));
        let app = with_optional_auth(protected, enable_login_guard).layer(auth_layer);

        app.oneshot(
            Request::builder()
                .uri("/v1/ws/logs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
    }

    #[tokio::test]
    async fn log_websocket_route_is_protected_when_auth_is_enabled() {
        assert_eq!(request_log_route(true).await, StatusCode::UNAUTHORIZED);
        assert_eq!(request_log_route(false).await, StatusCode::OK);
    }

    /// Issues a real login-session `Set-Cookie` through the register endpoint
    /// and returns the raw header value.
    async fn session_set_cookie_header(secure_session_cookie: bool) -> String {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite3");
        let pool = ConnectionManager::new_pool(db.to_str().unwrap())
            .await
            .unwrap();
        let session_store = SqliteStore::new(pool.clone());
        session_store.migrate().await.unwrap();
        let auth_layer = AuthManagerLayerBuilder::new(
            Backend::new(pool),
            SessionManagerLayer::new(session_store).with_secure(secure_session_cookie),
        )
        .build();
        let app = auth::router().layer(auth_layer);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/users/register")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"username":"biliup","password":"test-password"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        response
            .headers()
            .get(header::SET_COOKIE)
            .expect("login must establish a session cookie")
            .to_str()
            .unwrap()
            .to_string()
    }

    /// Direct HTTP access (the default deployment) must not mark the session
    /// cookie `Secure`, otherwise browsers on remote non-HTTPS origins drop it
    /// and every login silently bounces back to the login page.
    #[tokio::test]
    async fn session_cookie_secure_attribute_follows_configuration() {
        assert!(
            !session_set_cookie_header(false).await.contains("Secure"),
            "default session cookie must work over plain HTTP"
        );
        assert!(
            session_set_cookie_header(true).await.contains("Secure"),
            "--secure-session-cookie must mark the cookie Secure"
        );
    }
}
