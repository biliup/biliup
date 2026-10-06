//! 控制面进程内嵌的「本机」节点（F5，#1744 D14）：控制面自己也录 Fleet 房间。
//!
//! 在「节点」页启用后，控制面生成一把节点密钥、直接登记进 `fleet_nodes`（不用票据），写
//! `data/local-node.json`，再在本进程里起一个与远端节点完全相同的节点代理，经本机回环连自己的内嵌 relay。
//! 分派、心跳、`Ack`、事件、断线重连与重启后按缓存对账都走现成的节点代码，协议不变。
//!
//! 与远端节点只有两处不同：控制面不给它下发 Fleet 配置（录制用控制面自己的「空间配置」），
//! 以及关掉它时先把房间交出去、等它确认释放再吊销（见 [`Controller::remove_node`]），不留暂停的本地行。
//!
//! 没启用时这里什么都不做：不生成密钥、不写文件、不开端口，控制面的行为与没有这个模块时一样。
//! 托管行的映射与被移除清单沿用节点的文件名（`data/fleet-state.json`、`data/fleet-revoked.json`）：
//! 控制面的工作目录不能同时是节点（`data/node.json`），不会撞。

use super::controller::Controller;
use super::guard::ManagedHandle;
use super::node::{NodeAgent, NodeFile};
use super::reconcile::{self, Reconciler};
use super::revoked::RevokedHandle;
use super::{now_ms, store};
use crate::server::errors::{AppError, AppResult};
use crate::server::infrastructure::service_register::ServiceRegister;
use error_stack::bail;
use iroh::SecretKey;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tracing::{error, info, warn};

pub const LOCAL_NODE_FILE: &str = "data/local-node.json";
/// 节点列表与托管行提示里的名字
pub const LOCAL_NODE_NAME: &str = "本机";
/// 吊销后等节点代理自己收尾（转本地、停止）多久，之后直接停掉它
const RETIRE_WAIT: Duration = Duration::from_secs(10);
const RETIRE_POLL: Duration = Duration::from_millis(100);

/// 控制面持有的「本机」节点
pub struct LocalNode {
    file: PathBuf,
    services: ServiceRegister,
    managed: ManagedHandle,
    revoked: RevokedHandle,
    agent: tokio::sync::Mutex<Option<NodeAgent>>,
    node_id: Mutex<Option<i64>>,
    /// 启用与收尾互斥
    busy: tokio::sync::Mutex<()>,
}

impl LocalNode {
    pub fn new(
        file: PathBuf,
        services: ServiceRegister,
        managed: ManagedHandle,
        revoked: RevokedHandle,
    ) -> Self {
        LocalNode {
            file,
            services,
            managed,
            revoked,
            agent: tokio::sync::Mutex::default(),
            node_id: Mutex::default(),
            busy: tokio::sync::Mutex::default(),
        }
    }

    pub fn services(&self) -> &ServiceRegister {
        &self.services
    }

    /// 启用中的「本机」节点 id
    pub fn node_id(&self) -> Option<i64> {
        *self.node_id.lock().unwrap()
    }

    fn state_path(&self) -> PathBuf {
        reconcile::state_path(&self.file)
    }

    /// 「本机」上哪些行归 Fleet 管（`data/fleet-state.json`）；没启用过时为空
    pub fn fleet_state(&self) -> reconcile::FleetState {
        reconcile::read_state(&self.state_path()).unwrap_or_default()
    }

    /// 控制面启动时：有 `local-node.json` 就接着跑，按缓存认回托管行后连上来对账
    pub async fn resume(&self, controller: &Controller) {
        if !self.file.exists() {
            return;
        }
        let file = match NodeFile::load(&self.file) {
            Ok(file) => file,
            Err(e) => {
                error!(error = ?e, "「本机」节点没能启动：{} 读不了", self.file.display());
                return;
            }
        };
        let row = match store::node(controller.pool(), file.node_id).await {
            Ok(row) => row,
            Err(e) => {
                error!(error = ?e, "「本机」节点没能启动：读不了节点表");
                return;
            }
        };
        let endpoint = file.secret().map(|secret| secret.public().to_string()).ok();
        let same_controller = file.controller == controller.endpoint_id().to_string();
        match row {
            Some(row) if same_controller && Some(&row.endpoint_id) == endpoint.as_ref() => {
                *self.node_id.lock().unwrap() = Some(row.id);
                if row.revoked_at.is_some() {
                    // 上次吊销后没来得及收尾
                    self.retire(&file.controller, false).await;
                    return;
                }
            }
            _ => {
                // 控制面的数据被重置过：托管行留作本地行
                warn!(
                    "{} does not match this controller, the rooms it managed are local rooms now",
                    self.file.display()
                );
                reconcile::forget(&self.state_path());
                self.remove_file();
                return;
            }
        }
        let relays = controller.local_relays();
        if file.relays != relays {
            let file = NodeFile { relays, ..file };
            if let Err(e) = file.save(&self.file) {
                warn!(error = ?e, "could not update {}", self.file.display());
            }
        }
        if let Err(e) = self.start_agent().await {
            error!(error = ?e, "「本机」节点代理没能启动");
        }
    }

