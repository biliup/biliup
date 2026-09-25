//! biliup Fleet：一台控制面管理多台录播节点（#1744）。
//!
//! 控制面（`biliup server --controller`）与节点之间只有 iroh QUIC 连接，连接一律由节点发起，
//! 两端都经控制面内嵌的 relay 中转、能打洞时转直连。单机模式（不带 `--controller`、
//! 也没有 `data/node.json`）不创建 iroh 端点、不开任何端口、不写任何 Fleet 文件。
//!
//! 这一阶段只做加入、心跳与在线状态：不分派房间、不下发配置。

pub mod controller;
#[cfg(test)]
mod e2e_tests;
pub mod net;
pub mod node;
pub mod protocol;
pub mod relay;
pub mod store;
pub mod ticket;

use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionManager;
use crate::server::infrastructure::service_register::ServiceRegister;
use controller::{Controller, RelaySetup};
use error_stack::{ResultExt, bail};
use iroh::endpoint::{Connection, PortmapperConfig, QuicTransportConfig, VarInt, presets};
use iroh::{Endpoint, RelayMap, RelayMode, SecretKey};
use protocol::CloseCode;
use relay::{EmbeddedRelay, FleetAccess};
use sqlx::migrate::Migrator;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{error, info, warn};
use url::Url;

/// 控制面独立库的迁移，与主库 `migrations/` 各自编号
pub static FLEET_MIGRATOR: Migrator = sqlx::migrate!("./fleet_migrations");

pub const FLEET_DB: &str = "data/fleet.sqlite3";
pub const NODE_FILE: &str = "data/node.json";
pub const DEFAULT_RELAY_PORT: u16 = 19160;
/// 容器首次启动时自动 join 用的票据
pub const JOIN_TICKET_ENV: &str = "BILIUP_JOIN_TICKET";
/// 配合 [`JOIN_TICKET_ENV`]：为 `1` / `true` 时等同 `biliup node join --allow-hooks`
pub const JOIN_ALLOW_HOOKS_ENV: &str = "BILIUP_JOIN_ALLOW_HOOKS";

const KEEP_ALIVE: Duration = Duration::from_secs(5);
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);

/// `--relay-listen <addr|off>`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayListen {
    Addr(SocketAddr),
    Off,
}

impl Default for RelayListen {
    fn default() -> Self {
        RelayListen::Addr(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            DEFAULT_RELAY_PORT,
        ))
    }
}

impl FromStr for RelayListen {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.eq_ignore_ascii_case("off") {
            return Ok(RelayListen::Off);
        }
        if let Ok(port) = value.parse::<u16>() {
            return Ok(RelayListen::Addr(SocketAddr::new(
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                port,
            )));
        }
        value
            .parse()
            .map(RelayListen::Addr)
            .map_err(|_| format!("expected <ip:port>, <port> or off, got {value}"))
    }
}

/// `biliup server` 的 Fleet 参数
#[derive(Debug, Clone, Default)]
pub struct FleetOptions {
    /// `--controller`
    pub controller: bool,
    /// `--relay-listen`；`None` 为默认的 `0.0.0.0:19160`
    pub relay_listen: Option<RelayListen>,
    /// `--relay-url`（可重复）：写进票据的 relay 地址，覆盖自动列出的网卡地址
    pub relay_urls: Vec<Url>,
}

/// `/v1/me` 里的能力标记，前端据此显示「节点」菜单
#[derive(Debug, Clone, Copy, Default)]
pub struct FleetCapability {
    pub controller: bool,
}

/// 本进程在 Fleet 里的角色
#[derive(Clone, Default)]
pub enum Fleet {
    #[default]
    Standalone,
    Controller(Arc<Controller>),
    Node(Arc<Mutex<Option<node::NodeAgent>>>),
}

impl Fleet {
    pub fn capability(&self) -> FleetCapability {
        FleetCapability {
            controller: matches!(self, Fleet::Controller(_)),
        }
    }

