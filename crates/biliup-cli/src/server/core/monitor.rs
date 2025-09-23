use crate::server::core::download_manager::{ActorHandle, DownloaderMessage};
use crate::server::core::plugin::{DownloadPlugin, StreamStatus};
use crate::server::infrastructure::context::{Context, Stage, Worker, WorkerStatus};
use async_channel::{Receiver, Sender, bounded};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::info;

/// 启动客户端监控循环
///
/// # 参数
/// * `rooms_handle` - 房间处理器
/// * `plugin` - 下载插件
/// * `actor_handle` - Actor处理器
/// * `interval` - 监控间隔（秒）
async fn start_client(
    rooms_handle: Arc<RoomsHandle>,
    plugin: Arc<dyn DownloadPlugin + Send + Sync>,
    actor_handle: Arc<ActorHandle>,
    mut interval: u64,
) {
    let platform_name = &rooms_handle.name;
    info!("start -> [{platform_name}]");
    loop {
        // 获取下一个要检查的房间
        if let Some(room) = rooms_handle.next().await {
            let ulr = room.get_streamer().url;
            interval = room.get_config().event_loop_interval;
            info!("[{platform_name}] room: {ulr}");
            let mut ctx = Context::new(room.clone());
            // 检查直播状态
            match plugin.check_status(&mut ctx).await.unwrap() {
                StreamStatus::Live { stream_info } => {
                    info!("room: {ulr} is live -> {:?}", stream_info);
                    // 更新状态为等待中
                    room.change_status(Stage::Download, WorkerStatus::Pending);
                    // 发送下载开始消息
                    if actor_handle
                        .down_sender
                        .send(DownloaderMessage::Start(
                            plugin.clone(),
                            stream_info,
                            ctx,
                            rooms_handle.clone(),
                        ))
                        .await
                        .is_ok()
                    {
                        // 切换房间状态
                        rooms_handle.toggle(room).await;
                    }
                }
                StreamStatus::Offline => {}
                StreamStatus::Unknown => {}
            };
        }
        // 等待下一次检查
        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
}

/// 监控器
/// 负责监控直播间状态并管理下载任务
#[derive(Debug)]
pub struct Monitor {
    /// 房间处理器
    pub rooms_handle: Arc<RoomsHandle>,
    /// 监控任务句柄
    kill: JoinHandle<()>,
}

impl Monitor {
    /// 创建新的监控器实例
    ///
    /// # 参数
    /// * `plugin` - 下载插件
    /// * `actor_handle` - Actor处理器
    pub fn new(
        plugin: Arc<dyn DownloadPlugin + Send + Sync>,
        actor_handle: Arc<ActorHandle>,
    ) -> Self {
        // 创建房间处理器
        let handle = Arc::new(RoomsHandle::new(plugin.name()));
        // 启动监控任务
        let join_handle = tokio::spawn({
            let handle = Arc::clone(&handle);
            async move {
                start_client(handle, plugin, actor_handle, 10).await;
            }
        });
        Self {
            rooms_handle: Arc::clone(&handle),
            kill: join_handle,
        }
    }
}

impl Drop for Monitor {
    /// 监控器销毁时的清理逻辑
    fn drop(&mut self) {
        // 终止监控任务
        self.kill.abort();
        self.rooms_handle.kill.abort();
        info!("Monitor [{}] killed", self.rooms_handle.name)
    }
}

/// 房间处理器
/// 管理多个直播间的状态和操作
#[derive(Debug)]
pub struct RoomsHandle {
    /// 平台名称
    name: String,
    /// 消息发送器
    sender: Sender<ActorMessage>,
    /// Actor任务句柄
    kill: JoinHandle<()>,
}

impl RoomsHandle {
    /// 创建新的房间处理器实例
    ///
    /// # 参数
    /// * `name` - 平台名称
    pub fn new(name: &str) -> Self {
        // 创建消息通道
        let (sender, receiver) = bounded(1);
        let mut actor = RoomsActor::new(receiver);
        // 启动Actor任务
        let kill = tokio::spawn(async move { actor.run().await });

        Self {
            sender,
            kill,
            name: name.to_string(),
        }
    }

    /// 添加工作器到房间列表
    ///
    /// # 参数
    /// * `worker` - 要添加的工作器
    pub async fn add(&self, worker: Arc<Worker>) {
        let msg = ActorMessage::Add(worker);
        let _ = self.sender.send(msg).await;
    }

    /// 删除指定ID的工作器
    ///
    /// # 参数
    /// * `id` - 要删除的工作器ID
    ///
    /// # 返回
    /// 返回剩余工作器数量
    pub async fn del(&self, id: i64) -> usize {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::Del {
            respond_to: send,
            id,
        };

        // 忽略发送错误。如果发送失败，下面的recv.await也会失败
        // 没有必要检查两次失败
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    /// 获取下一个要处理的工作器
    ///
    /// # 返回
    /// 返回下一个工作器，如果没有则返回None
    async fn next(&self) -> Option<Arc<Worker>> {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::NextRoom { respond_to: send };

        // 忽略发送错误。如果发送失败，下面的recv.await也会失败
        // 没有必要检查两次失败
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    /// 切换工作器状态（在活跃和等待列表之间）
    ///
    /// # 参数
    /// * `worker` - 要切换的工作器
    pub async fn toggle(&self, worker: Arc<Worker>) {
        let msg = ActorMessage::Toggle(worker.clone());

        // 忽略发送错误
        let _ = self.sender.send(msg).await;
    }
}

/// Actor消息枚举
/// 定义RoomsActor可以处理的消息类型
enum ActorMessage {
    /// 获取下一个房间
    NextRoom {
        respond_to: oneshot::Sender<Option<Arc<Worker>>>,
    },
    /// 添加工作器
    Add(Arc<Worker>),
    /// 删除工作器
    Del {
        respond_to: oneshot::Sender<usize>,
        id: i64,
    },
    /// 切换工作器状态
    Toggle(Arc<Worker>),
}

/// 房间Actor
/// 管理房间列表的内部Actor
struct RoomsActor {
    /// 消息接收器
    receiver: Receiver<ActorMessage>,
    /// 当前索引
    index: usize,
    /// 活跃房间列表
    rooms: Vec<Arc<Worker>>,
    /// 等待房间列表
    waiting: Vec<Arc<Worker>>,
}

impl RoomsActor {
    /// 创建新的房间Actor实例
    fn new(receiver: Receiver<ActorMessage>) -> Self {
        Self {
            receiver,
            index: 0,
            rooms: Vec::new(),
            waiting: Vec::new(),
        }
    }

    /// 运行Actor主循环
    async fn run(&mut self) {
        while let Ok(msg) = self.receiver.recv().await {
            self.handle_message(msg);
        }
    }

    /// 处理接收到的消息
    fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::NextRoom { respond_to } => {
                // `let _ =` 忽略发送时的任何错误
                // 如果使用`select!`宏取消等待响应，可能会发生这种情况
                let _ = respond_to.send(self.next());
            }
            ActorMessage::Add(worker) => {
                self.rooms.push(worker);
                info!("Added room [{:?}]", self.rooms);
            }
            ActorMessage::Del { respond_to, id } => {
                // `let _ =` 忽略发送时的任何错误
                // 如果使用`select!`宏取消等待响应，可能会发生这种情况
                let _ = respond_to.send(self.del(id));
            }
            ActorMessage::Toggle(worker) => {
                self.toggle_keep_order(&worker);
            }
        }
    }

    /// 获取下一个工作器（循环遍历）
    fn next(&mut self) -> Option<Arc<Worker>> {
        // 如果内部Vec是空的，迭代结束（虽然是循环迭代器，但空集合无法产生任何值）
        if self.rooms.is_empty() {
            return None;
        }

        // 获取当前位置元素的克隆
        // 使用.get()并.cloned()是安全的做法
        let item = self.rooms[self.index].clone();

        // 更新index以便下一次调用，使用取模运算实现循环
        self.index = (self.index + 1) % self.rooms.len();

        Some(item)
    }

    /// 删除指定ID的工作器
    fn del(&mut self, id: i64) -> usize {
        // 从活跃房间列表中删除
        if let Some(i) = self.rooms.iter().position(|x| x.live_streamer.id == id) {
            self.rooms.remove(i); // 保序，但O(n)
        } else if let Some(i) = self.waiting.iter().position(|x| x.live_streamer.id == id) {
            // 从等待房间列表中删除
            self.waiting.swap_remove(i);
        };
        info!("Removed room [{:?}] {}", self.rooms, self.rooms.len());
        info!("Deleting room [{:?}] {}", self.waiting, self.waiting.len());
        self.rooms.len() + self.waiting.len()
    }

    /// 切换工作器状态，保持顺序
    fn toggle_keep_order(&mut self, worker: &Arc<Worker>) -> bool {
        // 从活跃列表移动到等待列表
        if let Some(i) = self.rooms.iter().position(|x| x == worker) {
            let val = self.rooms.remove(i); // 保序，但O(n)
            self.waiting.push(val);
            true
        } else if let Some(i) = self.waiting.iter().position(|x| x == worker) {
            // 从等待列表移动到活跃列表
            let val = self.waiting.swap_remove(i);
            self.rooms.push(val);
            true
        } else {
            false
        }
    }
}
