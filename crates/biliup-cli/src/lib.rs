pub mod server;
pub mod cli;
pub mod downloader;
pub mod uploader;

// use crate::server::api::router::ApplicationController;
use crate::server::app::ApplicationController;
use crate::server::core::download_manager::ActorHandle;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::repositories;
use crate::server::infrastructure::service_register::ServiceRegister;
use clap::ValueEnum;
use error_stack::ResultExt;
use std::net::ToSocketAddrs;
use std::sync::{Arc, RwLock};

pub async fn run(addr: (&str, u16), auth: bool) -> AppResult<()> {
    // let config = Arc::new(AppConfig::parse());

    tracing::info!(
        "environment loaded and configuration parsed, initializing Postgres connection and running migrations..."
    );
    let conn_pool = ConnectionManager::new_pool("data/data.sqlite3")
        .await
        .expect("could not initialize the database connection pool");

    let config = Arc::new(RwLock::new(repositories::get_config(&conn_pool).await?));
    let actor_handle = Arc::new(ActorHandle::new(
        config.read().unwrap().pool1_size,
        config.read().unwrap().pool2_size,
    ));
    let vec = Vec::new();

    let service_register = ServiceRegister::new(conn_pool, config.clone(), actor_handle, vec);

    tracing::info!("migrations successfully ran, initializing axum server...");
    let addr = addr
        .to_socket_addrs()
        .change_context(AppError::Unknown)?
        .next()
        .unwrap();
    ApplicationController::serve(&addr, auth, service_register)
        .await
        .attach("could not initialize application routes")?;
    Ok(())
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum UploadLine {
    Bldsa,
    Cnbldsa,
    Andsa,
    Atdsa,
    Bda2,
    Cnbd,
    Anbd,
    Atbd,
    Tx,
    Cntx,
    Antx,
    Attx,
    Bda,
    Txa,
    Alia,
}
