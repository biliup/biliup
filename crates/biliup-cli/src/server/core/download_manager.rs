use crate::server::common::upload::UActor;
use crate::server::core::monitor::Monitor;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::{Stage, Worker, WorkerStatus};
use async_channel::bounded;
use biliup::downloader::live::LivePlugin;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;
use tracing::info;

/// 下载管理器
/// 负责管理特定平台的下载任务，包括监控器和插件
pub struct DownloadManager {
    /// 下载插件
    // plugins: Vec<Arc<dyn LivePlugin + Send + Sync>>,
    rooms_handle: Arc<Monitor>,

    /// 下载池大小。真正的并发控制由 Monitor 持有的 Semaphore 完成。
    pub download_semaphore: u32,
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
        let mut u_kills = Vec::new();

        let rooms_handle = Arc::new(Monitor::new(
            up_tx.clone(),
            Arc::new(Semaphore::new(download_semaphore as usize)),
            pool.clone(),
        ));
        // 创建上传Actor
        for _ in 0..update_semaphore {
            let mut u_actor = UActor::new(up_rx.clone());
            let u_kill = tokio::spawn(async move { u_actor.run().await });
            u_kills.push(u_kill)
        }

        Self {
            download_semaphore,
            u_kills,
            rooms_handle,
        }
    }

    pub async fn add_plugin(&self, plugin: Arc<dyn LivePlugin + Send + Sync>) {
        let name = plugin.name().to_string();
        self.rooms_handle.add_plugin(plugin).await;
        info!("Added plugin[{}]", name);
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
        for h in &self.u_kills {
            h.abort();
        }
        info!("exit download manager");
    }
}