    /// 控制面才有的 `/v1/fleet/*` 路由
    pub fn router(&self) -> Option<axum::Router<()>> {
        match self {
            Fleet::Controller(controller) => {
                Some(crate::server::api::fleet::router(controller.clone()))
            }
            _ => None,
        }
    }

    pub async fn shutdown(&self) {
        match self {
            Fleet::Standalone => {}
            Fleet::Controller(controller) => controller.shutdown().await,
            Fleet::Node(agent) => {
                let agent = agent.lock().unwrap().take();
                if let Some(agent) = agent {
                    agent.shutdown().await;
                }
            }
        }
    }
}

/// 按参数与工作目录决定本进程的角色。单机模式只检查 `data/node.json` 是否存在，不做别的事。
pub async fn start(options: &FleetOptions, services: &ServiceRegister) -> AppResult<Fleet> {
    let node_file = Path::new(NODE_FILE);
    if options.controller {
        if node_file.exists() {
            bail!(AppError::Custom(format!(
                "--controller 与 {NODE_FILE} 不能同时使用：这台机器已经作为节点加入了别的控制面，先执行 `biliup node leave`"
            )));
        }
        return start_controller(options).await.map(Fleet::Controller);
    }
    if options.relay_listen.is_some() || !options.relay_urls.is_empty() {
        warn!("--relay-listen / --relay-url 只在 --controller 时生效，已忽略");
    }
    if !node_file.exists() {
        let Some(ticket) = std::env::var(JOIN_TICKET_ENV)
            .ok()
            .filter(|ticket| !ticket.trim().is_empty())
        else {
            return Ok(Fleet::Standalone);
        };
        let allow_hooks = std::env::var(JOIN_ALLOW_HOOKS_ENV)
            .is_ok_and(|value| matches!(value.trim(), "1" | "true" | "yes"));
        info!("{JOIN_TICKET_ENV} is set and {NODE_FILE} is missing, joining the fleet controller");
        match node::join(&ticket, allow_hooks, node_file).await {
            Ok(file) => info!(node = file.node_id, "joined the fleet controller"),
            Err(e) => {
                error!(error = ?e, "自动加入控制面失败，本次以单机模式运行");
                return Ok(Fleet::Standalone);
            }
        }
    }
    match node::NodeAgent::start(node_file.to_path_buf(), services.clone()).await {
        Ok(agent) => Ok(Fleet::Node(Arc::new(Mutex::new(Some(agent))))),
        Err(e) => {
            error!(error = ?e, "节点代理没能启动，本次以单机模式运行");
            Ok(Fleet::Standalone)
        }
    }
}

async fn start_controller(options: &FleetOptions) -> AppResult<Arc<Controller>> {
    let pool = ConnectionManager::new_pool_with(FLEET_DB, &FLEET_MIGRATOR)
        .await
        .attach("could not open the fleet database")?;
    let secret = store::identity(&pool, now_ms()).await?;
    let listen = options.relay_listen.unwrap_or_default();
    let (relay, relays) = match listen {
        RelayListen::Addr(addr) => {
            let relay =
                EmbeddedRelay::spawn(addr, FleetAccess::new(secret.public(), pool.clone())).await?;
            let bound = relay.addr();
            let advertised = if !options.relay_urls.is_empty() {
                options.relay_urls.clone()
            } else if bound.ip().is_unspecified() {
                // 空着表示每次按网卡地址现列
                Vec::new()
            } else {
                vec![net::relay_url(bound.ip(), bound.port())]
            };
            let setup = RelaySetup {
                local: vec![net::local_relay_url(bound)],
                advertised,
                embedded_port: Some(bound.port()),
            };
            (Some(relay), setup)
        }
        RelayListen::Off => {
            if options.relay_urls.is_empty() {
                bail!(AppError::Custom(
                    "--relay-listen off 时必须用 --relay-url 指定外部 relay".into()
                ));
            }
            let setup = RelaySetup {
                local: options.relay_urls.clone(),
                advertised: options.relay_urls.clone(),
                embedded_port: None,
            };
            (None, setup)
        }
    };
    Controller::start(pool, secret, relays, relay).await
}

