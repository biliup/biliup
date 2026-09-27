//! 控制面上的配对：按 `ha_pair` 起停主机（[`Primary`]）。
//!
//! 配对只在「本机」节点（F5）启用、且它就是 `ha_pair` 里的主机时生效；没有配对时这里什么都不做。

use super::primary::Primary;
use super::store::{self, Pair};
use super::wire::HaMessage;
use super::{Role, set_role};
use crate::server::errors::AppResult;
use crate::server::fleet::assignments;
use crate::server::fleet::controller::Controller;
use crate::server::fleet::protocol::{ControllerMessage, HA_SINCE};
use crate::server::infrastructure::service_register::ServiceRegister;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

pub struct Pairing {
    services: ServiceRegister,
    active: Mutex<Option<Active>>,
}

/// 生效中的配对
struct Active {
    pair: Pair,
    primary: Arc<Primary>,
}

impl Pairing {
    pub fn new(services: ServiceRegister) -> Self {
        Pairing {
            services,
            active: Mutex::default(),
        }
    }

    /// 生效中配对的主机，以及 `node` 是不是它的备机
    fn primary_for(&self, node: i64) -> Option<Arc<Primary>> {
        let active = self.active.lock().unwrap();
        let active = active.as_ref()?;
        (active.pair.standby_node_id == node).then(|| active.primary.clone())
    }

    /// 节点连上控制面（`Controller::session`）：是备机就接上主机。次版本低于 4 的节点不认识场次消息，不接
    pub fn node_connected(
        &self,
        node: i64,
        proto: u32,
        outbox: &mpsc::UnboundedSender<ControllerMessage>,
    ) {
        let Some(primary) = self.primary_for(node) else {
            return;
        };
        if proto < HA_SINCE {
            warn!(
                node,
                proto,
                "HA：备机的协议次版本低于 {HA_SINCE}，收不了场次消息，按备机离线处理；请把备机升级到与控制面相同的版本"
            );
            return;
        }
        primary.connected(outbox.clone());
    }

    /// 节点离线（它的连接从在线表里移除之后）
    pub fn node_offline(&self, node: i64) {
        if let Some(primary) = self.primary_for(node) {
            primary.disconnected();
        }
    }

    /// 节点发来的场次消息；不是备机的丢掉
    pub fn node_message(&self, node: i64, message: HaMessage) {
        match self.primary_for(node) {
            Some(primary) => primary.standby_message(message),
            None => debug!(
                node,
                kind = message.kind(),
                "HA frame from a node that is not the standby"
            ),
        }
    }

    /// 场次对齐的窗口：控制面自己的 `live_merge_minutes`
    fn window(&self) -> i64 {
        let minutes = self.services.config.read().unwrap().live_merge_minutes;
        i64::try_from(minutes.saturating_mul(60_000)).unwrap_or(i64::MAX)
    }

    /// 控制面启动时（「本机」节点恢复之后）
    pub async fn resume(&self, controller: &Controller) {
        let pair = match store::pair(controller.pool()).await {
            Ok(Some(pair)) => pair,
            Ok(None) => return,
            Err(e) => {
                error!(error = ?e, "HA：读不了配对设置，本次不启用一主一备");
                return;
            }
        };
        if controller.local_node_id() != Some(pair.primary_node_id) {
            warn!(
                primary = pair.primary_node_id,
                "HA：配对里的主机不是启用中的「本机」节点，配对暂不生效"
            );
            return;
        }
        if let Err(e) = self.activate(controller, &pair).await {
            error!(error = ?e, "HA：主机没能启动，本次不启用一主一备");
        }
    }

    async fn activate(&self, controller: &Controller, pair: &Pair) -> AppResult<()> {
        let primary = Primary::start(
            controller.pool().clone(),
            pair.mode,
            pair.params,
            self.window(),
        )
        .await?;
        primary.set_rooms(rooms(controller, pair.primary_node_id).await?);
        set_role(Some(Role::Primary(primary.clone())));
        let active = Active {
            pair: pair.clone(),
            primary,
        };
        if let Some(previous) = self.active.lock().unwrap().replace(active) {
            previous.primary.stop();
        }
        Ok(())
    }

    pub fn shutdown(&self) {
        if let Some(active) = self.active.lock().unwrap().take() {
            active.primary.stop();
            set_role(None);
        }
    }
}

/// 主机（「本机」节点）持有的房间：主播地址 → 控制面房间 id
async fn rooms(controller: &Controller, node: i64) -> AppResult<HashMap<String, i64>> {
    let (rooms, _) = assignments::desired_state(controller.pool(), node).await?;
    Ok(rooms
        .into_iter()
        .map(|room| (room.spec.url, room.id))
        .collect())
}
