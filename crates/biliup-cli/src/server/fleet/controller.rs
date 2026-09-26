//! 控制面：接受节点的 iroh 连接，维护在线状态与最近 5 分钟的曲线，按分派给节点下发期望状态。

mod configuration;
mod dispatch;

pub use configuration::{CONTROLLER_VERSION, NodeConfigState, version_older};
pub use dispatch::{CreateRoom, DispatchError, RoomStatus, RoomView, UpdateRoom, strip_hooks};

use super::assignments;
use super::config_store;
use super::model::Account;
use super::protocol::{
    self, Ack, CONFIG_SINCE, CloseCode, ConfigAck, ControllerMessage, DESIRED_STATE_SINCE,
    DesiredState, Heartbeat, Hello, NodeMessage, OFFLINE_AFTER, Summary, Tools,
};
use super::relay::EmbeddedRelay;
use super::store::{self, NodeRow, Redeem};
use super::{bind_endpoint, close_with, now_ms};
use crate::server::common::system_stats::Sample;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use error_stack::ResultExt;
use iroh::endpoint::{Connection, Incoming, RecvStream, SendStream};
use iroh::{Endpoint, EndpointId, RelayConfig, RelayMap, SecretKey};
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::{debug, info, warn};
use url::Url;

/// 心跳曲线在内存里留多久，与 `SystemMonitor` 同口径
const CURVE_WINDOW_MS: i64 = 5 * 60 * 1000;
/// 在线节点的 `last_seen_at` / `last_summary` 至多这么久落一次盘
const PERSIST_EVERY_MS: i64 = 60 * 1000;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);
const FRAME_LOG_TARGET: &str = "biliup_cli::server::fleet::frames";

/// 控制面给出的 relay 地址来源
#[derive(Debug, Clone)]
pub struct RelaySetup {
    /// 控制面自己连的 relay
    pub local: Vec<Url>,
    /// 写进票据、下发给节点的 relay；为空表示按网卡地址现列
    pub advertised: Vec<Url>,
    /// 内嵌 relay 监听的端口；用外部 relay 时为 `None`
    pub embedded_port: Option<u16>,
}

struct LiveNode {
    seq: u64,
    connection: Connection,
    connected_at: i64,
    last_message_at: i64,
    last_persisted_at: i64,
    version: String,
    /// `Hello` 里的协议次版本号
    proto: u32,
    tools: Option<Tools>,
    summary: Option<Summary>,
    interval_ms: u64,
    samples: VecDeque<Sample>,
    rooms: Vec<serde_json::Value>,
    /// 发往这条连接的帧；写帧的循环在 `session` 里
    outbox: mpsc::UnboundedSender<ControllerMessage>,
    /// 最近一次下发的期望状态版本号
    pushed_version: Option<u64>,
    /// 节点最近一次 `Ack` 的版本号、持有的房间（id → epoch）与落地失败的房间
    acked_version: Option<u64>,
    held: HashMap<i64, i64>,
    failed: HashMap<i64, String>,
    /// 最近一次 `Ack` 里的配置应答（次版本 ≥ 2）
    config_ack: Option<ConfigAck>,
}

impl LiveNode {
    fn accepts_desired_state(&self) -> bool {
        self.proto >= DESIRED_STATE_SINCE
    }

    fn apply_ack(&mut self, ack: &Ack, now: i64) {
        self.acked_version = Some(ack.version);
        self.held = ack.held.iter().map(|room| (room.id, room.epoch)).collect();
        self.failed = ack
            .failed
            .iter()
            .map(|room| (room.id, room.error.clone()))
            .collect();
        self.config_ack = ack.config.clone();
        self.last_message_at = now;
    }

