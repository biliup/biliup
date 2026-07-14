use crate::server::common::download::start_download_workflow;
use crate::server::common::upload::UploaderMessage;
use crate::server::core::live::{batch_check_request, live_request, streamer_info};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::context::{Context, Stage, Worker, WorkerStatus};
use crate::server::infrastructure::models::StreamerInfo;
use async_channel::Sender;
use biliup::downloader::live::{LivePlugin, LiveStatus};
use ormlite::Model;
use ormlite::model::ModelBuilder;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, oneshot};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, trace, warn};

/// 批量检测平台的开播缓存：一次批量请求的结果在一个检测周期内复用，
/// 供逐间循环快速判定某房间是否开播，避免对未开播房间做逐间请求。
#[derive(Debug)]
struct BatchLiveCache {
    live_urls: HashSet<String>,
    refreshed_at: Instant,
}

/// 批量检测对单个房间的判定结果。
enum BatchVerdict {
    /// 批量结果显示开播，继续逐间完整检测以获取流信息
    Live,
    /// 批量结果显示未开播，直接放回队列
    Offline,
    /// 平台不支持批量检测或本轮批量请求失败，回退逐间检测
    Fallback,
}

/// 批量结果为未开播时的快速轮换间隔：缓存命中无网络开销，
/// 短暂停即可在一个检测周期内扫完整个队列（网络频率仍由缓存 TTL 限制）。
const BATCH_ROTATE_SLEEP: Duration = Duration::from_secs(1);

/// 房间处理器
/// 管理多个直播间的状态和操作
#[derive(Debug)]
pub struct Monitor {
    /// 消息发送器
    sender: tokio::sync::mpsc::Sender<ActorMessage>,
    /// Actor任务句柄
    pool: ConnectionPool,
    /// 上传消息发送器，下载任务产生分段后会通过它交给上传流程。
    uploader: Sender<UploaderMessage>,
    /// 下载池许可。监控循环必须先拿到许可，才允许检测开播并启动录制。
    /// 这样 “开播了/成功开始录制” 只会出现在真正拥有下载并发槽位时。
    /// 许可由下载任务持有到录制结束，pool1_size 的唯一限流语义在这里表达。
    download_slots: Arc<Semaphore>,
    monitors: RwLock<HashMap<String, JoinHandle<()>>>,
    /// 各批量检测平台的开播缓存（platform_name -> 最近一次批量结果）。
    batch_live: RwLock<HashMap<String, BatchLiveCache>>,
}

impl Drop for Monitor {
    /// 监控器销毁时的清理逻辑
    fn drop(&mut self) {
        let sender = self.sender.clone();
        tokio::spawn(async move {
            let msg = ActorMessage::Shutdown;
            let _ = sender.send(msg).await;
            info!("RoomsHandle killed")
        });
        // 终止监控任务
        // self.kill.abort();
        // self.rooms_handle.kill.abort();
    }
}

impl Monitor {
    /// 创建新的房间处理器实例
    ///
    /// # 参数
    /// * `name` - 平台名称
    pub fn new(
        uploader: Sender<UploaderMessage>,
        download_slots: Arc<Semaphore>,
        pool: ConnectionPool,
    ) -> Self {
        // 创建消息通道
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let mut actor = RoomsActor::new(receiver);
        // 启动Actor任务
        let _kill = tokio::spawn(async move { actor.run().await });

        Self {
            sender,
            pool,
            uploader,
            download_slots,
            monitors: Default::default(),
            batch_live: Default::default(),
        }
    }

