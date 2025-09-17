use biliup::client::StatelessClient;

use anyhow::Context;
use axum::routing::{delete, get, post, put};
use axum::{Extension, Router, http};

use crate::server;
use crate::server::api::auth;
use crate::server::api::bilibili_endpoints::{
    archive_pre_endpoint, get_myinfo_endpoint, get_proxy_endpoint,
};
use crate::server::api::endpoints::{
    delete_streamers_endpoint, delete_user_endpoint, get_configuration, get_streamer_info,
    get_streamers_endpoint, get_upload_streamer_endpoint, get_upload_streamers_endpoint,
    get_users_endpoint, post_streamers_endpoint, put_streamers_endpoint,
};
use crate::server::api::spa::static_handler;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::service_register::ServiceRegister;
use crate::server::infrastructure::users::Backend;
use axum::http::HeaderValue;
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

pub struct ApplicationController;

impl ApplicationController {
    pub async fn serve(addr: &SocketAddr, service_register: ServiceRegister) -> AppResult<()> {
        // Session layer.
        //
        // This uses `tower-sessions` to establish a layer that will provide the session
        // as a request extension.
        let session_store = SqliteStore::new(service_register.pool.clone());
        session_store
            .migrate()
            .await
            .change_context(AppError::Unknown)?;

        let deletion_task = tokio::task::spawn(
            session_store
                .clone()
                .continuously_delete_expired(tokio::time::Duration::from_secs(60)),
        );

        // Generate a cryptographic key to sign the session cookie.
        let key = Key::generate();

        let session_layer = SessionManagerLayer::new(session_store)
            .with_secure(false)
            .with_expiry(Expiry::OnInactivity(Duration::days(1)))
            .with_signed(key);

        // Auth service.
        //
        // This combines the session layer with our backend to establish the auth
        // service which will provide the auth session as a request extension.
        let backend = Backend::new(service_register.pool.clone());
        let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

        // let app = protected::router()

        let client = StatelessClient::default();
        // let vec = service_register.streamers_service.get_streamers().await?;
        // build our application with a route
        let enable_login_guard = true;
        let mut app = server::router::router(service_register);
        if enable_login_guard {
            app = app
                .route_layer(login_required!(Backend))
                .merge(auth::router());
        }
        app = app
            .layer(auth_layer)
            .layer(
                // see https://docs.rs/tower-http/latest/tower_http/cors/index.html
                // for more details
                //
                // pay attention that for some request types like posting content-type: application/json
                // it is required to add ".allow_headers([http::header::CONTENT_TYPE])"
                // or see this issue https://github.com/tokio-rs/axum/issues/849
                CorsLayer::new()
                    .allow_headers([http::header::CONTENT_TYPE])
                    .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
                    .allow_methods(AllowMethods::any()),
            )
            .layer(Extension(client.clone()))
            .fallback(static_handler);
        // run our app with hyper
        // `axum::Server` is a re-export of `hyper::Server`
        info!("routes initialized, listening on {}", addr);
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .change_context(AppError::Unknown)?;

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal(deletion_task.abort_handle()))
            .await
            .change_context(AppError::Unknown)
            .attach("error while starting API server")?;

        deletion_task
            .await
            .change_context(AppError::Unknown)?
            .change_context(AppError::Unknown)?;

        Ok(())
    }
}

async fn shutdown_signal(deletion_task_abort_handle: AbortHandle) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { deletion_task_abort_handle.abort() },
        _ = terminate => { deletion_task_abort_handle.abort() },
    }
}
