use crate::server::core::download_manager::{ActorHandle, DownloaderMessage};
use crate::server::core::plugin::{DownloadPlugin, StreamStatus};
use crate::server::infrastructure::context::{Worker, WorkerStatus};
use async_channel::{Receiver, Sender, bounded};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::info;

async fn start_client(
    rooms_handle: Arc<RoomsHandle>,
    plugin: Arc<dyn DownloadPlugin + Send + Sync>,
    actor_handle: Arc<ActorHandle>,
    mut interval: u64,
) {
    let platform_name = &rooms_handle.name;
    info!("start -> [{platform_name}]");
    loop {
        if let Some(room) = rooms_handle.next().await {
            let ulr = room.get_streamer().await.unwrap().url;
            interval = room.get_config().await.unwrap().event_loop_interval;
            info!("[{platform_name}] room: {ulr}");
            match plugin.check_status(&ulr).await.unwrap() {
                StreamStatus::Live { stream_info } => {
                    info!("room: {ulr} is live -> {:?}", stream_info);
                    *room.downloader_status.write().unwrap() = WorkerStatus::Pending;
                    if actor_handle
                        .down_sender
                        .send(DownloaderMessage::Start(
                            plugin.clone(),
                            stream_info,
                            room.clone(),
                            rooms_handle.clone(),
                        ))
                        .await
                        .is_ok()
                    {
                        rooms_handle.toggle(room).await;
                    }
                }
                StreamStatus::Offline => {}
                StreamStatus::Unknown => {}
            };
        }
        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
}

#[derive(Debug)]
pub struct Monitor {
    pub rooms_handle: Arc<RoomsHandle>,
    kill: JoinHandle<()>,
}

impl Monitor {
    pub fn new(
        plugin: Arc<dyn DownloadPlugin + Send + Sync>,
        actor_handle: Arc<ActorHandle>,
    ) -> Self {
        let handle = Arc::new(RoomsHandle::new(plugin.name()));
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
    fn drop(&mut self) {
        self.kill.abort();
        self.rooms_handle.kill.abort();
        info!("Monitor [{}] killed", self.rooms_handle.name)
    }
}

#[derive(Debug)]
pub struct RoomsHandle {
    name: String,
    sender: Sender<ActorMessage>,
    kill: JoinHandle<()>,
}

impl RoomsHandle {
    pub fn new(name: &str) -> Self {
        let (sender, receiver) = bounded(1);
        let mut actor = RoomsActor::new(receiver);
        let kill = tokio::spawn(async move { actor.run().await });

        Self {
            sender,
            kill,
            name: name.to_string(),
        }
    }

    pub async fn add(&self, worker: Arc<Worker>) {
        let msg = ActorMessage::Add(worker);
        let _ = self.sender.send(msg).await;
    }

    pub async fn del(&self, id: i64) -> usize {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::Del {
            respond_to: send,
            id,
        };

        // Ignore send errors. If this send fails, so does the
        // recv.await below. There's no reason to check the
        // failure twice.
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    async fn next(&self) -> Option<Arc<Worker>> {
        let (send, recv) = oneshot::channel();
        let msg = ActorMessage::NextRoom { respond_to: send };

        // Ignore send errors. If this send fails, so does the
        // recv.await below. There's no reason to check the
        // failure twice.
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    pub async fn toggle(&self, worker: Arc<Worker>) {
        let msg = ActorMessage::Toggle(worker.clone());

        // Ignore send errors. If this send fails, so does the
        // recv.await below. There's no reason to check the
        // failure twice.
        let _ = self.sender.send(msg).await;
    }
}

enum ActorMessage {
    NextRoom {
        respond_to: oneshot::Sender<Option<Arc<Worker>>>,
    },
    Add(Arc<Worker>),
    Del {
        respond_to: oneshot::Sender<usize>,
        id: i64,
    },
    Toggle(Arc<Worker>),
}

struct RoomsActor {
    receiver: Receiver<ActorMessage>,
    index: usize,
    rooms: Vec<Arc<Worker>>,
    waiting: Vec<Arc<Worker>>,
}

impl RoomsActor {
    fn new(receiver: Receiver<ActorMessage>) -> Self {
        Self {
            receiver,
            index: 0,
            rooms: Vec::new(),
            waiting: Vec::new(),
        }
    }

    async fn run(&mut self) {
        while let Ok(msg) = self.receiver.recv().await {
            self.handle_message(msg);
        }
    }

    fn handle_message(&mut self, msg: ActorMessage) {
        match msg {
            ActorMessage::NextRoom { respond_to } => {
                // The `let _ =` ignores any errors when sending.
                //
                // This can happen if the `select!` macro is used
                // to cancel waiting for the response.
                let _ = respond_to.send(self.next());
            }
            ActorMessage::Add(worker) => {
                self.rooms.push(worker);
                info!("Added room [{:?}]", self.rooms);
            }
            ActorMessage::Del { respond_to, id } => {
                // The `let _ =` ignores any errors when sending.
                //
                // This can happen if the `select!` macro is used
                // to cancel waiting for the response.
                let _ = respond_to.send(self.del(id));
            }
            ActorMessage::Toggle(worker) => {
                self.toggle_keep_order(&worker);
            }
        }
    }

    fn next(&mut self) -> Option<Arc<Worker>> {
        // 如果内部 Vec 是空的，迭代结束（虽然是循环迭代器，但空集合无法产生任何值）
        if self.rooms.is_empty() {
            return None;
        }

        // 获取当前位置元素的克隆
        // 使用 .get() 并 .cloned() 是安全的做法
        let item = self.rooms[self.index].clone();

        // 更新 index 以便下一次调用，使用取模运算实现循环
        self.index = (self.index + 1) % self.rooms.len();

        Some(item)
    }

    fn del(&mut self, id: i64) -> usize {
        if let Some(i) = self.rooms.iter().position(|x| x.id == id) {
            self.rooms.remove(i); // 保序，但 O(n)
        } else if let Some(i) = self.waiting.iter().position(|x| x.id == id) {
            self.waiting.swap_remove(i);
        };
        info!("Removed room [{:?}] {}", self.rooms, self.rooms.len());
        info!("Deleting room [{:?}] {}", self.waiting, self.waiting.len());
        self.rooms.len() + self.waiting.len()
    }

    fn toggle_keep_order(&mut self, worker: &Arc<Worker>) -> bool {
        if let Some(i) = self.rooms.iter().position(|x| x == worker) {
            let val = self.rooms.remove(i); // 保序，但 O(n)
            self.waiting.push(val);
            true
        } else if let Some(i) = self.waiting.iter().position(|x| x == worker) {
            let val = self.waiting.swap_remove(i);
            self.rooms.push(val);
            true
        } else {
            false
        }
    }
}