    /// 启动客户端监控循环
    ///
    /// # 参数
    /// * `rooms_handle` - 房间处理器
    /// * `plugin` - 下载插件
    /// * `actor_handle` - Actor处理器
    /// * `interval` - 监控间隔（秒）
    pub(crate) async fn start_monitor(
        self: &Arc<Self>,
        platform_name: &str,
        plugin: Arc<dyn LivePlugin + Send + Sync>,
    ) {
        info!("start -> [{platform_name}]");
        // 获取下一个要检查的房间
        while let Some(room) = self.next(platform_name).await {
            // 更新状态为等待中
            room.change_status(Stage::Download, WorkerStatus::Pending)
                .await;
            let url = room.get_streamer().url.clone();
            let interval = room.get_config().event_loop_interval;
            // 批量检测平台：先用一个检测周期内共享的批量结果快速判定是否开播。
            // 未开播直接跳过（不占用下载槽位、不做逐间请求），并以较短间隔轮换到
            // 下一个房间，使整条队列在一个检测周期内扫完；批量请求本身由缓存 TTL 限流。
            if plugin.supports_batch_check() {
                match self.batch_verdict(platform_name, &plugin, &room, interval).await {
                    BatchVerdict::Offline => {
                        self.wake_waker(room.id()).await;
                        debug!(url = url, "批量检测未开播");
                        tokio::time::sleep(BATCH_ROTATE_SLEEP).await;
                        continue;
                    }
                    // Live / Fallback 都继续走下面的逐间完整检测
                    BatchVerdict::Live | BatchVerdict::Fallback => {}
                }
            }
            let Some(download_permit) = self.try_acquire_download_slot(&room).await else {
                self.wake_waker(room.id()).await;
                tokio::time::sleep(Duration::from_secs(interval)).await;
                continue;
            };
            let request = live_request(&room);
            // 检查直播状态
            match plugin.check_stream(request).await {
                Ok(LiveStatus::Live { stream }) => {
                    let sql_no_id = streamer_info(&stream);
                    let insert = match StreamerInfo::builder()
                        .url(sql_no_id.url.clone())
                        .name(room.live_streamer.remark.clone())
                        .title(sql_no_id.title.clone())
                        .date(sql_no_id.date)
                        .live_cover_path(sql_no_id.live_cover_path.clone())
                        .insert(&self.pool)
                        .await
                    {
                        Ok(insert) => insert,
                        Err(e) => {
                            error!(e=?e, "插入数据库失败");
                            self.wake_waker(room.id()).await;
                            continue;
                        }
                    };
                    info!(url = url, "room: is live -> 开播了");

                    let context = Context::new(insert.id, room.clone(), self.pool.clone(), *stream);
                    let downloader = plugin.clone();
                    let uploader = self.uploader.clone();
                    let rooms_handle = Arc::clone(self);

                    // 只能在已经拿到下载池许可后启动录制。许可移动到任务内并持有到流程结束，
                    // 因此 pool1_size 只在这里表达，不再通过下载 Actor 池或消息队列重复限流。
                    tokio::spawn(async move {
                        let _download_permit = download_permit;
                        start_download_workflow(downloader, context, uploader, rooms_handle).await;
                    });

                    info!("成功开始录制 {}", url);
                }
                Ok(LiveStatus::Offline) => {
                    self.wake_waker(room.id()).await;
                    debug!(url = room.get_streamer().url, "未开播")
                }
                Err(e) => {
                    self.wake_waker(room.id()).await;
                    error!(e=?e, ctx=room.get_streamer().url,"检查直播间出错")
                }
            };
            // 等待下一次检查
            tokio::time::sleep(Duration::from_secs(interval)).await;
        }
        info!("exit -> [{platform_name}]")
    }

    async fn try_acquire_download_slot(&self, room: &Arc<Worker>) -> Option<OwnedSemaphorePermit> {
        match self.download_slots.clone().try_acquire_owned() {
            Ok(permit) => Some(permit),
            Err(_) => {
                debug!(
                    url = room.get_streamer().url,
                    "download pool is full, skip live check"
                );
                None
            }
        }
    }

