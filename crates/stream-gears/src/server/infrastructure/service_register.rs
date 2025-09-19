use crate::server::config;
use crate::server::config::Config;
use crate::server::core::download_manager::{ActorHandle, DownloadManager};
use crate::server::core::monitor::Monitor;
use crate::server::core::plugin;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::Worker;
use axum::extract::FromRef;
use error_stack::bail;
use ormlite::Model;
use std::sync::{Arc, RwLock};
use tracing::info;

#[derive(FromRef, Clone)]
pub struct ServiceRegister {
    pub pool: ConnectionPool,
    pub workers: Arc<RwLock<Vec<Arc<Worker>>>>,
    pub managers: Arc<Vec<DownloadManager>>,
    pub config: Arc<RwLock<Config>>,
    pub actor_handle: Arc<ActorHandle>,
}

/// A simple service container responsible for managing the various services our API endpoints will pull from through axum extensions.
impl ServiceRegister {
    pub fn new(pool: ConnectionPool, config: Config) -> Self {
        info!("initializing utility services...");

        let cf = config::CONFIG.get_or_init(|| {
            Arc::new(RwLock::new(config))
            // Arc::new(RwLock::new(Config::builder()
            //     // .user(UserConfig::default())
            //     // .douyu_cdn("hw-h5".to_string())
            //     // .douyu_rate(0)
            //     .streamers(HashMap::<String, StreamerConfig>::new())
            //     .build()))
        });

        info!(config=?cf);
        let actor_handle = Arc::new(ActorHandle::new(
            cf.read().unwrap().pool1_size,
            cf.read().unwrap().pool2_size,
        ));

        info!("utility services initialized, building feature services...");

        let vec = plugin::from_py(actor_handle.clone()).unwrap();

        info!("feature services successfully initialized!");

        ServiceRegister {
            pool,
            workers: Arc::new(Default::default()),
            managers: Arc::new(vec),
            config: cf.clone(),
            actor_handle,
        }
    }

    pub fn get_manager(&self, url: &str) -> Option<&DownloadManager> {
        self.managers.iter().find(|&manager| manager.matches(url)).map(|v| v as _)
    }

    pub async fn add_room(
        &self,
        id: i64,
        url: &str,
        monitor: Arc<Monitor>,
    ) -> AppResult<Option<()>> {
        let worker = Arc::new(Worker::new(id, url, self.pool.clone()));
        monitor.rooms_handle.add(worker.clone()).await;
        self.workers.write().unwrap().push(worker);
        info!("add {url} success");
        Ok(Some(()))
    }

    pub async fn del_room(&self, id: i64) -> AppResult<()> {
        let Some(i) = self.workers.read().unwrap().iter().position(|x| x.id == id) else {
            return Err(error_stack::Report::new(AppError::Unknown));
        };

        let removed = self.workers.write().unwrap().swap_remove(i);
        let url = &removed.url;
        let Some(manager) = self.get_manager(url) else {
            info!("not found url: {url}");
            bail!(AppError::Unknown)
        };
        let monitor = manager.ensure_monitor();
        let len = monitor.rooms_handle.del(id).await;
        info!("{id} removed, remained len {len}");
        if len == 0 {
            *manager.monitor.lock().unwrap() = None;
        }

        Ok(())
    }
}

// impl FromRef<ServiceRegister> for ConnectionPool {
//     fn from_ref(app_state: &ServiceRegister) -> ConnectionPool {
//         app_state.pool.clone()
//     }
// }

// pub(crate) async fn add_room(&self, id: i64, pool: ConnectionPool) -> Arc<Worker> {
//     let arc = self.ensure_monitor().rooms_handle.clone();
//     let worker = Arc::new(Worker::new(id, pool, arc.clone()));
//     arc.add(worker.clone()).await;
//     worker
// }
//
// pub async fn del_room(&self, id: i64) {
//     let len = self.ensure_monitor().rooms_handle.del(id).await;
//     info!("{id} removed, remained len {len}");
//     if len == 0 {
//         *self.monitor.lock().unwrap() = None;
//     }
// }
