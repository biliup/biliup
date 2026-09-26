//! 节点端：`biliup node join / leave / status` 与 `biliup server` 里的节点代理。

use super::accounts;
use super::guard::ManagedHandle;
use super::protocol::{
    self, CloseCode, ControllerMessage, HEARTBEAT_INTERVAL, Heartbeat, Hello, JoinProof,
    NodeMessage, PROTOCOL_MINOR, PoolUsage, Pools, ToolStatus, Tools,
};
use super::reconcile::{self, Reconciler};
use super::relay::{DENY_REVOKED, DENY_TOKEN_INVALID, DENY_UNKNOWN};
use super::store::{hex, parse_hex};
use super::ticket::JoinTicket;
use super::{bind_endpoint, close_with, now_ms};
use crate::server::api::redact;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::context::WorkerStatus;
use crate::server::infrastructure::service_register::ServiceRegister;
use error_stack::{Report, ResultExt, bail};
use iroh::endpoint::{ConnectionError, RecvStream, SendStream};
use iroh::{
    Endpoint, EndpointAddr, EndpointId, RelayConfig, RelayMap, RelayUrl, SecretKey, Watcher,
};
use iroh_tickets::Ticket;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

pub const NODE_FILE_VERSION: u32 = 1;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
const REPLY_TIMEOUT: Duration = Duration::from_secs(15);
const BACKOFF_MIN: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(30);
/// 被同一把钥匙的新连接顶掉后，等这么久再试，免得两个进程来回互顶
const SUPERSEDED_WAIT: Duration = Duration::from_secs(30);
/// 探测 relay 地址 TCP 端口的超时
const RELAY_PROBE_TIMEOUT: Duration = Duration::from_secs(3);
const SHUTDOWN_WAIT: Duration = Duration::from_secs(5);
/// 对账里停一个正在录的房间要等下载器收尾，多给一点时间
const SHUTDOWN_GRACE: Duration = Duration::from_secs(30);

/// `data/node.json`（0600）
#[derive(Clone, Serialize, Deserialize)]
pub struct NodeFile {
    pub version: u32,
    /// 控制面 EndpointId（十六进制）
    pub controller: String,
    /// 控制面最近一次下发的 relay 地址
    pub relays: Vec<String>,
    pub node_id: i64,
    /// 节点 iroh 私钥（十六进制）：节点的长期凭据
    pub secret_key: String,
    pub allow_hooks: bool,
    pub joined_at: i64,
}

impl fmt::Debug for NodeFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeFile")
            .field("controller", &self.controller)
            .field("relays", &self.relays)
            .field("node_id", &self.node_id)
            .field("secret_key", &"[redacted]")
            .field("allow_hooks", &self.allow_hooks)
            .finish()
    }
}

impl NodeFile {
    pub fn load(path: &Path) -> AppResult<Self> {
        let text = std::fs::read_to_string(path)
            .change_context(AppError::Unknown)
            .attach_with(|| format!("could not read {}", path.display()))?;
        let file: NodeFile = serde_json::from_str(&text)
            .change_context(AppError::Unknown)
            .attach_with(|| format!("{} is not a valid node file", path.display()))?;
        if file.version != NODE_FILE_VERSION {
            bail!(AppError::Custom(format!(
                "{} has unsupported version {}",
                path.display(),
                file.version
            )));
        }
        file.secret()?;
        file.controller_id()?;
        Ok(file)
    }