    /// 用批量检测结果判定单个房间是否开播。
    /// 缓存超过一个检测周期即刷新：以当前房间的客户端/配置为该平台所有房间发起一次批量请求。
    async fn batch_verdict(
        self: &Arc<Self>,
        platform_name: &str,
        plugin: &Arc<dyn LivePlugin + Send + Sync>,
        room: &Arc<Worker>,
        interval: u64,
    ) -> BatchVerdict {
        let url = room.get_streamer().url.clone();

        // 命中未过期缓存直接判定
        if let Some(cache) = self.batch_live.read().unwrap().get(platform_name)
            && cache.refreshed_at.elapsed() < Duration::from_secs(interval)
        {
            return if cache.live_urls.contains(&url) {
                BatchVerdict::Live
            } else {
                BatchVerdict::Offline
            };
        }

        // 缓存过期或缺失：为该平台所有房间发起一次批量检测
        let urls = self.platform_urls(platform_name).await;
        if urls.is_empty() {
            return BatchVerdict::Fallback;
        }
        let request = batch_check_request(room, urls);
        match plugin.batch_check(request).await {
            Ok(live_urls) => {
                let live_urls: HashSet<String> = live_urls.into_iter().collect();
                let is_live = live_urls.contains(&url);
                self.batch_live.write().unwrap().insert(
                    platform_name.to_string(),
                    BatchLiveCache {
                        live_urls,
                        refreshed_at: Instant::now(),
                    },
                );
                if is_live {
                    BatchVerdict::Live
                } else {
                    BatchVerdict::Offline
                }
            }
            Err(e) => {
                // 批量请求失败时回退逐间检测，避免整平台漏检
                warn!(platform = platform_name, e = ?e, "批量检测失败，回退逐间检测");
                BatchVerdict::Fallback
            }
        }
    }

    /// 获取某平台当前队列中所有房间的 URL（用于批量检测）。
    async fn platform_urls(self: &Arc<Self>, platform_name: &str) -> Vec<String> {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::PlatformUrls {
            respond_to: send,
            platform_name: platform_name.to_owned(),
        };
        let _ = self.sender.send(msg).await;
        recv.await.unwrap_or_default()
    }

    /// 添加工作器到房间列表
    ///
    /// # 参数
    /// * `worker` - 要添加的工作器
    pub async fn add(
        self: &Arc<Self>,
        worker: Arc<Worker>,
    ) -> Option<Arc<dyn LivePlugin + Send + Sync>> {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::Add(send, worker.clone());
        let _ = self.sender.send(msg).await;
        let plugin = recv.await.expect("Actor task has been killed")?;

        self.rooms_handle_pool(plugin.clone());
        Some(plugin)
    }

    /// 添加工作器到房间列表
    ///
    /// # 参数
    /// * `worker` - 要添加的工作器
    pub async fn add_plugin(&self, plugin: Arc<dyn LivePlugin + Send + Sync>) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::AddPlugin(send, plugin);
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    /// 删除指定ID的工作器
    ///
    /// # 参数
    /// * `id` - 要删除的工作器ID
    ///
    /// # 返回
    /// 返回剩余工作器数量
    pub async fn del(&self, id: i64) {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::Del {
            respond_to: send,
            id,
        };

        // 忽略发送错误。如果发送失败，下面的recv.await也会失败
        // 没有必要检查两次失败
        let _ = self.sender.send(msg).await;
        if let Some(worker) = recv.await.expect("Actor task has been killed") {
            worker
                .change_status(Stage::Download, WorkerStatus::Idle)
                .await;
        }
    }

