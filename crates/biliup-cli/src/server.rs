pub mod errors;

pub mod api {
    pub mod bilibili_endpoints;
    pub mod endpoints;
    pub mod router;
}

pub mod core;

pub mod infrastructure {
    pub mod repositories {
        pub mod download_records_repository;
        pub mod live_streamers_repository;
        pub mod upload_records_repository;
        pub mod upload_streamers_repository;
        pub mod users_repository;
        pub mod videos_repository;
    }

    pub mod connection_pool;
    pub mod live_streamers_service;
    pub mod service_register;
}

use anyhow::{Context, Result};

use crate::server::api::router::ApplicationController;
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::service_register::ServiceRegister;
use std::net::ToSocketAddrs;

pub async fn run(addr: (&str, u16)) -> Result<()> {
    // let config = Arc::new(AppConfig::parse());

    tracing::info!(
        "environment loaded and configuration parsed, initializing Postgres connection and running migrations..."
    );
    let conn_pool = ConnectionManager::new_pool()
        .await
        .expect("could not initialize the database connection pool");

    let service_register = ServiceRegister::new(conn_pool);

    tracing::info!("migrations successfully ran, initializing axum server...");
    let addr = addr.to_socket_addrs()?.next().unwrap();
    ApplicationController::serve(&addr, service_register)
        .await
        .context("could not initialize application routes")?;
    Ok(())
}
