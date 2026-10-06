//! 控制面内嵌的 iroh relay：纯 HTTP，与 axum 共用同一个 tokio runtime。
//!
//! relay 不做开放中转，只放行控制面自己、节点表里未吊销的公钥，以及在 Bearer 头里
//! 出示有效未过期 join token id 的连接（正在 join 的节点）。

use super::now_ms;
use super::store;
use super::ticket::is_token_id;
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::connection_pool::ConnectionPool;
use error_stack::ResultExt;
use iroh::EndpointId;
use iroh_relay::server::{Access, AccessControl, ClientRequest, RelayConfig, Server, ServerConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// 拒绝原因发回给客户端，节点据此给出中文提示；保持 ASCII、不带内部细节。
pub const DENY_TOKEN_INVALID: &str = "biliup-fleet: token-invalid";
pub const DENY_REVOKED: &str = "biliup-fleet: revoked";
pub const DENY_UNKNOWN: &str = "biliup-fleet: unknown-endpoint";
pub const DENY_INTERNAL: &str = "biliup-fleet: internal-error";

#[derive(Debug)]
pub struct FleetAccess {
    controller: EndpointId,
    pool: ConnectionPool,
}

impl FleetAccess {
    pub fn new(controller: EndpointId, pool: ConnectionPool) -> Self {
        FleetAccess { controller, pool }
    }

    async fn decide(
        &self,
        endpoint: EndpointId,
        token: Option<String>,
    ) -> Result<(), &'static str> {
        if endpoint == self.controller {
            return Ok(());
        }
        let node = store::node_by_endpoint(&self.pool, &endpoint.to_string())
            .await
            .map_err(|_| DENY_INTERNAL)?;
        match node {
            Some(node) if node.revoked_at.is_none() => return Ok(()),
            Some(_) => return Err(DENY_REVOKED),
            None => {}
        }
        let Some(token) = token else {
            return Err(DENY_UNKNOWN);
        };
        if !is_token_id(&token) {
            return Err(DENY_TOKEN_INVALID);
        }
        match store::token_is_open(&self.pool, &token, now_ms()).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(DENY_TOKEN_INVALID),
            Err(_) => Err(DENY_INTERNAL),
        }
    }
}

impl AccessControl for FleetAccess {
    async fn on_connect(&self, request: &ClientRequest) -> Access {
        let endpoint = request.endpoint_id();
        match self.decide(endpoint, request.auth_token()).await {
            Ok(()) => {
                debug!(endpoint = %endpoint.fmt_short(), "relay admitted");
                Access::Allow
            }
            Err(reason) => {
                warn!(endpoint = %endpoint.fmt_short(), reason, "relay rejected a connection");
                Access::Deny {
                    reason: Some(reason.to_string()),
                }
            }
        }
    }
}

pub struct EmbeddedRelay {
    server: Server,
    addr: SocketAddr,
}

impl EmbeddedRelay {
    pub async fn spawn(listen: SocketAddr, access: FleetAccess) -> AppResult<Self> {
        let mut relay = RelayConfig::new(listen);
        relay.access = Arc::new(access);
        let mut config = ServerConfig::default();
        config.relay = Some(relay);
        let server = Server::spawn(config)
            .await
            .change_context(AppError::Custom(format!(
                "could not start the embedded relay on {listen}"
            )))?;
        let addr = server.http_addr().unwrap_or(listen);
        info!(%addr, "fleet relay listening (plain HTTP)");
        Ok(EmbeddedRelay { server, addr })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn shutdown(self) {
        if let Err(e) = self.server.shutdown().await {
            warn!(error = %e, "embedded relay did not shut down cleanly");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::fleet::FLEET_MIGRATOR;
    use crate::server::infrastructure::connection_pool::ConnectionManager;
    use iroh::SecretKey;

    #[tokio::test]
    async fn admits_only_the_controller_nodes_and_open_tokens() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fleet.sqlite3");
        let pool = ConnectionManager::new_pool_with(path.to_str().unwrap(), &FLEET_MIGRATOR)
            .await
            .unwrap();
        let controller = SecretKey::generate().public();
        let access = FleetAccess::new(controller, pool.clone());

        assert_eq!(access.decide(controller, None).await, Ok(()));

        let stranger = SecretKey::generate().public();
        assert_eq!(access.decide(stranger, None).await, Err(DENY_UNKNOWN));
        assert_eq!(
            access.decide(stranger, Some("zzzzzz".into())).await,
            Err(DENY_TOKEN_INVALID)
        );
        assert_eq!(
            access.decide(stranger, Some("not a token".into())).await,
            Err(DENY_TOKEN_INVALID)
        );

        let now = now_ms();
        let (token, secret) = store::create_token(&pool, None, now, now + 60_000)
            .await
            .unwrap();
        assert_eq!(
            access.decide(stranger, Some(token.id.clone())).await,
            Ok(())
        );

        let store::Redeem::Joined(node) = store::redeem_token(
            &pool,
            &token.id,
            &secret,
            &stranger.to_string(),
            "n",
            false,
            now,
        )
        .await
        .unwrap() else {
            panic!()
        };
        // 令牌用掉之后，靠节点表放行
        assert_eq!(access.decide(stranger, None).await, Ok(()));
        let other = SecretKey::generate().public();
        assert_eq!(
            access.decide(other, Some(token.id.clone())).await,
            Err(DENY_TOKEN_INVALID)
        );

        store::revoke_node(&pool, node.id, now).await.unwrap();
        assert_eq!(access.decide(stranger, None).await, Err(DENY_REVOKED));
    }
}