    async fn start_agent(&self) -> AppResult<()> {
        let agent = NodeAgent::start(
            self.file.clone(),
            self.services.clone(),
            self.managed.clone(),
            self.revoked.clone(),
        )
        .await?;
        *self.agent.lock().await = Some(agent);
        Ok(())
    }

    /// 在「节点」页启用：登记节点、写 `local-node.json`、起节点代理。已经启用时返回 `Ok(None)`。
    pub async fn enable(
        &self,
        controller: &Controller,
        allow_hooks: bool,
    ) -> AppResult<Option<i64>> {
        let _busy = self.busy.lock().await;
        if self.node_id().is_some() {
            return Ok(None);
        }
        if self.file.exists() {
            bail!(AppError::Custom(format!(
                "{} 已存在但「本机」节点没有在运行，详见控制面启动日志",
                self.file.display()
            )));
        }
        // 上次的映射（例如收尾时进程被杀）不能被新的「本机」认领
        reconcile::forget(&self.state_path());
        let secret = SecretKey::generate();
        let row = store::insert_node(
            controller.pool(),
            &secret.public().to_string(),
            LOCAL_NODE_NAME,
            allow_hooks,
            now_ms(),
        )
        .await?;
        let file = NodeFile {
            local: true,
            ..NodeFile::new(
                controller.endpoint_id(),
                controller.local_relays(),
                row.id,
                &secret,
                allow_hooks,
            )
        };
        if let Err(e) = file.save(&self.file) {
            let _ = store::revoke_node(controller.pool(), row.id, now_ms()).await;
            return Err(e);
        }
        *self.node_id.lock().unwrap() = Some(row.id);
        info!(node = row.id, allow_hooks, "「本机」节点已启用");
        if let Err(e) = self.start_agent().await {
            error!(error = ?e, "「本机」节点代理没能启动");
            bail!(AppError::Custom(
                "「本机」节点已登记，但节点代理没能启动，详见日志；可以在节点列表里移除它".into()
            ));
        }
        Ok(Some(row.id))
    }

    /// 「本机」节点被吊销之后：吊销时连着就等节点代理自己收尾，等不到（或没连着）就停掉它、
    /// 替它把剩下的托管行转本地并暂停，最后删掉 `local-node.json`。
    pub async fn retire(&self, controller: &str, connected: bool) {
        let _busy = self.busy.lock().await;
        let agent = self.agent.lock().await.take();
        if let Some(agent) = agent {
            let deadline = tokio::time::Instant::now() + RETIRE_WAIT;
            while connected && !agent.is_finished() && tokio::time::Instant::now() < deadline {
                tokio::time::sleep(RETIRE_POLL).await;
            }
            agent.shutdown().await;
        }
        let state = self.state_path();
        if state.exists() {
            let file = NodeFile::load(&self.file).ok();
            let mut reconciler = Reconciler::resume(
                state,
                controller,
                LOCAL_NODE_NAME.to_string(),
                file.is_some_and(|file| file.allow_hooks),
                true,
                self.services.clone(),
                self.managed.clone(),
            )
            .await;
            let paused = reconciler.release_revoked(&self.revoked).await;
            if paused > 0 {
                warn!(
                    paused,
                    "「本机」节点没能在吊销前交出全部房间，这些主播已转为本地主播并暂停，确认后在「直播管理」手动恢复"
                );
            }
        }
        *self.managed.write().unwrap() = None;
        self.remove_file();
        *self.node_id.lock().unwrap() = None;
        info!("「本机」节点已关闭");
    }

    fn remove_file(&self) {
        remove(&self.file);
    }

    /// 控制面退出时（在停录制调度之前）停掉节点代理
    pub async fn shutdown(&self) {
        let agent = self.agent.lock().await.take();
        if let Some(agent) = agent {
            agent.shutdown().await;
        }
    }

    #[cfg(test)]
    pub(crate) async fn agent_running(&self) -> bool {
        self.agent
            .lock()
            .await
            .as_ref()
            .is_some_and(|agent| !agent.is_finished())
    }
}

fn remove(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warn!(error = %e, "could not remove {}", path.display()),
    }
}