    /// 先写临时文件再改名；Unix 上创建时就是 0600，私钥不会有一刻对其他用户可读。
    pub fn save(&self, path: &Path) -> AppResult<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).change_context(AppError::Unknown)?;
        }
        let tmp = path.with_extension("json.tmp");
        let body = serde_json::to_vec_pretty(self).change_context(AppError::Unknown)?;
        {
            use std::io::Write;
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create(true).truncate(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options
                .open(&tmp)
                .change_context(AppError::Unknown)
                .attach_with(|| format!("could not write {}", tmp.display()))?;
            file.write_all(&body).change_context(AppError::Unknown)?;
            file.sync_all().change_context(AppError::Unknown)?;
        }
        std::fs::rename(&tmp, path)
            .change_context(AppError::Unknown)
            .attach_with(|| format!("could not write {}", path.display()))
    }

    pub fn secret(&self) -> AppResult<SecretKey> {
        parse_hex::<32>(&self.secret_key)
            .map(|bytes| SecretKey::from_bytes(&bytes))
            .ok_or_else(|| Report::new(AppError::Custom("node.json: invalid secret_key".into())))
    }

    pub fn controller_id(&self) -> AppResult<EndpointId> {
        self.controller
            .parse()
            .change_context(AppError::Custom("node.json: invalid controller id".into()))
    }

    fn relay_urls(&self) -> Vec<RelayUrl> {
        self.relays
            .iter()
            .filter_map(|relay| relay.parse().ok())
            .collect()
    }
}

fn relay_map(relays: &[RelayUrl], auth_token: Option<&str>) -> RelayMap {
    RelayMap::from_iter(relays.iter().map(|url| {
        let config = RelayConfig::new(url.clone(), None);
        match auth_token {
            Some(token) => config.with_auth_token(token),
            None => config,
        }
    }))
}

/// 票据里的 relay 地址通常是同一个内嵌 relay 的几个别名（控制面的各个网卡地址）。iroh 会对每个地址各连一次，
/// 同一把密钥的连接在 relay 上互相顶掉，所以只用一个：按票据顺序（公网在前）取第一个 TCP 连得上的。
/// 只有一个地址时不探测；一个都连不上时返回 `None`，由调用方决定用哪个。
async fn pick_relay(relays: &[RelayUrl]) -> Option<RelayUrl> {
    if relays.len() <= 1 {
        return relays.first().cloned();
    }
    let probes = relays.iter().map(|url| async move {
        let (Some(host), Some(port)) = (url.host(), url.port_or_known_default()) else {
            return false;
        };
        let host = match host {
            url::Host::Domain(domain) => domain.to_string(),
            url::Host::Ipv4(ip) => ip.to_string(),
            url::Host::Ipv6(ip) => ip.to_string(),
        };
        let connect = tokio::net::TcpStream::connect((host.as_str(), port));
        matches!(timeout(RELAY_PROBE_TIMEOUT, connect).await, Ok(Ok(_)))
    });
    let reachable = futures::future::join_all(probes).await;
    relays
        .iter()
        .zip(reachable)
        .find_map(|(url, ok)| ok.then(|| url.clone()))
}

/// 一次性的连接（join / leave）：探测不到就用第一个，交给 iroh 自己去试（比如本机解析不了 relay 的域名）
async fn pick_relay_or_first(relays: &[RelayUrl]) -> Vec<RelayUrl> {
    pick_relay(relays)
        .await
        .or_else(|| relays.first().cloned())
        .into_iter()
        .collect()
}

/// 把运行中端点的 relay map 从 `current` 换成 `relays`
async fn use_relays(endpoint: &Endpoint, current: &mut Vec<RelayUrl>, relays: Vec<RelayUrl>) {
    for url in relays.iter().filter(|url| !current.contains(url)) {
        endpoint
            .insert_relay(url.clone(), Arc::new(RelayConfig::new(url.clone(), None)))
            .await;
    }
    for url in current.iter().filter(|url| !relays.contains(url)) {
        endpoint.remove_relay(url).await;
    }
    *current = relays;
}

fn controller_addr(controller: EndpointId, relays: &[RelayUrl]) -> EndpointAddr {
    relays
        .iter()
        .fold(EndpointAddr::new(controller), |addr, url| {
            addr.with_relay_url(url.clone())
        })
}

/// join / leave / 节点代理共用的 `Hello`；账号、工具与已持有的房间只有节点代理才填
fn hello(allow_hooks: bool) -> Hello {
    Hello {
        proto: PROTOCOL_MINOR,
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: host_name(),
        allow_hooks,
        ..Hello::default()
    }
}

