use crate::LogHandle;
use crate::server::api::live_media::ImageProxy;
use crate::server::common::system_stats::SystemMonitor;
use crate::server::config::Config;
use crate::server::core::download_manager::DownloadManager;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::Worker;
use crate::server::infrastructure::models::live_streamer::LiveStreamer;
use crate::server::infrastructure::models::upload_streamer::UploadStreamer;
use crate::server::workbench::clips::export::ClipExports;
use axum::extract::FromRef;
use biliup::client::StatelessClient;
use biliup::downloader::live::builtin_plugins;
use error_stack::Report;
use error_stack::fmt::ColorMode;
use std::sync::{Arc, RwLock};
use tracing::info;

/// 切片产物的目录（相对工作目录）。
pub const CLIPS_DIR: &str = "clips";

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

    pub log_handle: LogHandle,

    /// 录制中直播间封面 / 头像的图片代理（抓取客户端 + 内存缓存）
    pub image_proxy: Arc<ImageProxy>,

    /// 控制台首页的系统状态采样（CPU / 内存 / 磁盘 / 网速），随服务启停
    pub system: Arc<SystemMonitor>,
    /// 切片导出任务（产物在工作目录下的 `clips/`）
    pub clips: Arc<ClipExports>,
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
    pub async fn new(
        pool: ConnectionPool,
        config: Arc<RwLock<Config>>,
        download_manager: DownloadManager,
        log_handle: LogHandle,
    ) -> Self {
        Report::set_color_mode(ColorMode::None);
        info!("initializing utility services...");
        // 创建默认的HTTP客户端
        let client = StatelessClient::default();

        info!("utility services initialized, building feature services...");

        // download_manager.push(DownloadManager::new(YY::new(), actor_handle.clone()));
        for plugin in builtin_plugins() {
            download_manager.add_plugin(plugin).await;
        }

        info!("feature services successfully initialized!");
        ServiceRegister {
            clips: Arc::new(ClipExports::new(pool.clone(), CLIPS_DIR)),
            pool,
            managers: Arc::new(download_manager),
            config: config.clone(),
            client,
            log_handle,
            image_proxy: Arc::new(ImageProxy::new()),
            system: SystemMonitor::spawn(),
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