    /// 删除指定ID的工作器
    ///
    /// # 参数
    /// * `id` - 要删除的工作器ID
    ///
    /// # 返回
    /// 返回剩余工作器数量
    pub async fn get_worker(&self, id: i64) -> Option<Arc<Worker>> {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::GetWorker {
            respond_to: send,
            id,
        };

        // 忽略发送错误。如果发送失败，下面的recv.await也会失败
        // 没有必要检查两次失败
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    /// 删除指定ID的工作器
    ///
    /// # 参数
    /// * `id` - 要删除的工作器ID
    ///
    /// # 返回
    /// 返回剩余工作器数量
    pub async fn get_all(&self) -> Vec<Arc<Worker>> {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::GetAll { respond_to: send };

        // 忽略发送错误。如果发送失败，下面的recv.await也会失败
        // 没有必要检查两次失败
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    /// 获取下一个要处理的工作器
    ///
    /// # 返回
    /// 返回下一个工作器，如果没有则返回None
    async fn next(self: &Arc<Self>, platform_name: &str) -> Option<Arc<Worker>> {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::NextRoom {
            respond_to: send,
            platform_name: platform_name.to_owned(),
        };

        // 忽略发送错误。如果发送失败，下面的recv.await也会失败
        // 没有必要检查两次失败
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    /// 放回工作队列
    ///
    /// # 参数
    /// * `worker` - 要切换的工作器
    pub async fn wake_waker(
        self: &Arc<Self>,
        id: i64,
    ) -> Option<Arc<dyn LivePlugin + Send + Sync>> {
        let (send, recv) = oneshot::channel();

        let msg = ActorMessage::WakeWaker(send, id);

        // 忽略发送错误
        let _ = self.sender.send(msg).await;
        let plugin = recv.await.expect("Actor task has been killed")?;
        self.rooms_handle_pool(plugin.clone());
        Some(plugin)
    }

    /// 移出工作队列
    ///
    /// # 参数
    /// * `worker` - 要切换的工作器
    pub async fn make_waker(&self, id: i64) {
        let (send, recv) = oneshot::channel();

        let msg = ActorMessage::MakeWaker(send, id);

        // 忽略发送错误
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    fn spawn_monitor_task(
        this: Arc<Self>,
        plugin: Arc<dyn LivePlugin + Send + Sync>,
        platform_name: String,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            this.start_monitor(&platform_name, plugin).await;
        })
    }

    fn rooms_handle_pool(self: &Arc<Self>, plugin: Arc<dyn LivePlugin + Send + Sync>) {
        let platform_name = plugin.name().to_owned();
        match self.monitors.write().unwrap().entry(platform_name.clone()) {
            Entry::Occupied(mut entry) => {
                // 已经有一个任务了，检查是否结束
                if entry.get().is_finished() {
                    // 旧任务已经结束，重新 spawn 一个
                    let handle = Self::spawn_monitor_task(
                        Arc::clone(self),
                        plugin.clone(),
                        platform_name.clone(),
                    );
                    entry.insert(handle); // 替换旧的 JoinHandle
                } else {
                    // 任务还在跑，不做任何事
                }
            }
            Entry::Vacant(entry) => {
                // 没有任务，正常 spawn
                let handle = Self::spawn_monitor_task(
                    Arc::clone(self),
                    plugin.clone(),
                    platform_name.clone(),
                );
                entry.insert(handle);
            }
        }
    }
}

/// Actor消息枚举
/// 定义RoomsActor可以处理的消息类型
enum ActorMessage {
    /// 获取下一个房间
    NextRoom {
        respond_to: oneshot::Sender<Option<Arc<Worker>>>,
        platform_name: String,
    },
    /// 获取某平台所有房间的 URL（用于批量检测）
    PlatformUrls {
        respond_to: oneshot::Sender<Vec<String>>,
        platform_name: String,
    },
    /// 添加工作器
    Add(
        oneshot::Sender<Option<Arc<dyn LivePlugin + Send + Sync>>>,
        Arc<Worker>,
    ),
    /// 添加工作器
    AddPlugin(oneshot::Sender<()>, Arc<dyn LivePlugin + Send + Sync>),
    /// 删除工作器
    Del {
        respond_to: oneshot::Sender<Option<Arc<Worker>>>,
        id: i64,
    },
    /// 查找
    GetWorker {
        respond_to: oneshot::Sender<Option<Arc<Worker>>>,
        id: i64,
    },
    /// 查找所有
    GetAll {
        respond_to: oneshot::Sender<Vec<Arc<Worker>>>,
    },
    /// 放回工作队列
    WakeWaker(
        oneshot::Sender<Option<Arc<dyn LivePlugin + Send + Sync>>>,
        i64,
    ),
    /// 移出工作队列
    MakeWaker(oneshot::Sender<()>, i64),
    Shutdown,
}

/// 房间Actor
/// 管理房间列表的内部Actor
/// 平台名称
//     name: String,
struct RoomsActor {
    /// 消息接收器
    receiver: tokio::sync::mpsc::Receiver<ActorMessage>,
    /// 活跃房间列表
    platforms: HashMap<String, VecDeque<Arc<Worker>>>,
    /// 当前索引
    /// 等待房间列表
    all_workers: Vec<Arc<Worker>>,
    // index: usize,
    // rooms: Vec<Arc<Worker>>,
    // waiting: Vec<Arc<Worker>>,
    /// 下载插件
    plugins: Vec<Arc<dyn LivePlugin + Send + Sync>>,
}

impl RoomsActor {
    /// 创建新的房间Actor实例
    fn new(receiver: tokio::sync::mpsc::Receiver<ActorMessage>) -> Self {
        Self {
            receiver,
            // index: 0,
            platforms: Default::default(),
            all_workers: Default::default(),
            plugins: Vec::new(),
        }
    }

    /// 运行Actor主循环
    /// 处理接收到的消息
    async fn run(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                ActorMessage::NextRoom {
                    respond_to,
                    platform_name,
                } => {
                    // `let _ =` 忽略发送时的任何错误
                    // 如果使用`select!`宏取消等待响应，可能会发生这种情况
                    let _ = respond_to.send(self.next(&platform_name));
                }
                ActorMessage::PlatformUrls {
                    respond_to,
                    platform_name,
                } => {
                    // `let _ =` 忽略发送时的任何错误
                    let _ = respond_to.send(self.platform_urls(&platform_name));
                }
                ActorMessage::Add(respond_to, worker) => {
                    let plugin = self.add(worker);
                    let _ = respond_to.send(plugin);
                }
                ActorMessage::Del { respond_to, id } => {
                    // `let _ =` 忽略发送时的任何错误
                    // 如果使用`select!`宏取消等待响应，可能会发生这种情况

                    let _ = respond_to.send(self.del(id).await);
                }
                ActorMessage::WakeWaker(sender, id) => {
                    // `let _ =` 忽略发送时的任何错误
                    let _ = sender.send(self.push_back(id));
                }
                ActorMessage::Shutdown => {
                    return;
                }
                ActorMessage::GetWorker { respond_to, id } => {
                    let option = self.get_worker(id);
                    // `let _ =` 忽略发送时的任何错误
                    let _ = respond_to.send(option);
                }
                ActorMessage::GetAll { respond_to } => {
                    // `let _ =` 忽略发送时的任何错误
                    let _ = respond_to.send(self.get_all());
                }
                ActorMessage::MakeWaker(respond_to, id) => {
                    self.pop(id);
                    // `let _ =` 忽略发送时的任何错误
                    let _ = respond_to.send(());
                }
                ActorMessage::AddPlugin(respond_to, plugin) => {
                    self.add_plugin(plugin);
                    // `let _ =` 忽略发送时的任何错误
                    let _ = respond_to.send(());
                }
            }
        }
        info!("Rooms actor terminated");
    }