async fn tools() -> Tools {
    let ffmpeg = crate::tools::ffmpeg_status().await;
    Tools {
        ffmpeg: ToolStatus {
            available: ffmpeg.available,
            version: ffmpeg.version,
        },
    }
}

/// 本机界面上「由控制面 xxx 管理」里的 xxx：relay 地址的主机名，没有就用控制面 id 的短写
fn controller_label(file: &NodeFile) -> String {
    file.relays
        .iter()
        .find_map(|relay| {
            url::Url::parse(relay)
                .ok()
                .and_then(|url| url.host_str().map(str::to_string))
        })
        .or_else(|| {
            file.controller_id()
                .ok()
                .map(|id| id.fmt_short().to_string())
        })
        .unwrap_or_else(|| file.controller.chars().take(10).collect())
}

fn host_name() -> String {
    sysinfo::System::host_name()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "node".to_string())
}

/// relay 拒绝了我们时给出拒绝原因；一直没被拒绝就永远不返回。
async fn relay_denial(endpoint: &Endpoint) -> String {
    let mut watcher = endpoint.home_relay_status();
    loop {
        if let Some(reason) = watcher
            .get()
            .iter()
            .find_map(|status| status.auth_denied_reason().map(str::to_string))
        {
            return reason;
        }
        if watcher.updated().await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

fn relay_connected(endpoint: &Endpoint) -> bool {
    endpoint
        .home_relay_status()
        .get()
        .iter()
        .any(|status| status.is_connected())
}

fn close_code(error: &ConnectionError) -> Option<CloseCode> {
    match error {
        ConnectionError::ApplicationClosed(close) => {
            CloseCode::from_code(close.error_code.into_inner())
        }
        _ => None,
    }
}

enum Dial {
    Connected(iroh::endpoint::Connection, SendStream, RecvStream),
    Denied(String),
    Unreachable(String),
}

async fn dial(endpoint: &Endpoint, controller: EndpointAddr, hello: &Hello) -> Dial {
    let connect = async {
        let connection = endpoint
            .connect(controller, protocol::ALPN)
            .await
            .map_err(|e| e.to_string())?;
        let (mut send, recv) = connection.open_bi().await.map_err(|e| e.to_string())?;
        protocol::write_frame(&mut send, &NodeMessage::Hello(hello.clone()))
            .await
            .map_err(|e| e.to_string())?;
        Ok::<_, String>((connection, send, recv))
    };
    tokio::select! {
        result = timeout(CONNECT_TIMEOUT, connect) => match result {
            Ok(Ok((connection, send, recv))) => Dial::Connected(connection, send, recv),
            Ok(Err(e)) => Dial::Unreachable(e),
            Err(_) => Dial::Unreachable("timed out".into()),
        },
        reason = relay_denial(endpoint) => Dial::Denied(reason),
    }
}

/// 等控制面对 `Hello` 的应答。连接被关掉时给出关闭码。
async fn read_welcome(
    connection: &iroh::endpoint::Connection,
    recv: &mut RecvStream,
) -> Result<(i64, Vec<String>), Option<CloseCode>> {
    let frame = tokio::select! {
        frame = timeout(REPLY_TIMEOUT, protocol::read_frame::<_, ControllerMessage>(recv)) => frame,
        reason = connection.closed() => return Err(close_code(&reason)),
    };
    match frame {
        Ok(Ok(Some(ControllerMessage::Welcome { node_id, relays }))) => Ok((node_id, relays)),
        Ok(Ok(Some(_))) => Err(Some(CloseCode::Protocol)),
        _ => Err(connection.close_reason().as_ref().and_then(close_code)),
    }
}

fn unreachable_message(relays: &[String]) -> String {
    format!(
        "连不上控制面的 relay {}：控制面需要一个本机能访问的 TCP 端口，见部署说明",
        relays.join(", ")
    )
}

fn denial_message(reason: &str) -> String {
    if reason.contains(DENY_TOKEN_INVALID) {
        "控制面拒绝了这张票据：已被使用、已过期或已作废，请在控制面重新生成".into()
    } else if reason.contains(DENY_REVOKED) {
        "这台节点已被控制面移除".into()
    } else if reason.contains(DENY_UNKNOWN) {
        "控制面不认识这台节点的密钥（节点已被移除，或控制面的数据被重置）".into()
    } else {
        format!("控制面的 relay 拒绝了连接：{reason}")
    }
}

/// `biliup node join <票据>`：生成节点密钥，向控制面出示票据里的秘密，拿到节点 id 后写 `data/node.json` 并退出。
pub async fn join(ticket: &str, allow_hooks: bool, node_file: &Path) -> AppResult<NodeFile> {
    let ticket = JoinTicket::decode_string(ticket.trim())
        .map_err(|e| Report::new(AppError::Custom(format!("票据无效：{e}"))))?;
    if node_file.exists() {
        bail!(AppError::Custom(format!(
            "本机已经加入了控制面（{} 已存在）。要换控制面，先执行 `biliup node leave`",
            node_file.display()
        )));
    }
    if ticket.expires_at <= now_ms() {
        bail!(AppError::Custom(
            "票据已过期，请在控制面重新生成".to_string()
        ));
    }
    let relays: Vec<RelayUrl> = ticket
        .relays
        .iter()
        .filter_map(|relay| relay.parse().ok())
        .collect();
    if relays.is_empty() {
        bail!(AppError::Custom("票据里没有可用的 relay 地址".into()));
    }
    let relays = pick_relay_or_first(&relays).await;

    let secret = SecretKey::generate();
    let endpoint = bind_endpoint(
        secret.clone(),
        relay_map(&relays, Some(&ticket.token)),
        false,
    )
    .await
    .attach("could not bind the fleet node endpoint")?;
    let hello = Hello {
        join: Some(JoinProof {
            token: ticket.token.clone(),
            secret: hex(&ticket.secret),
        }),
        ..hello(allow_hooks)
    };
    info!(controller = %ticket.controller.fmt_short(), relays = ?ticket.relays, "joining fleet controller");
    let result = async {
        let (connection, _send, mut recv) =
            match dial(&endpoint, controller_addr(ticket.controller, &relays), &hello).await {
                Dial::Connected(connection, send, recv) => (connection, send, recv),
                Dial::Denied(reason) => bail!(AppError::Custom(denial_message(&reason))),
                Dial::Unreachable(e) => {
                    debug!(error = %e, relay_connected = relay_connected(&endpoint), "join dial failed");
                    bail!(AppError::Custom(unreachable_message(&ticket.relays)))
                }
            };
        let reply = read_welcome(&connection, &mut recv).await;
        let outcome = match reply {
            Ok((node_id, relays)) => {
                let file = NodeFile {
                    version: NODE_FILE_VERSION,
                    controller: ticket.controller.to_string(),
                    relays: if relays.is_empty() {
                        ticket.relays.clone()
                    } else {
                        relays
                    },
                    node_id,
                    secret_key: hex(&secret.to_bytes()),
                    allow_hooks,
                    joined_at: now_ms(),
                };
                file.save(node_file).map(|()| file)
            }
            Err(Some(CloseCode::Unauthorized)) => Err(Report::new(AppError::Custom(
                "控制面拒绝了这张票据：已被使用、已过期或已作废，请在控制面重新生成".into(),
            ))),
            Err(Some(CloseCode::Revoked)) => Err(Report::new(AppError::Custom(
                "控制面拒绝加入：这把密钥已被移除".into(),
            ))),
            Err(code) => Err(Report::new(AppError::Custom(format!(
                "控制面没有完成加入（{}）",
                code.map_or("连接中断", CloseCode::as_str)
            )))),
        };
        close_with(&connection, CloseCode::Normal);
        outcome
    }
    .await;
    endpoint.close().await;
    result
}

/// `biliup node leave`：通知控制面把自己移出节点表，然后删掉本地凭据。
/// 控制面不可达时只删本地凭据，并提示去控制面手动移除。
pub async fn leave(node_file: &Path) -> AppResult<bool> {
    let file = NodeFile::load(node_file)?;
    let relays = pick_relay_or_first(&file.relay_urls()).await;
    let endpoint = bind_endpoint(file.secret()?, relay_map(&relays, None), false)
        .await
        .attach("could not bind the fleet node endpoint")?;
    let hello = hello(file.allow_hooks);
    let notified = match dial(
        &endpoint,
        controller_addr(file.controller_id()?, &relays),
        &hello,
    )
    .await
    {
        Dial::Connected(connection, mut send, mut recv) => {
            let acknowledged = match read_welcome(&connection, &mut recv).await {
                Ok(_) => {
                    let sent = protocol::write_frame(&mut send, &NodeMessage::Leave)
                        .await
                        .is_ok();
                    sent && timeout(REPLY_TIMEOUT, connection.closed()).await.is_ok()
                }
                // 已经被移除了，等于离开成功
                Err(Some(CloseCode::Revoked)) => true,
                Err(_) => false,
            };
            close_with(&connection, CloseCode::Normal);
            acknowledged
        }
        Dial::Denied(reason) => reason.contains(DENY_REVOKED),
        Dial::Unreachable(e) => {
            debug!(error = %e, "leave dial failed");
            false
        }
    };
    endpoint.close().await;
    std::fs::remove_file(node_file)
        .change_context(AppError::Unknown)
        .attach_with(|| format!("could not remove {}", node_file.display()))?;
    // 托管的房间与模板留在本机库里，从此是本地的
    reconcile::forget(&reconcile::state_path(node_file));
    Ok(notified)
}

/// `biliup node status`：只读本地 `data/node.json`，不连控制面（免得顶掉正在运行的节点代理）。
pub fn status(node_file: &Path) -> AppResult<Option<serde_json::Value>> {
    if !node_file.exists() {
        return Ok(None);
    }
    let file = NodeFile::load(node_file)?;
    Ok(Some(serde_json::json!({
        "node_id": file.node_id,
        "endpoint_id": file.secret()?.public().to_string(),
        "controller": file.controller,
        "relays": file.relays,
        "allow_hooks": file.allow_hooks,
        "joined_at": file.joined_at,
    })))
}

/// `biliup server` 里的节点代理：按 `data/node.json` 连控制面，每 10 s 发一次心跳，断了就退避重连。
pub struct NodeAgent {
    stop: watch::Sender<bool>,
    task: JoinHandle<()>,
}

impl NodeAgent {
    pub async fn start(
        node_file: PathBuf,
        services: ServiceRegister,
        managed: ManagedHandle,
    ) -> AppResult<Self> {
        let file = NodeFile::load(&node_file)?;
        // relay 在每次连接前现挑（见 `pick_relay`），启动时不等探测
        let endpoint = bind_endpoint(file.secret()?, RelayMap::empty(), false)
            .await
            .attach("could not bind the fleet node endpoint")?;
        info!(
            node = file.node_id,
            endpoint_id = %endpoint.id(),
            controller = %file.controller_id()?.fmt_short(),
            "fleet node agent started"
        );
        let reconciler = Reconciler::resume(
            reconcile::state_path(&node_file),
            &file.controller,
            controller_label(&file),
            file.allow_hooks,
            services.clone(),
            managed,
        )
        .await;
        let (stop, stopped) = watch::channel(false);
        let task = tokio::spawn(run_agent(
            endpoint, node_file, file, services, reconciler, stopped,
        ));
        Ok(NodeAgent { stop, task })
    }

    /// 在停录制调度之前调用（REC-23）：对账做到一半时等它做完，免得调度先停、对账还在往里加房间
    pub async fn shutdown(mut self) {
        let _ = self.stop.send(true);
        if timeout(SHUTDOWN_WAIT, &mut self.task).await.is_err() {
            warn!("fleet node agent is still applying the desired state, waiting for it");
            if timeout(SHUTDOWN_GRACE, &mut self.task).await.is_err() {
                self.task.abort();
                let _ = self.task.await;
            }
        }
    }

    /// 节点代理已经停了（被移除、凭据被删或收到了停止信号）
    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
    }
}

enum Outcome {
    /// 连上过，之后断开
    Dropped,
    /// 没连上
    Failed,
    Superseded,
    /// 被移除或密钥不被承认：不再重连
    Rejected(String),
    Stopped,
}

async fn run_agent(
    endpoint: Endpoint,
    node_file: PathBuf,
    mut file: NodeFile,
    services: ServiceRegister,
    mut reconciler: Reconciler,
    mut stopped: watch::Receiver<bool>,
) {
    let mut backoff = BACKOFF_MIN;
    let mut relays = Vec::new();
    loop {
        if *stopped.borrow() {
            break;
        }
        if !node_file.exists() {
            info!("{} removed, fleet node agent stops", node_file.display());
            reconciler.release();
            break;
        }
        let outcome = session(
            &endpoint,
            &mut relays,
            &node_file,
            &mut file,
            &services,
            &mut reconciler,
            &mut stopped,
        )
        .await;
        let wait = match outcome {
            Outcome::Stopped => break,
            Outcome::Rejected(message) => {
                error!(
                    "{message}；节点代理停止重连，控制面分派的房间转为本机房间继续录。重新加入请先执行 `biliup node leave`，再用新票据 join"
                );
                reconciler.release();
                break;
            }
            Outcome::Superseded => {
                warn!("另一个进程用同一把节点密钥连上了控制面，{SUPERSEDED_WAIT:?} 后再试");
                backoff = BACKOFF_MIN;
                SUPERSEDED_WAIT
            }
            Outcome::Dropped => {
                backoff = BACKOFF_MIN;
                BACKOFF_MIN
            }
            Outcome::Failed => {
                let wait = backoff;
                backoff = (backoff * 2).min(BACKOFF_MAX);
                wait
            }
        };
        debug!(?wait, "fleet node reconnecting");
        tokio::select! {
            _ = tokio::time::sleep(wait) => {}
            _ = stopped.changed() => break,
        }
    }
    endpoint.close().await;
}

async fn session(
    endpoint: &Endpoint,
    relays: &mut Vec<RelayUrl>,
    node_file: &Path,
    file: &mut NodeFile,
    services: &ServiceRegister,
    reconciler: &mut Reconciler,
    stopped: &mut watch::Receiver<bool>,
) -> Outcome {
    let Ok(controller) = file.controller_id() else {
        return Outcome::Rejected("node.json 里的控制面 id 无效".into());
    };
    let mut reported_accounts = accounts::public(&accounts::scan(&services.pool).await);
    let hello = Hello {
        accounts: reported_accounts.clone(),
        tools: Some(tools().await),
        rooms: reconciler.held(),
        state_version: reconciler.state_version(),
        ..hello(file.allow_hooks)
    };
    let connect = async {
        let listed = file.relay_urls();
        // 都连不上（断网、控制面在重启）时沿用上次挑中的，别把所有别名都塞回去
        let picked = match pick_relay(&listed).await {
            Some(url) => vec![url],
            None if relays.is_empty() => listed.first().cloned().into_iter().collect(),
            None => relays.clone(),
        };
        use_relays(endpoint, relays, picked).await;
        dial(endpoint, controller_addr(controller, relays), &hello).await
    };
    let dialed = tokio::select! {
        dialed = connect => dialed,
        _ = stopped.changed() => return Outcome::Stopped,
    };
    let (connection, mut send, mut recv) = match dialed {
        Dial::Connected(connection, send, recv) => (connection, send, recv),
        Dial::Denied(reason) => {
            let message = denial_message(&reason);
            if reason.contains(DENY_REVOKED) || reason.contains(DENY_UNKNOWN) {
                return Outcome::Rejected(message);
            }
            warn!("{message}");
            return Outcome::Failed;
        }
        Dial::Unreachable(e) => {
            warn!(error = %e, "{}", unreachable_message(&file.relays));
            return Outcome::Failed;
        }
    };
    match read_welcome(&connection, &mut recv).await {
        Ok((node_id, relays)) => {
            info!(node = node_id, "connected to fleet controller");
            update_relays(node_file, file, relays);
        }
        Err(code) => return closed_outcome(code, false),
    }

    let mut since = None;
    let mut ticker = tokio::time::interval(HEARTBEAT_INTERVAL);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let mut heartbeat = heartbeat(services, &mut since).await;
                let current = accounts::public(&accounts::scan(&services.pool).await);
                if current != reported_accounts {
                    heartbeat.accounts = Some(current.clone());
                    reported_accounts = current;
                }
                if let Err(e) = protocol::write_frame(&mut send, &NodeMessage::Heartbeat(heartbeat)).await {
                    debug!(error = %e, "fleet heartbeat failed");
                    let reason = connection.closed().await;
                    return closed_outcome(close_code(&reason), true);
                }
            }
            frame = protocol::read_frame::<_, ControllerMessage>(&mut recv) => match frame {
                Ok(Some(ControllerMessage::Relays { relays })) => update_relays(node_file, file, relays),
                Ok(Some(ControllerMessage::DesiredState(desired))) => {
                    let ack = reconciler.apply(desired).await;
                    if let Err(e) = protocol::write_frame(&mut send, &NodeMessage::Ack(ack)).await {
                        debug!(error = %e, "fleet ack failed");
                        let reason = connection.closed().await;
                        return closed_outcome(close_code(&reason), true);
                    }
                }
                Ok(Some(ControllerMessage::Welcome { .. })) => {}
                Ok(None) | Err(_) => {
                    let reason = connection.closed().await;
                    return closed_outcome(close_code(&reason), true);
                }
            },
            reason = connection.closed() => return closed_outcome(close_code(&reason), true),
            _ = stopped.changed() => {
                close_with(&connection, CloseCode::Normal);
                return Outcome::Stopped;
            }
        }
    }
}

