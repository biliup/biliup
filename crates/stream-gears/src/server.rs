mod api;
mod app;
pub(crate) mod config;
mod core;
mod errors;
pub mod infrastructure;
mod router;
mod util;

use error_stack::ResultExt;
use futures::TryFutureExt;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::PyListMethods;
use pyo3::{PyResult, Python, pyfunction};
use std::net::ToSocketAddrs;
// use tokio::sync::mpsc::Receiver;
use crate::server::app::ApplicationController;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::repositories;
use crate::server::infrastructure::service_register::ServiceRegister;
use tracing::info;

#[pyfunction]
pub fn main_loop(py: Python<'_>) -> PyResult<()> {
    py.detach(_main)
        .map_err(|e| PyRuntimeError::new_err(format!("{e:?}")))

    // for item in plugin_list.iter() {
    //     // 每个元素都是一个类（type）
    //     let ty: &Bound<PyType> = item.downcast()?;
    //
    //     // 确认是否是 DownloadBase 的子类
    //     if ty.is_subclass(download_base)? {
    //         let name: String = ty.getattr("__name__")?.extract()?;
    //         println!("发现插件类: {name}");
    //
    //         // 如果要实例化（若不是抽象类且构造器无参）
    //         // let obj = ty.call0()?;
    //         // 或者需要参数：ty.call1((arg1, arg2,))?;
    //
    //         // 如果要调用方法（示例）
    //         // if let Ok(method) = obj.getattr("download") {
    //         //     let result = method.call0()?;
    //         //     println!("download() 返回: {}", result.repr()?.extract::<String>()?);
    //         // }
    //     }
    // }
}

#[tokio::main]
async fn _main() -> AppResult<()> {
    info!(
        "environment loaded and configuration parsed, initializing Postgres connection and running migrations..."
    );
    let conn_pool = ConnectionManager::new_pool("data/data.sqlite3")
        .await
        .expect("could not initialize the database connection pool");
    
    let configs = repositories::get_config(&conn_pool).await?;
    let service_register = ServiceRegister::new(conn_pool, configs);

    let all_streamer = repositories::get_all_streamer(&service_register.pool).await?;
    let all_uploader = repositories::get_all_uploader(&service_register.pool).await?;
    // let mut monitors = Vec::new();

    for streamer in all_streamer {
        // workers.push(Arc::new(Worker::new(streamer.id, service_register.pool.clone())));
        if let Some(manager) = service_register.get_manager(&streamer.url) {
            let monitor = manager.ensure_monitor();
            let _ = service_register
                .add_room(streamer.id, &streamer.url, monitor)
                .await?;
        };
    }

    info!("migrations successfully ran, initializing axum server...");
    let addr = ("0.0.0.0", 19159);
    let addr = addr
        .to_socket_addrs()
        .change_context(AppError::Unknown)?
        .next()
        .unwrap();
    ApplicationController::serve(&addr, service_register)
        .await
        .attach("could not initialize application routes")?;
    Ok(())
}
