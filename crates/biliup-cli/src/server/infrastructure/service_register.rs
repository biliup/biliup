use crate::server::config::Config;
use crate::server::core::download_manager::DownloadManager;
use crate::server::core::plugin::yy::YY;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::Worker;
use crate::server::infrastructure::models::live_streamer::LiveStreamer;
use crate::server::infrastructure::models::upload_streamer::UploadStreamer;
use axum::extract::FromRef;
use biliup::client::StatelessClient;
use std::sync::{Arc, RwLock};
use error_stack::fmt::ColorMode;
use error_stack::Report;
use tracing::info;

/// 服务注册器
/// 负责管理应用程序中的各种服务实例，包括数据库连接池、工作器、下载管理器等
#[derive(FromRef, Clone)]
pub struct ServiceRegister {
    /// 数据库连接池
    pub pool: ConnectionPool,
    /// 下载管理器列表
    pub managers: Arc<DownloadManager>,
    /// 全局配置
    pub config: Arc<RwLock<Config>>,
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
        download_manager: DownloadManager,
    ) -> Self {
        Report::set_color_mode(ColorMode::None);
        info!("initializing utility services...");
        // 创建默认的HTTP客户端
        let client = StatelessClient::default();

        info!("utility services initialized, building feature services...");

        // download_manager.push(DownloadManager::new(YY::new(), actor_handle.clone()));
        download_manager.add_plugin(Arc::new(YY::new()));

        info!("feature services successfully initialized!");
        ServiceRegister {
            pool,
            managers: Arc::new(download_manager),
            config: config.clone(),
            client,
        }
    }

    pub fn worker(
        &self,
        live_streamer: LiveStreamer,
        upload_streamer: Option<UploadStreamer>,
    ) -> Worker {
        Worker::new(
            live_streamer,
            upload_streamer,
            self.config.clone(),
            self.client.clone(),
        )
    }

    pub async fn cleanup(&self) {
        self.managers.cleanup().await;
    }
}

// impl FromRef<ServiceRegister> for ConnectionPool {
//     fn from_ref(app_state: &ServiceRegister) -> ConnectionPool {
//         app_state.pool.clone()
//     }
// }
