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
use tower_sessions::cookie::Key;
use tower_sessions::{ExpiredDeletion, Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;
use tracing::info;

/// 应用程序控制器，负责启动和管理Web服务器
pub struct ApplicationController;

impl ApplicationController {
    /// 启动Web服务器
    pub async fn serve(addr: &SocketAddr, service_register: ServiceRegister) -> AppResult<()> {
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
        let key = Key::generate();

        // 配置会话管理层
        let session_layer = SessionManagerLayer::new(session_store)
            .with_secure(false)
            .with_expiry(Expiry::OnInactivity(Duration::days(7)));
        // .with_signed(key);

        // 认证服务配置
        // 将会话层与后端结合，建立认证服务，将认证会话作为请求扩展提供
        let backend = Backend::new(service_register.pool.clone());
        let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

        // 构建应用程序路由
        let enable_login_guard = true; // 是否启用登录保护
        let mut app = server::router::router(service_register);
        if enable_login_guard {
            app = app
                .route_layer(login_required!(Backend)) // 添加登录验证中间件
                .merge(auth::router()); // 合并认证路由
        }
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
            .route("/v1/ws/logs", get(ws_logs)) // 获取视频列表
            .fallback(static_handler); // 静态文件处理回退

        // 启动HTTP服务器
        info!("routes initialized, listening on {}", addr);
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .change_context(AppError::Unknown)?;

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal(deletion_task.abort_handle()))
            .await
            .change_context(AppError::Unknown)
            .attach("error while starting API server")?;

        // 等待会话清理任务完成
        deletion_task
            .await
            .change_context(AppError::Unknown)?
            .change_context(AppError::Unknown)?;

        Ok(())
    }
}

/// 优雅关闭信号处理
async fn shutdown_signal(deletion_task_abort_handle: AbortHandle) {
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
}
