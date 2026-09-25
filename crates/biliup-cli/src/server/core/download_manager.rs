use crate::server::common::upload::UActor;
use crate::server::core::monitor::Monitor;
use crate::server::core::slots::Slots;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::{Stage, Worker, WorkerStatus};
use async_channel::bounded;
use biliup::downloader::live::LivePlugin;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::info;

/// 下载管理器
/// 负责管理特定平台的下载任务，包括监控器和插件
pub struct DownloadManager {
    /// 下载插件
    // plugins: Vec<Arc<dyn LivePlugin + Send + Sync>>,
    rooms_handle: Arc<Monitor>,

    /// 下载池（pool1_size）：同时录制的直播间上限，由 Monitor 在检测开播前占用。
    download_slots: Arc<Slots>,
    /// 上传池（pool2_size）：同时进行的上传流程上限，由上传Actor在处理每条消息前占用。
    upload_slots: Arc<Slots>,
    /// 上传Actor任务句柄
    u_kill: JoinHandle<()>,
}

impl DownloadManager {
    /// 创建新的下载管理器实例
    ///
    /// # 参数
    /// * `pool1_size` - 下载池容量
    /// * `pool2_size` - 上传池容量
    /// * `pool` - 数据库连接池
    pub fn new(pool1_size: u32, pool2_size: u32, pool: ConnectionPool) -> Self {
        // 创建消息通道
        let (up_tx, up_rx) = bounded(16);
        let download_slots = Arc::new(Slots::new(pool1_size as usize));
        let upload_slots = Arc::new(Slots::new(pool2_size as usize));

        let rooms_handle = Arc::new(Monitor::new(up_tx, download_slots.clone(), pool));
        // 创建上传Actor
        let u_actor = UActor::new(up_rx, upload_slots.clone());
        let u_kill = tokio::spawn(u_actor.run());

        Self {
            rooms_handle,
            download_slots,
            upload_slots,
            u_kill,
        }
    }

    /// 按新配置调整下载池 / 上传池容量，立即生效。
    ///
    /// 调小不会打断正在进行的录制和上传，占用数降到新容量以下后才会开始新的任务。
    pub fn resize_pools(&self, pool1_size: u32, pool2_size: u32) {
        self.download_slots.resize(pool1_size as usize);
        self.upload_slots.resize(pool2_size as usize);
    }

    /// 下载池当前容量
    pub fn download_pool_size(&self) -> usize {
        self.download_slots.capacity()
    }

    /// 上传池当前容量
    pub fn upload_pool_size(&self) -> usize {
        self.upload_slots.capacity()
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

    /// 按房间 URL 找到对应的平台插件。
    pub async fn plugin_for(&self, url: &str) -> Option<Arc<dyn LivePlugin + Send + Sync>> {
        self.rooms_handle.plugin_for(url).await
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
        // 上传Actor的在途上传任务在它的 JoinSet 里，随它一起取消
        self.u_kill.abort();
        info!("exit download manager");
    }
}
