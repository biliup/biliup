use crate::server::common::download::{DActor, DownloaderMessage};
use crate::server::common::upload::{UActor, UploaderMessage};
use crate::server::core::downloader::Downloader;
use crate::server::core::monitor::Monitor;
use crate::server::core::plugin::DownloadPlugin;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use async_channel::{Sender, bounded};
use core::fmt;
use error_stack::ResultExt;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
/// 下载管理器
/// 负责管理特定平台的下载任务，包括监控器和插件
pub struct DownloadManager {
    /// 监控器实例（可选，使用Mutex保护）
    pub monitor: Mutex<Option<Arc<Monitor>>>,
    /// 下载插件
    plugin: Arc<dyn DownloadPlugin + Send + Sync>,
    /// Actor处理器
    actor_handle: Arc<ActorHandle>,
}

impl DownloadManager {
    /// 创建新的下载管理器实例
    ///
    /// # 参数
    /// * `plugin` - 下载插件实现
    /// * `actor_handle` - Actor处理器
    pub fn new(
        plugin: impl DownloadPlugin + Send + Sync + 'static,
        actor_handle: Arc<ActorHandle>,
    ) -> Self {
        Self {
            monitor: Mutex::new(None),
            plugin: Arc::new(plugin),
            actor_handle,
        }
    }

    /// 确保监控器存在，如果不存在则创建新的
    ///
    /// # 返回
    /// 返回监控器的Arc引用
    pub fn ensure_monitor(&self, pool: ConnectionPool) -> Arc<Monitor> {
        self.monitor
            .lock()
            .unwrap()
            .get_or_insert_with(|| {
                Arc::new(Monitor::new(
                    self.plugin.clone(),
                    self.actor_handle.clone(),
                    pool,
                ))
            })
            .clone()
    }

    /// 检查URL是否匹配此下载管理器的插件
    ///
    /// # 参数
    /// * `url` - 要检查的URL
    ///
    /// # 返回
    /// 如果URL匹配返回true，否则返回false
    pub fn matches(&self, url: &str) -> bool {
        self.plugin.matches(url)
    }
}

impl fmt::Debug for DownloadManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DownloadManager [{:?}]", self.monitor)
    }
}

/// Actor处理器
/// 管理下载和上传Actor的生命周期
pub struct ActorHandle {
    /// 下载信号量数量
    download_semaphore: u32,
    /// 上传信号量数量
    update_semaphore: u32,
    /// 上传消息发送器
    pub up_sender: Sender<UploaderMessage>,
    /// 下载消息发送器
    pub down_sender: Sender<DownloaderMessage>,
    /// 下载Actor任务句柄列表
    pub(crate) d_kills: Vec<JoinHandle<()>>,
    /// 上传Actor任务句柄列表
    pub(crate) u_kills: Vec<JoinHandle<()>>,
}

impl ActorHandle {
    /// 创建新的Actor处理器实例
    ///
    /// # 参数
    /// * `download_semaphore` - 下载Actor数量
    /// * `update_semaphore` - 上传Actor数量
    pub fn new(download_semaphore: u32, update_semaphore: u32) -> Self {
        // 创建消息通道
        let (up_tx, up_rx) = bounded(16);
        let (down_tx, down_rx) = bounded(1);
        let mut d_kills = Vec::new();
        let mut u_kills = Vec::new();
        // 创建下载Actor
        for _ in 0..download_semaphore {
            let mut d_actor = DActor::new(down_rx.clone(), up_tx.clone());
            let d_kill = tokio::spawn(async move { d_actor.run().await });
            d_kills.push(d_kill)
        }
        // 创建上传Actor
        for _ in 0..update_semaphore {
            let mut u_actor = UActor::new(up_rx.clone());
            let u_kill = tokio::spawn(async move { u_actor.run().await });
            u_kills.push(u_kill)
        }

        Self {
            download_semaphore,
            update_semaphore,
            up_sender: up_tx,
            down_sender: down_tx,
            d_kills,
            u_kills,
        }
    }
}

impl Drop for ActorHandle {
    fn drop(&mut self) {
        // 发送端随 ActorHandle 一起被 drop，会关闭通道（如果没有其他 sender 克隆）。
        // 为避免 tokio 任务在后台“挂着”，这里直接 abort。
        for h in &self.d_kills {
            h.abort();
        }
        for h in &self.u_kills {
            h.abort();
        }
    }
}