/// 控制面与节点共用的 iroh 端点配置：`presets::Minimal`、不做端口映射、不用 n0 的 relay 与 DNS 发现，
/// 只连给定的 relay；keep-alive 5 s、空闲 30 s 断开。`accept` 为真时接受 `biliup/fleet/1` 连接。
pub(crate) async fn bind_endpoint(
    secret: SecretKey,
    relays: RelayMap,
    accept: bool,
) -> AppResult<Endpoint> {
    let idle = IDLE_TIMEOUT
        .try_into()
        .change_context(AppError::Custom("invalid idle timeout".into()))?;
    let transport = QuicTransportConfig::builder()
        .keep_alive_interval(KEEP_ALIVE)
        .max_idle_timeout(Some(idle))
        .build();
    let mut builder = Endpoint::builder(presets::Minimal)
        .secret_key(secret)
        .relay_mode(RelayMode::Custom(relays))
        .portmapper_config(PortmapperConfig::Disabled)
        .transport_config(transport)
        .dns_resolver(net::dns_resolver());
    if accept {
        builder = builder.alpns(vec![protocol::ALPN.to_vec()]);
    }
    builder
        .bind()
        .await
        .change_context(AppError::Custom("could not bind the iroh endpoint".into()))
}

pub(crate) fn close_with(connection: &Connection, code: CloseCode) {
    connection.close(VarInt::from_u32(code as u32), code.as_str().as_bytes());
}

pub(crate) fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_listen_parses_addresses_ports_and_off() {
        assert_eq!("off".parse::<RelayListen>(), Ok(RelayListen::Off));
        assert_eq!("OFF".parse::<RelayListen>(), Ok(RelayListen::Off));
        assert_eq!(
            "19170".parse::<RelayListen>(),
            Ok(RelayListen::Addr("0.0.0.0:19170".parse().unwrap()))
        );
        assert_eq!(
            "192.168.1.2:19160".parse::<RelayListen>(),
            Ok(RelayListen::Addr("192.168.1.2:19160".parse().unwrap()))
        );
        assert!("nope".parse::<RelayListen>().is_err());
        assert_eq!(
            RelayListen::default(),
            RelayListen::Addr("0.0.0.0:19160".parse().unwrap())
        );
    }

    /// 已发布的 Fleet 迁移同样是只读历史，规矩与主库的 `shipped_migration_checksums_are_frozen` 相同：
    /// 修 SQL 只能新增迁移文件。
    #[test]
    fn shipped_fleet_migration_checksums_are_frozen() {
        const FROZEN: &[(i64, &str)] = &[(
            1,
            "ffd740cff5fdaeb33e832d290112ce69dbc46d8993f753d7d98acb6bd9b634cc929890bae23a4385280f18dafaabe8c1",
        )];
        let embedded: Vec<(i64, String)> = FLEET_MIGRATOR
            .iter()
            .map(|migration| (migration.version, store::hex(&migration.checksum)))
            .collect();
        let frozen: Vec<(i64, String)> = FROZEN
            .iter()
            .map(|(version, checksum)| (*version, (*checksum).to_string()))
            .collect();
        assert_eq!(embedded, frozen);
    }

    #[tokio::test]
    async fn fleet_database_is_separate_and_numbered_from_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fleet.sqlite3");
        let pool = ConnectionManager::new_pool_with(path.to_str().unwrap(), &FLEET_MIGRATOR)
            .await
            .unwrap();
        let versions: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(versions, [1]);
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name LIKE 'fleet_%' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            tables,
            ["fleet_identity", "fleet_join_tokens", "fleet_nodes"]
        );
        // 主库的表一张都不在这里
        let foreign: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sqlite_master WHERE name IN ('livestreamers', 'web_users', 'clips')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(foreign, 0);
        pool.close().await;
        // 重开不会重复迁移
        let pool = ConnectionManager::new_pool_with(path.to_str().unwrap(), &FLEET_MIGRATOR)
            .await
            .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}