    fn apply(&mut self, heartbeat: Heartbeat, now: i64) {
        self.summary = Some(Summary::of(&heartbeat));
        self.interval_ms = heartbeat.stats.interval_ms;
        let newest = self.samples.back().map_or(i64::MIN, |sample| sample.ts);
        self.samples.extend(
            heartbeat
                .stats
                .samples
                .into_iter()
                .filter(|sample| sample.ts > newest),
        );
        // 采样时刻是节点的时钟，按最新一条往前截，不受两台机器的时钟差影响
        if let Some(latest) = self.samples.back().map(|sample| sample.ts) {
            while self
                .samples
                .front()
                .is_some_and(|sample| sample.ts <= latest - CURVE_WINDOW_MS)
            {
                self.samples.pop_front();
            }
        }
        self.rooms = heartbeat.rooms;
        self.last_message_at = now;
    }

    fn online(&self, now: i64) -> bool {
        now - self.last_message_at < OFFLINE_AFTER.as_millis() as i64
    }

    fn path(&self) -> Option<&'static str> {
        let paths = self.connection.paths();
        let selected = paths.iter().find(|path| path.is_selected())?;
        Some(if selected.is_relay() {
            "relay"
        } else {
            "direct"
        })
    }
}

/// `GET /v1/fleet/nodes` 里的一台节点
#[derive(Debug, Serialize)]
pub struct NodeView {
    pub id: i64,
    pub name: String,
    pub endpoint_id: String,
    pub labels: serde_json::Value,
    pub allow_hooks: bool,
    pub created_at: i64,
    pub last_seen_at: Option<i64>,
    pub version: Option<String>,
    /// 在线时 `Hello` 里的协议次版本号；低于 1 的节点收不了房间
    pub proto: Option<u32>,
    /// 在线时节点上外部工具的可用情况
    pub tools: Option<Tools>,
    pub online: bool,
    pub connected_at: Option<i64>,
    /// 当前走的路径：`relay` 或 `direct`；离线为 `null`
    pub path: Option<&'static str>,
    /// 在线时是最新心跳的摘要，离线时是最后落盘的摘要
    pub summary: Option<Summary>,
    pub interval_ms: Option<u64>,
    /// 最近 5 分钟的采样（节点时钟）；离线为空
    pub samples: Vec<Sample>,
    /// 分派给它的房间数（含迁移中、还没交给它的）
    pub assigned_rooms: i64,
    /// 节点上报的 B 站账号（只有 mid 与昵称）
    pub accounts: Vec<Account>,
    /// 最近一次下发的期望状态它是否已经确认
    pub synced: Option<bool>,
    /// 配置的同步情况、版本是否过旧与覆盖了哪些键
    pub config: NodeConfigState,
}

pub struct Controller {
    pool: ConnectionPool,
    endpoint: Endpoint,
    relays: RelaySetup,
    relay: tokio::sync::Mutex<Option<EmbeddedRelay>>,
    live: Mutex<HashMap<i64, LiveNode>>,
    next_seq: AtomicU64,
    accept_task: Mutex<Option<JoinHandle<()>>>,
    /// 期望状态版本号：毫秒时钟与「上一个 + 1」取大，重启后也不会倒退
    version: AtomicU64,
    /// 改分派与下发期望状态互斥：同一台节点收到的快照按版本号递增，改动之后生成的快照一定看得到改动
    dispatch: tokio::sync::Mutex<()>,
}

impl Controller {
    pub async fn start(
        pool: ConnectionPool,
        secret: SecretKey,
        relays: RelaySetup,
        relay: Option<EmbeddedRelay>,
    ) -> AppResult<Arc<Self>> {
        let map = RelayMap::from_iter(
            relays
                .local
                .iter()
                .map(|url| RelayConfig::new(url.clone().into(), None)),
        );
        let endpoint = bind_endpoint(secret, map, true)
            .await
            .attach("could not bind the fleet controller endpoint")?;
        info!(
            endpoint_id = %endpoint.id(),
            relays = ?relays.local.iter().map(Url::as_str).collect::<Vec<_>>(),
            "fleet controller endpoint ready"
        );
        let controller = Arc::new(Controller {
            pool,
            endpoint,
            relays,
            relay: tokio::sync::Mutex::new(relay),
            live: Mutex::default(),
            next_seq: AtomicU64::new(1),
            accept_task: Mutex::default(),
            version: AtomicU64::new(0),
            dispatch: tokio::sync::Mutex::new(()),
        });
        let task = tokio::spawn(accept_loop(
            Arc::downgrade(&controller),
            controller.endpoint.clone(),
        ));
        *controller.accept_task.lock().unwrap() = Some(task);
        Ok(controller)
    }

