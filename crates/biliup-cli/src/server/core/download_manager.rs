use crate::server::common::download::{DActor, DownloaderMessage};
use crate::server::common::upload::UActor;
use crate::server::core::monitor::Monitor;
use crate::server::core::plugin::DownloadPlugin;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::{Stage, Worker, WorkerStatus};
use async_channel::{Sender, bounded};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::info;

/// 下载管理器
/// 负责管理特定平台的下载任务，包括监控器和插件
pub struct DownloadManager {
    /// 下载插件
    // plugins: Vec<Arc<dyn DownloadPlugin + Send + Sync>>,
    rooms_handle: Arc<Monitor>,

    /// 下载信号量数量
    download_semaphore: u32,
    /// 上传信号量数量
    update_semaphore: u32,
    /// 下载消息发送器
    pub down_sender: Sender<DownloaderMessage>,
    /// 下载Actor任务句柄列表
    pub(crate) d_kills: Vec<JoinHandle<()>>,
    /// 上传Actor任务句柄列表
    pub(crate) u_kills: Vec<JoinHandle<()>>,
}

impl DownloadManager {
    /// 创建新的下载管理器实例
    ///
    /// # 参数
    /// * `plugin` - 下载插件实现
    /// * `actor_handle` - Actor处理器
    pub fn new(download_semaphore: u32, update_semaphore: u32, pool: ConnectionPool) -> Self {
        // 创建消息通道
        let (up_tx, up_rx) = bounded(16);
        let (down_tx, down_rx) = bounded(1);
        let mut d_kills = Vec::new();
        let mut u_kills = Vec::new();

        let rooms_handle = Arc::new(Monitor::new(down_tx.clone(), pool.clone()));
        // 创建下载Actor
        for _ in 0..download_semaphore {
            let mut d_actor = DActor::new(down_rx.clone(), up_tx.clone(), rooms_handle.clone());
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
            down_sender: down_tx,
            d_kills,
            u_kills,
            rooms_handle,
        }
    }

    pub fn add_plugin(&self, plugin: Arc<dyn DownloadPlugin + Send + Sync>) {
        let rooms_handle = self.rooms_handle.clone();
        tokio::spawn(async move {
            let name = plugin.name().to_string();
            rooms_handle.add_plugin(plugin).await;
            info!("Added plugin[{}]", name);
        });
    }

    pub async fn add_room(&self, worker: Worker) -> Option<()> {
        let arc = Arc::new(worker);
        self.rooms_handle.add(arc.clone()).await?;
        Some(())
    }

    pub async fn del_room(&self, id: i64) {
        self.rooms_handle.del(id).await
    }

    pub async fn get_rooms(&self) -> Vec<Arc<Worker>> {
        self.rooms_handle.get_all().await
    }

    /// 移出工作队列
    pub async fn make_waker(&self, id: i64) {
        self.rooms_handle.make_waker(id).await
    }

    pub async fn wake_waker(&self, id: i64) {
        self.rooms_handle.wake_waker(id).await;
    }

    pub async fn get_room_by_id(&self, id: i64) -> Option<Arc<Worker>> {
        self.rooms_handle
            .get_all()
            .await
            .iter()
            .find(|worker| worker.id() == id)
            .cloned()
    }

    pub async fn cleanup(&self) {
        let vec = self.rooms_handle.get_all().await;
        for worker in vec {
            worker
                .change_status(Stage::Download, WorkerStatus::Idle)
                .await;
        }
        info!("Cleanup complete");
    }
}

impl Drop for DownloadManager {
    fn drop(&mut self) {
        // 发送端随 ActorHandle 一起被 drop，会关闭通道（如果没有其他 sender 克隆）。
        // 为避免 tokio 任务在后台“挂着”，这里直接 abort。
        // for h in &self.d_kills {
        //     h.abort();
        // }
        for h in &self.u_kills {
            h.abort();
        }
        info!("exit download manager");
    }
}