fn closed_outcome(code: Option<CloseCode>, was_connected: bool) -> Outcome {
    match code {
        Some(CloseCode::Revoked) => Outcome::Rejected("这台节点已被控制面移除".into()),
        Some(CloseCode::Unauthorized) => Outcome::Rejected(
            "控制面不认识这台节点的密钥（节点已被移除，或控制面的数据被重置）".into(),
        ),
        Some(CloseCode::Superseded) => Outcome::Superseded,
        Some(CloseCode::Protocol) => {
            warn!("控制面以 protocol 关闭了连接：两端版本可能不兼容，请升级到同一版本");
            Outcome::Failed
        }
        _ if was_connected => Outcome::Dropped,
        _ => Outcome::Failed,
    }
}

fn update_relays(node_file: &Path, file: &mut NodeFile, relays: Vec<String>) {
    if relays.is_empty() || relays == file.relays {
        return;
    }
    info!(?relays, "fleet controller relays changed");
    file.relays = relays;
    if let Err(e) = file.save(node_file) {
        warn!(error = ?e, "could not update {}", node_file.display());
    }
}

fn status_name(status: &WorkerStatus) -> &'static str {
    match status {
        WorkerStatus::Working(_) => "Working",
        WorkerStatus::Pending => "Pending",
        WorkerStatus::Idle => "Idle",
        WorkerStatus::Pause => "Pause",
    }
}