    pub fn endpoint_id(&self) -> EndpointId {
        self.endpoint.id()
    }

    pub fn pool(&self) -> &ConnectionPool {
        &self.pool
    }

    /// 写进票据、下发给节点的 relay 地址
    pub fn advertised_relays(&self) -> Vec<Url> {
        if !self.relays.advertised.is_empty() {
            return self.relays.advertised.clone();
        }
        match self.relays.embedded_port {
            Some(port) => super::net::interface_relay_urls(port),
            None => self.relays.local.clone(),
        }
    }

    /// 内嵌 relay 的端口；界面拿它和浏览器地址栏里的主机名拼候选地址
    pub fn relay_port(&self) -> Option<u16> {
        self.relays.embedded_port
    }

    /// 写进票据的 relay：`--relay-url` 在前，然后是界面补充的地址（`extra`），
    /// 没有 `--relay-url` 时最后是按网卡地址列的
    pub fn ticket_relays(&self, extra: &[Url]) -> Vec<Url> {
        let mut relays = self.relays.advertised.clone();
        let rest = if self.relays.advertised.is_empty() {
            self.advertised_relays()
        } else {
            Vec::new()
        };
        for url in extra.iter().chain(&rest) {
            if !relays.contains(url) {
                relays.push(url.clone());
            }
        }
        relays
    }

    fn advertised_strings(&self) -> Vec<String> {
        self.advertised_relays()
            .into_iter()
            .map(|url| url.to_string())
            .collect()
    }

    pub async fn nodes(&self) -> AppResult<Vec<NodeView>> {
        let rows = store::list_nodes(&self.pool).await?;
        let counts = assignments::assigned_counts(&self.pool).await?;
        let mut overrides = config_store::node_overrides(&self.pool).await?;
        let mut accounts: HashMap<i64, Vec<Account>> = HashMap::new();
        for account in assignments::list_accounts(&self.pool).await? {
            accounts.entry(account.node_id).or_default().push(Account {
                mid: account.mid,
                uname: account.uname,
            });
        }
        let now = now_ms();
        let live = self.live.lock().unwrap();
        Ok(rows
            .into_iter()
            .map(|row| {
                let id = row.id;
                let mut view = view(row, live.get(&id), now);
                view.assigned_rooms = counts.get(&id).copied().unwrap_or(0);
                view.accounts = accounts.remove(&id).unwrap_or_default();
                view.config.override_keys = overrides
                    .remove(&id)
                    .map(|patch| patch.into_iter().map(|(key, _)| key).collect())
                    .unwrap_or_default();
                view
            })
            .collect())
    }

    /// 移除节点：吊销公钥，在线的连接当场以 `revoked` 关闭。
    /// 分派给它的房间变成未分派；它自己按约定把这些房间转成本地房间继续录。
    pub async fn revoke(&self, id: i64) -> AppResult<bool> {
        let guard = self.dispatch.lock().await;
        let Some(endpoint_id) = store::revoke_node(&self.pool, id, now_ms()).await? else {
            return Ok(false);
        };
        if let Some(node) = self.live.lock().unwrap().remove(&id) {
            close_with(&node.connection, CloseCode::Revoked);
        }
        let affected = assignments::unassign_node(&self.pool, id, now_ms()).await?;
        drop(guard);
        info!(node = id, %endpoint_id, rooms = affected.len(), "fleet node revoked");
        // 等它释放的房间不再等，交给各自的新节点
        self.push_all().await;
        Ok(true)
    }

