use crate::server::config::Config;
use crate::server::core::download_manager::{ActorHandle, DownloadManager};
use crate::server::core::monitor::Monitor;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::{Context, Worker};
use crate::server::infrastructure::models::{LiveStreamer, UploadStreamer};
use axum::extract::FromRef;
use biliup::client::StatelessClient;
use error_stack::bail;
use std::sync::{Arc, RwLock};
use tracing::info;

/// 服务注册器
/// 负责管理应用程序中的各种服务实例，包括数据库连接池、工作器、下载管理器等
#[derive(FromRef, Clone)]
pub struct ServiceRegister {
    /// 数据库连接池
    pub pool: ConnectionPool,
    /// 工作器列表
    pub workers: Arc<RwLock<Vec<Arc<Worker>>>>,
    /// 下载管理器列表
    pub managers: Arc<Vec<DownloadManager>>,
    /// 全局配置
    pub config: Arc<RwLock<Config>>,
    /// Actor处理器
    pub actor_handle: Arc<ActorHandle>,
    /// HTTP客户端
    pub client: StatelessClient,
}

/// 简单的服务容器，负责管理API端点通过axum扩展获取的各种服务
impl ServiceRegister {
    /// 创建新的服务注册器实例
    /// 
    /// # 参数
    /// * `pool` - 数据库连接池
    /// * `config` - 全局配置
    /// * `actor_handle` - Actor处理器
    /// * `download_manager` - 下载管理器列表
    pub fn new(
        pool: ConnectionPool,
        config: Arc<RwLock<Config>>,
        actor_handle: Arc<ActorHandle>,
        download_manager: Vec<DownloadManager>,
    ) -> Self {
        info!("initializing utility services...");
        // 创建默认的HTTP客户端
        let client = StatelessClient::default();

        info!(config=?config);

        info!("utility services initialized, building feature services...");

        info!("feature services successfully initialized!");

        ServiceRegister {
            pool,
            workers: Arc::new(Default::default()),
            managers: Arc::new(download_manager),
            config: config.clone(),
            actor_handle,
            client,
        }
    }

    /// 根据URL获取匹配的下载管理器
    /// 
    /// # 参数
    /// * `url` - 直播流URL
    /// 
    /// # 返回
    /// 返回匹配的下载管理器引用，如果没有匹配的则返回None
    pub fn get_manager(&self, url: &str) -> Option<&DownloadManager> {
        self.managers
            .iter()
            .find(|&manager| manager.matches(url))
            .map(|v| v as _)
    }

    /// 添加新的直播间到监控列表
    /// 
    /// # 参数
    /// * `monitor` - 监控器实例
    /// * `live_streamer` - 直播主播信息
    /// * `upload_streamer` - 上传配置（可选）
    pub async fn add_room(
        &self,
        monitor: Arc<Monitor>,
        live_streamer: LiveStreamer,
        upload_streamer: Option<UploadStreamer>,
    ) -> AppResult<Option<()>> {
        // 创建新的工作器实例
        let worker = Arc::new(Worker::new(
            live_streamer,
            upload_streamer,
            self.config.clone(),
            self.client.clone(),
        ));
        // 将工作器添加到监控器和工作器列表中
        monitor.rooms_handle.add(worker.clone()).await;
        self.workers.write().unwrap().push(worker.clone());
        info!("add {worker:?} success");
        Ok(Some(()))
    }

    /// 删除指定ID的直播间
    /// 
    /// # 参数
    /// * `id` - 要删除的直播间ID
    pub async fn del_room(&self, id: i64) -> AppResult<()> {
        // 在工作器列表中查找要删除的工作器
        let Some(i) = self
            .workers
            .read()
            .unwrap()
            .iter()
            .position(|x| x.live_streamer.id == id)
        else {
            return Err(error_stack::Report::new(AppError::Unknown));
        };

        // 从工作器列表中移除
        let removed = self.workers.write().unwrap().swap_remove(i);
        let url = &removed.live_streamer.url;
        // 获取对应的下载管理器
        let Some(manager) = self.get_manager(url) else {
            info!("not found url: {url}");
            bail!(AppError::Unknown)
        };
        // 从监控器中删除房间
        let monitor = manager.ensure_monitor();
        let len = monitor.rooms_handle.del(id).await;
        info!("id: {id} removed, remained len {len}");
        // 如果没有剩余房间，清理监控器
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