async fn heartbeat(services: &ServiceRegister, since: &mut Option<i64>) -> Heartbeat {
    let stats = services.system.snapshot(*since);
    if let Some(last) = stats.samples.last() {
        *since = Some(last.ts);
    }
    let managers = &services.managers;
    let workers = managers.get_rooms().await;
    let mut recording = 0;
    let rooms = workers
        .iter()
        .map(|worker| {
            let downloader = worker.downloader_status.read().unwrap();
            if matches!(*downloader, WorkerStatus::Working(_)) {
                recording += 1;
            }
            let mut room = serde_json::json!({
                "downloader_status": status_name(&downloader),
                "uploader_status": status_name(&worker.uploader_status.read().unwrap()),
                "live_streamer": worker.live_streamer,
                "upload_streamer": worker.upload_streamer,
            });
            redact::streamer_hooks(&mut room["live_streamer"]);
            redact::upload_template(&mut room["upload_streamer"]);
            room
        })
        .collect();
    Heartbeat {
        stats,
        pools: Pools {
            download: PoolUsage {
                capacity: managers.download_pool_size(),
                occupied: managers.download_pool_occupied(),
            },
            upload: PoolUsage {
                capacity: managers.upload_pool_size(),
                occupied: managers.upload_pool_occupied(),
            },
        },
        rooms,
        recording,
        accounts: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> NodeFile {
        NodeFile {
            version: NODE_FILE_VERSION,
            controller: SecretKey::generate().public().to_string(),
            relays: vec!["http://192.168.1.2:19160/".into()],
            node_id: 3,
            secret_key: hex(&SecretKey::generate().to_bytes()),
            allow_hooks: false,
            joined_at: 1,
        }
    }

    #[test]
    fn node_file_round_trips_with_owner_only_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data/node.json");
        let file = sample();
        file.save(&path).unwrap();
        let loaded = NodeFile::load(&path).unwrap();
        assert_eq!(loaded.secret_key, file.secret_key);
        assert_eq!(loaded.node_id, 3);
        assert!(!path.with_extension("json.tmp").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn debug_and_status_never_show_the_private_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("node.json");
        let file = sample();
        file.save(&path).unwrap();
        assert!(!format!("{file:?}").contains(&file.secret_key));
        let shown = status(&path).unwrap().unwrap().to_string();
        assert!(!shown.contains(&file.secret_key));
        assert!(shown.contains(&file.secret().unwrap().public().to_string()));
        assert!(status(&dir.path().join("missing.json")).unwrap().is_none());
    }

    #[test]
    fn close_codes_decide_whether_to_reconnect() {
        assert!(matches!(
            closed_outcome(Some(CloseCode::Revoked), true),
            Outcome::Rejected(_)
        ));
        assert!(matches!(
            closed_outcome(Some(CloseCode::Superseded), true),
            Outcome::Superseded
        ));
        assert!(matches!(closed_outcome(None, true), Outcome::Dropped));
        assert!(matches!(closed_outcome(None, false), Outcome::Failed));
        assert!(matches!(
            closed_outcome(Some(CloseCode::Normal), true),
            Outcome::Dropped
        ));
    }

    #[tokio::test]
    async fn one_reachable_relay_is_picked_in_ticket_order() {
        let live = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let other = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = |listener: &tokio::net::TcpListener| -> RelayUrl {
            format!("http://{}/", listener.local_addr().unwrap())
                .parse()
                .unwrap()
        };
        let (live_url, other_url, closed_url) = (url(&live), url(&other), url(&closed));
        drop(closed);

        let picked = pick_relay(&[closed_url.clone(), live_url.clone(), other_url.clone()]).await;
        assert_eq!(picked.as_ref(), Some(&live_url));
        let picked = pick_relay(&[other_url.clone(), live_url.clone()]).await;
        assert_eq!(picked.as_ref(), Some(&other_url));
        let unreachable = [closed_url.clone(), other_url.clone()];
        drop(other);
        assert_eq!(pick_relay(&unreachable).await, None);
        assert_eq!(pick_relay_or_first(&unreachable).await, unreachable[..1]);
        // 只有一个地址时不探测
        assert_eq!(
            pick_relay(std::slice::from_ref(&closed_url)).await,
            Some(closed_url)
        );
    }

    #[test]
    fn relay_denials_map_to_actionable_messages() {
        assert!(denial_message(DENY_TOKEN_INVALID).contains("重新生成"));
        assert!(denial_message(DENY_REVOKED).contains("移除"));
        assert!(
            unreachable_message(&["http://10.0.0.2:19160/".into()])
                .starts_with("连不上控制面的 relay http://10.0.0.2:19160/")
        );
    }
}