    fn next_version(&self) -> u64 {
        let now = u64::try_from(now_ms()).unwrap_or_default();
        let mut current = self.version.load(Ordering::Relaxed);
        loop {
            let next = now.max(current + 1);
            match self.version.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return next,
                Err(actual) => current = actual,
            }
        }
    }

    /// 改分派时记下的版本号：之后下发的快照版本号都比它大
    fn current_version(&self) -> i64 {
        i64::try_from(self.version.load(Ordering::Relaxed)).unwrap_or(i64::MAX)
    }

    /// 给一台在线节点下发它此刻完整的期望状态。离线或太旧（收不了 `DesiredState`）的节点跳过，
    /// 它连上来时会收到。
    pub async fn push(&self, node: i64) {
        let _guard = self.dispatch.lock().await;
        self.push_locked(node).await;
    }

    async fn push_locked(&self, node: i64) {
        let proto = self
            .live
            .lock()
            .unwrap()
            .get(&node)
            .filter(|live| live.accepts_desired_state())
            .map(|live| live.proto);
        let Some(proto) = proto else {
            return;
        };
        let version = self.next_version();
        let (rooms, templates) = match assignments::desired_state(&self.pool, node).await {
            Ok(desired) => desired,
            Err(e) => {
                warn!(node, error = ?e, "could not build the desired state");
                return;
            }
        };
        // 次版本 1 的节点照常收房间，配置不发
        let config = if proto >= CONFIG_SINCE {
            match self.desired_config(node).await {
                Ok(config) => Some(config),
                Err(e) => {
                    warn!(node, error = ?e, "could not build the desired config");
                    return;
                }
            }
        } else {
            None
        };
        let message = ControllerMessage::DesiredState(DesiredState {
            version,
            rooms,
            templates,
            config,
        });
        if let Some(live) = self.live.lock().unwrap().get_mut(&node)
            && live.outbox.send(message).is_ok()
        {
            live.pushed_version = Some(version);
        }
    }

    pub async fn push_many(&self, nodes: impl IntoIterator<Item = Option<i64>>) {
        let mut nodes: Vec<i64> = nodes.into_iter().flatten().collect();
        nodes.sort_unstable();
        nodes.dedup();
        let _guard = self.dispatch.lock().await;
        for node in nodes {
            self.push_locked(node).await;
        }
    }

    pub async fn push_all(&self) {
        let nodes: Vec<i64> = self.live.lock().unwrap().keys().copied().collect();
        self.push_many(nodes.into_iter().map(Some)).await;
    }

    pub async fn shutdown(&self) {
        if let Some(task) = self.accept_task.lock().unwrap().take() {
            task.abort();
        }
        let live: Vec<LiveNode> = self.live.lock().unwrap().drain().map(|(_, n)| n).collect();
        for node in &live {
            close_with(&node.connection, CloseCode::Normal);
        }
        self.endpoint.close().await;
        if let Some(relay) = self.relay.lock().await.take() {
            relay.shutdown().await;
        }
    }

    async fn handle(self: Arc<Self>, incoming: Incoming) {
        let connection = match timeout(HANDSHAKE_TIMEOUT, incoming).await {
            Ok(Ok(connection)) => connection,
            Ok(Err(e)) => {
                debug!(error = %e, "fleet handshake failed");
                return;
            }
            Err(_) => {
                debug!("fleet handshake timed out");
                return;
            }
        };
        let remote = connection.remote_id();
        let streams = timeout(HANDSHAKE_TIMEOUT, async {
            let (send, mut recv) = connection.accept_bi().await.ok()?;
            let body = protocol::read_frame_bytes(&mut recv).await.ok()??;
            Some((send, recv, body))
        })
        .await;
        let Ok(Some((send, recv, body))) = streams else {
            debug!(remote = %remote.fmt_short(), "fleet peer sent no hello");
            close_with(&connection, CloseCode::Protocol);
            return;
        };
        log_frame(None, remote, &body);
        let hello = match protocol::decode::<NodeMessage>(&body) {
            Ok(NodeMessage::Hello(hello)) => hello,
            _ => {
                warn!(remote = %remote.fmt_short(), "fleet peer did not start with hello");
                close_with(&connection, CloseCode::Protocol);
                return;
            }
        };
        let result = if hello.join.is_some() {
            self.join(connection.clone(), send, remote, hello).await
        } else {
            self.session(connection.clone(), send, recv, remote, hello)
                .await
        };
        if let Err(e) = result {
            warn!(remote = %remote.fmt_short(), error = ?e, "fleet connection failed");
            close_with(&connection, CloseCode::Normal);
        }
    }

    async fn join(
        &self,
        connection: Connection,
        mut send: SendStream,
        remote: EndpointId,
        hello: Hello,
    ) -> AppResult<()> {
        let proof = hello.join.expect("checked by caller");
        let secret = store::parse_hex::<{ super::ticket::SECRET_LEN }>(&proof.secret);
        let outcome = match secret {
            Some(secret) => {
                store::redeem_token(
                    &self.pool,
                    &proof.token,
                    &secret,
                    &remote.to_string(),
                    hello.name.trim(),
                    hello.allow_hooks,
                    now_ms(),
                )
                .await?
            }
            None => Redeem::InvalidToken,
        };
        match outcome {
            Redeem::Joined(node) => {
                info!(
                    node = node.id,
                    name = %node.name,
                    endpoint_id = %remote,
                    token = %proof.token,
                    "fleet node joined"
                );
                let welcome = ControllerMessage::Welcome {
                    node_id: node.id,
                    relays: self.advertised_strings(),
                };
                protocol::write_frame(&mut send, &welcome)
                    .await
                    .change_context(AppError::Unknown)?;
                let _ = send.finish();
                // 节点写完 node.json 会自己关连接
                let _ = timeout(HANDSHAKE_TIMEOUT, connection.closed()).await;
            }
            Redeem::InvalidToken => {
                warn!(remote = %remote.fmt_short(), token = %proof.token, "fleet join rejected: invalid or used token");
                close_with(&connection, CloseCode::Unauthorized);
            }
            Redeem::Revoked => {
                warn!(remote = %remote.fmt_short(), "fleet join rejected: key was revoked");
                close_with(&connection, CloseCode::Revoked);
            }
        }
        Ok(())
    }

    async fn session(
        &self,
        connection: Connection,
        mut send: SendStream,
        mut recv: RecvStream,
        remote: EndpointId,
        hello: Hello,
    ) -> AppResult<()> {
        let row = store::node_by_endpoint(&self.pool, &remote.to_string()).await?;
        let node = match row {
            Some(node) if node.revoked_at.is_none() => node,
            Some(_) => {
                info!(remote = %remote.fmt_short(), "fleet connection from a revoked node");
                close_with(&connection, CloseCode::Revoked);
                return Ok(());
            }
            None => {
                warn!(remote = %remote.fmt_short(), "fleet connection from an unknown key");
                close_with(&connection, CloseCode::Unauthorized);
                return Ok(());
            }
        };
        let id = node.id;
        if hello.proto >= 1 {
            // 节点自己的意愿以它此刻的 node.json 为准；账号每次连上整份替换
            if hello.allow_hooks != node.allow_hooks {
                store::set_allow_hooks(&self.pool, id, hello.allow_hooks).await?;
            }
            assignments::replace_accounts(&self.pool, id, &hello.accounts, now_ms()).await?;
        }
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        let now = now_ms();
        let (outbox, mut frames) = mpsc::unbounded_channel();
        let previous = self.live.lock().unwrap().insert(
            id,
            LiveNode {
                seq,
                connection: connection.clone(),
                connected_at: now,
                last_message_at: now,
                last_persisted_at: now,
                version: hello.version.clone(),
                proto: hello.proto,
                tools: hello.tools.clone(),
                summary: node
                    .last_summary
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok()),
                interval_ms: 0,
                samples: VecDeque::new(),
                rooms: Vec::new(),
                outbox,
                pushed_version: None,
                acked_version: None,
                held: HashMap::new(),
                failed: HashMap::new(),
                config_ack: None,
            },
        );
        if let Some(previous) = previous {
            close_with(&previous.connection, CloseCode::Superseded);
        }
        info!(node = id, name = %node.name, version = %hello.version, "fleet node online");
        store::record_seen(&self.pool, id, now, &hello.version, None).await?;

        let welcome = ControllerMessage::Welcome {
            node_id: id,
            relays: self.advertised_strings(),
        };
        let result = async {
            protocol::write_frame(&mut send, &welcome)
                .await
                .change_context(AppError::Unknown)?;
            if hello.proto >= DESIRED_STATE_SINCE {
                self.log_stale_rooms(id, &hello).await;
                // 连上就按当前分派下发一次完整的期望状态；节点据此停掉不该再录的（epoch 对账）
                self.push(id).await;
            }
            let writer = async {
                while let Some(message) = frames.recv().await {
                    log_outgoing(id, remote, &message);
                    if let Err(e) = protocol::write_frame(&mut send, &message).await {
                        debug!(node = id, error = %e, "could not write fleet frame");
                        break;
                    }
                }
            };
            tokio::select! {
                result = self.read_loop(&connection, &mut recv, remote, id, seq) => result,
                () = writer => Ok(()),
            }
        }
        .await;

        let removed = {
            let mut live = self.live.lock().unwrap();
            if live.get(&id).is_some_and(|node| node.seq == seq) {
                live.remove(&id)
            } else {
                None
            }
        };
        if let Some(node) = removed {
            let summary = node
                .summary
                .as_ref()
                .and_then(|s| serde_json::to_string(s).ok());
            let _ = store::record_seen(
                &self.pool,
                id,
                node.last_message_at,
                &node.version,
                summary.as_deref(),
            )
            .await;
            info!(node = id, "fleet node offline");
        }
        result
    }

    async fn read_loop(
        &self,
        connection: &Connection,
        recv: &mut RecvStream,
        remote: EndpointId,
        id: i64,
        seq: u64,
    ) -> AppResult<()> {
        loop {
            let frame = tokio::select! {
                frame = timeout(OFFLINE_AFTER, protocol::read_frame_bytes(recv)) => frame,
                reason = connection.closed() => {
                    debug!(node = id, %reason, "fleet connection closed");
                    return Ok(());
                }
            };
            let body = match frame {
                Err(_) => {
                    info!(node = id, "no heartbeat for {:?}, closing", OFFLINE_AFTER);
                    close_with(connection, CloseCode::Normal);
                    return Ok(());
                }
                Ok(Ok(Some(body))) => body,
                Ok(Ok(None)) | Ok(Err(protocol::FrameError::Io(_))) => return Ok(()),
                Ok(Err(e)) => {
                    warn!(node = id, error = %e, "bad fleet frame");
                    close_with(connection, CloseCode::Protocol);
                    return Ok(());
                }
            };
            log_frame(Some(id), remote, &body);
            let message = match protocol::decode::<NodeMessage>(&body) {
                Ok(message) => message,
                Err(e) => {
                    warn!(node = id, error = %e, "bad fleet frame");
                    close_with(connection, CloseCode::Protocol);
                    return Ok(());
                }
            };
            match message {
                NodeMessage::Heartbeat(heartbeat) => self.heartbeat(id, seq, heartbeat).await,
                NodeMessage::Event(event) => {
                    debug!(node = id, kind = %event.kind, "fleet node event");
                    self.touch(id, seq);
                }
                NodeMessage::Ack(ack) => self.ack(id, seq, ack).await,
                NodeMessage::Leave => {
                    {
                        let _guard = self.dispatch.lock().await;
                        store::revoke_node(&self.pool, id, now_ms()).await?;
                        assignments::unassign_node(&self.pool, id, now_ms()).await?;
                    }
                    info!(node = id, "fleet node left");
                    close_with(connection, CloseCode::Normal);
                    self.push_all().await;
                    return Ok(());
                }
                NodeMessage::Hello(_) => {
                    close_with(connection, CloseCode::Protocol);
                    return Ok(());
                }
            }
        }
    }

    /// 节点 `Hello` 里带来的、按本地缓存持有的房间与当前分派对不上的，记一笔；
    /// 紧接着下发的期望状态会让节点停掉它们。
    async fn log_stale_rooms(&self, id: i64, hello: &Hello) {
        if hello.rooms.is_empty() {
            return;
        }
        let current = match assignments::desired_state(&self.pool, id).await {
            Ok((rooms, _)) => rooms,
            Err(_) => return,
        };
        for held in &hello.rooms {
            match current.iter().find(|room| room.id == held.id) {
                Some(room) if room.epoch == held.epoch => {}
                Some(room) => debug!(
                    node = id,
                    room = held.id,
                    held = held.epoch,
                    current = room.epoch,
                    "node holds an older epoch of a room that is still assigned to it"
                ),
                None => info!(
                    node = id,
                    room = held.id,
                    epoch = held.epoch,
                    "node still holds a room that is no longer assigned to it, releasing"
                ),
            }
        }
    }

    async fn ack(&self, id: i64, seq: u64, ack: Ack) {
        let now = now_ms();
        {
            let mut live = self.live.lock().unwrap();
            let Some(node) = live.get_mut(&id).filter(|node| node.seq == seq) else {
                return;
            };
            node.apply_ack(&ack, now);
        }
        let held: Vec<i64> = ack.held.iter().map(|room| room.id).collect();
        let version = i64::try_from(ack.version).unwrap_or(i64::MAX);
        let confirmed = {
            let _guard = self.dispatch.lock().await;
            match assignments::confirm_releases(&self.pool, id, version, &held, now).await {
                Ok(confirmed) => confirmed,
                Err(e) => {
                    warn!(node = id, error = ?e, "could not confirm fleet room releases");
                    return;
                }
            }
        };
        if confirmed.is_empty() {
            return;
        }
        info!(node = id, rooms = ?confirmed, "fleet rooms released");
        let mut targets = Vec::new();
        for room in confirmed {
            if let Ok(Some(room)) = assignments::room(&self.pool, room).await {
                targets.push(room.node_id);
            }
        }
        self.push_many(targets).await;
    }

    fn touch(&self, id: i64, seq: u64) {
        if let Some(node) = self.live.lock().unwrap().get_mut(&id)
            && node.seq == seq
        {
            node.last_message_at = now_ms();
        }
    }

    async fn heartbeat(&self, id: i64, seq: u64, mut heartbeat: Heartbeat) {
        let now = now_ms();
        if let Some(accounts) = heartbeat.accounts.take()
            && let Err(e) = assignments::replace_accounts(&self.pool, id, &accounts, now).await
        {
            warn!(node = id, error = ?e, "could not record fleet node accounts");
        }
        let persist = {
            let mut live = self.live.lock().unwrap();
            let Some(node) = live.get_mut(&id).filter(|node| node.seq == seq) else {
                return;
            };
            node.apply(heartbeat, now);
            if now - node.last_persisted_at >= PERSIST_EVERY_MS {
                node.last_persisted_at = now;
                Some((
                    node.version.clone(),
                    node.summary
                        .as_ref()
                        .and_then(|s| serde_json::to_string(s).ok()),
                ))
            } else {
                None
            }
        };
        if let Some((version, summary)) = persist
            && let Err(e) =
                store::record_seen(&self.pool, id, now, &version, summary.as_deref()).await
        {
            warn!(node = id, error = ?e, "could not persist fleet heartbeat");
        }
    }
}