    fn add(&mut self, worker: Arc<Worker>) -> Option<Arc<dyn LivePlugin + Send + Sync>> {
        let plugin = self.matches(&worker.live_streamer.url)?;
        let platform_name = plugin.name().to_owned();
        self.all_workers.push(worker.clone());

        match self.platforms.entry(platform_name) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().push_back(worker.clone());
                // entry.remove(); // 可以删除
            }
            Entry::Vacant(entry) => {
                entry.insert(VecDeque::from([worker.clone()])); // 插入新值
            }
        }
        debug!("Added room [{}]", worker.live_streamer.url);
        Some(plugin)
    }

    fn add_plugin(&mut self, plugin: Arc<dyn LivePlugin + Send + Sync>) {
        self.plugins.push(plugin);
        debug!("Added plugin size[{}]", self.plugins.len());
    }

    fn get_worker(&mut self, id: i64) -> Option<Arc<Worker>> {
        self.all_workers
            .iter()
            .find(|worker| worker.id() == id)
            .cloned()
    }

    fn get_all(&mut self) -> Vec<Arc<Worker>> {
        reuse_vec_arc(&mut self.all_workers.iter())
    }

    /// 获取下一个工作器（循环遍历）
    fn next(&mut self, platform_name: &str) -> Option<Arc<Worker>> {
        // 如果内部Vec是空的，迭代结束（虽然是循环迭代器，但空集合无法产生任何值）
        let arc = self.platforms.get_mut(platform_name)?.pop_front()?;

        *arc.downloader_status.write().unwrap() = WorkerStatus::Pending;

        Some(arc)
    }

    /// 获取某平台所有房间的 URL（含正在检测、已弹出队列的房间）。
    /// 以 all_workers 为源按插件归属过滤，保证覆盖整平台而非仅队列内房间。
    fn platform_urls(&self, platform_name: &str) -> Vec<String> {
        self.all_workers
            .iter()
            .filter(|worker| {
                self.matches(&worker.live_streamer.url)
                    .is_some_and(|plugin| plugin.name() == platform_name)
            })
            .map(|worker| worker.live_streamer.url.clone())
            .collect()
    }

    /// 放回工作队列
    fn push_back(&mut self, id: i64) -> Option<Arc<dyn LivePlugin + Send + Sync>> {
        // 在总数组中找不到，说明该房间已被移除我们也不放回
        let worker = self.get_worker(id)?;
        if let WorkerStatus::Pause = *worker.downloader_status.write().unwrap() {
            // 暂停状态则不放回
            warn!("Paused room [{}]", worker.live_streamer.url);
            return None;
        }
        for (name, queue) in self.platforms.iter_mut() {
            if queue.iter().any(|w| w.id() == id) {
                // 说明找到了已经入队的房间，则是更新的情况
                warn!(name = name, "房间已更新无需入队");
                return None;
            }
        }

        let plugin = self.matches(&worker.live_streamer.url)?;
        self.platforms
            .get_mut(plugin.name())?
            .push_back(worker.clone());
        *worker.downloader_status.write().unwrap() = WorkerStatus::Idle;
        Some(plugin)
    }

    /// 移出工作队列
    fn pop(&mut self, id: i64) {
        for (_name, queue) in self.platforms.iter_mut() {
            if let Some(pos) = queue.iter().position(|w| w.id() == id) {
                queue.remove(pos); // 只删掉这个队列中第一个匹配的 worker
                return;
            }
        }
        warn!("移出工作队列 failed: No room found with id {}", id);
    }

    /// 删除指定ID的工作器
    async fn del(&mut self, id: i64) -> Option<Arc<Worker>> {
        let worker = self.get_worker(id)?;
        let plugin = self.matches(&worker.live_streamer.url)?;
        let platform_name = plugin.name();
        // 从 platforms 中删除
        if let Some(workers) = self.platforms.get_mut(platform_name) {
            workers.retain(|w| w.id() != id);
        } else {
            error!("Removed room [{:?}] {}", platform_name, id);
        }

        // 从 all_workers 中删除
        self.all_workers.retain(|w| w.id() != id);

        debug!("del worker size[{}]", self.all_workers.len());
        Some(worker)
    }

    /// 检查URL是否匹配此下载管理器的插件
    ///
    /// # 参数
    /// * `url` - 要检查的URL
    ///
    /// # 返回
    /// 如果URL匹配返回true，否则返回false
    pub fn matches(&self, url: &str) -> Option<Arc<dyn LivePlugin + Send + Sync>> {
        for plugin in &self.plugins {
            trace!(
                platform_name = plugin.name(),
                url = url,
                "Found plugin for URL"
            );
            if plugin.matches(url) {
                return Some(plugin.clone());
            }
        }
        None
    }
}

fn reuse_vec_arc<'a, T: 'a, U: Iterator<Item = &'a Arc<T>>>(v: &mut U) -> Vec<Arc<T>> {
    v.into_iter().cloned().collect()
}