fn view(row: NodeRow, live: Option<&LiveNode>, now: i64) -> NodeView {
    let labels = serde_json::from_str(&row.labels).unwrap_or(serde_json::Value::Array(vec![]));
    let stored_summary = row
        .last_summary
        .as_deref()
        .and_then(|s| serde_json::from_str::<Summary>(s).ok());
    let config = NodeConfigState::of(
        live,
        live.map(|node| node.version.as_str())
            .or(row.last_version.as_deref()),
    );
    match live {
        Some(node) => NodeView {
            id: row.id,
            name: row.name,
            endpoint_id: row.endpoint_id,
            labels,
            allow_hooks: row.allow_hooks,
            created_at: row.created_at,
            last_seen_at: Some(node.last_message_at),
            version: Some(node.version.clone()),
            proto: Some(node.proto),
            tools: node.tools.clone(),
            online: node.online(now),
            connected_at: Some(node.connected_at),
            path: node.path(),
            summary: node.summary.clone().or(stored_summary),
            interval_ms: (node.interval_ms > 0).then_some(node.interval_ms),
            samples: node.samples.iter().copied().collect(),
            assigned_rooms: 0,
            accounts: Vec::new(),
            synced: node.accepts_desired_state().then(|| {
                node.pushed_version.is_some() && node.pushed_version == node.acked_version
            }),
            config,
        },
        None => NodeView {
            id: row.id,
            name: row.name,
            endpoint_id: row.endpoint_id,
            labels,
            allow_hooks: row.allow_hooks,
            created_at: row.created_at,
            last_seen_at: row.last_seen_at,
            version: row.last_version,
            proto: None,
            tools: None,
            online: false,
            connected_at: None,
            path: None,
            summary: stored_summary,
            interval_ms: None,
            samples: Vec::new(),
            assigned_rooms: 0,
            accounts: Vec::new(),
            synced: None,
            config,
        },
    }
}

/// 在控制面侧记录解密后的帧（debug 级别，单独的 target），用来核对帧里没有白名单外的字段。
/// join 秘密在记录前抹掉。
fn log_frame(node: Option<i64>, remote: EndpointId, body: &[u8]) {
    if !tracing::enabled!(target: FRAME_LOG_TARGET, tracing::Level::DEBUG) {
        return;
    }
    debug!(
        target: FRAME_LOG_TARGET,
        node,
        remote = %remote.fmt_short(),
        bytes = body.len(),
        frame = %redacted_frame(body),
        "fleet frame"
    );
}

fn redacted_frame(body: &[u8]) -> String {
    match serde_json::from_slice::<serde_json::Value>(body) {
        Ok(mut value) => {
            if let Some(secret) = value.pointer_mut("/join/secret") {
                *secret = serde_json::Value::String("[redacted]".into());
            }
            value.to_string()
        }
        Err(_) => format!("<{} bytes, not json>", body.len()),
    }
}

/// 控制面发出的帧同样按 [`FRAME_LOG_TARGET`] 记下，用来核对下发的期望状态里没有凭据
fn log_outgoing(node: i64, remote: EndpointId, message: &ControllerMessage) {
    if !tracing::enabled!(target: FRAME_LOG_TARGET, tracing::Level::DEBUG) {
        return;
    }
    let body = serde_json::to_string(message).unwrap_or_default();
    debug!(
        target: FRAME_LOG_TARGET,
        node,
        remote = %remote.fmt_short(),
        bytes = body.len(),
        direction = "out",
        frame = %body,
        "fleet frame"
    );
}

async fn accept_loop(controller: std::sync::Weak<Controller>, endpoint: Endpoint) {
    while let Some(incoming) = endpoint.accept().await {
        let Some(controller) = controller.upgrade() else {
            break;
        };
        tokio::spawn(controller.handle(incoming));
    }
}
