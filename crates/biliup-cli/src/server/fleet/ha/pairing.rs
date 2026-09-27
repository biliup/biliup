//! 控制面上的配对：主副指定（`/v1/fleet/ha`）、按 `ha_pair` 起停主机（[`Primary`]），
//! 以及把主机「本机」持有的房间与模板镜像给备机。
//!
//! 镜像不改 `fleet_rooms` 的分派（不占 F2 的「一房间一节点」）：给备机下发期望状态时，把主机「本机」
//! 此刻的房间与它们用的模板一起放进去，再带上 [`HaAssignment`]。边录边传的房间不镜像（§5.1）。
//!
//! 配对只在「本机」节点（F5）启用、且它就是 `ha_pair` 里的主机时生效；没有配对时这里什么都不做，
//! 下发的期望状态与以前逐字相同。

use super::params::{HaMode, HaParams};
use super::primary::Primary;
use super::store::{self, Pair};
use super::wire::{HaAssignment, HaMessage, ManualAction};
use super::{Role, set_role, sync_downloader};
use crate::server::config::Config;
use crate::server::errors::AppResult;
use crate::server::fleet::assignments;
use crate::server::fleet::controller::Controller;
use crate::server::fleet::model::{DesiredRoom, DesiredTemplate};
use crate::server::fleet::now_ms;
use crate::server::fleet::protocol::{ControllerMessage, HA_SINCE};
use crate::server::infrastructure::service_register::ServiceRegister;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// 控制面启动时，给节点下发期望状态之前最多等配对载入这么久
const READY_WAIT: Duration = Duration::from_secs(30);
/// `GET /v1/fleet/ha` 带多少条最近的场次
const RECENT_SESSIONS: i64 = 200;

pub struct Pairing {
    services: ServiceRegister,
    active: Mutex<Option<Active>>,
    /// 启动时载入过 `ha_pair`。之前连上来的备机先等一等，免得收到不带配对的期望状态而解除
    ready: watch::Sender<bool>,
    /// 指定与解除一个一个来
    busy: tokio::sync::Mutex<()>,
    /// 控制面的配置改了就给主机与备机重发期望状态（边录边传的房间可能变了）
    watcher: Mutex<Option<JoinHandle<()>>>,
}

/// 生效中的配对
#[derive(Clone)]
struct Active {
    pair: Pair,
    primary: Arc<Primary>,
}

/// `PUT /v1/fleet/ha`
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Designate {
    /// 备机的节点 id
    pub standby: i64,
    pub mode: HaMode,
    /// 不填时沿用现有配对的参数（没有配对时用默认值）；填了就整份替换，缺的字段取默认值
    #[serde(default)]
    pub params: Option<HaParams>,
}

/// 不能指定、不能转发的原因
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    Invalid(String),
    NotFound(String),
    Conflict(String),
}

/// 不纳入配对的房间
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Excluded {
    pub id: i64,
    pub url: String,
    pub remark: String,
    pub reason: &'static str,
}

impl Excluded {
    fn sync_downloader(room: &DesiredRoom) -> Self {
        Excluded {
            id: room.id,
            url: room.spec.url.clone(),
            remark: room.spec.remark.clone(),
            reason: "边录边传（sync-downloader）的房间不纳入一主一备：两台同时录会出两份稿件，只由主机录",
        }
    }
}

/// 主机「本机」持有的房间里纳入配对的与不纳入的（按主机的配置叠上房间覆写判断边录边传）
fn split(config: &Config, rooms: Vec<DesiredRoom>) -> (Vec<DesiredRoom>, Vec<Excluded>) {
    let mut kept = Vec::new();
    let mut excluded = Vec::new();
    for room in rooms {
        if sync_downloader(config, room.spec.override_cfg.clone()) {
            excluded.push(Excluded::sync_downloader(&room));
        } else {
            kept.push(room);
        }
    }
    (kept, excluded)
}

/// 主机的钩子认的房间：主播地址 → 控制面房间 id
fn room_map(rooms: &[DesiredRoom]) -> HashMap<String, i64> {
    rooms
        .iter()
        .map(|room| (room.spec.url.clone(), room.id))
        .collect()
}

/// 备机的期望状态里加上镜像的房间与它们用的模板
fn mirror(
    pair: &Pair,
    kept: Vec<DesiredRoom>,
    primary_templates: Vec<DesiredTemplate>,
    rooms: &mut Vec<DesiredRoom>,
    templates: &mut Vec<DesiredTemplate>,
) -> HaAssignment {
    let used: BTreeSet<i64> = kept.iter().filter_map(|room| room.template_id).collect();
    for template in primary_templates {
        if used.contains(&template.id) && !templates.iter().any(|t| t.id == template.id) {
            templates.push(template);
        }
    }
    let ids = kept.iter().map(|room| room.id).collect();
    for room in kept {
        if !rooms.iter().any(|r| r.id == room.id) {
            rooms.push(room);
        }
    }
    HaAssignment::of(pair, ids)
}

/// 指定备机之前的检查：主机是启用中的「本机」，备机是一台在线、协议次版本 ≥ 4 的普通节点。
/// 只改模式或参数（备机不变）时备机可以不在线，连上来时收到新的参数
/// `node` 是备机候选的节点名与是否已被移除
fn refusal(
    local: Option<i64>,
    standby: i64,
    node: Option<(&str, bool)>,
    proto: Option<u32>,
    same_standby: bool,
) -> Option<Refused> {
    let Some(local) = local else {
        return Some(Refused::Conflict(
            "一主一备的主机是控制面的「本机」节点：先在节点页启用「本机」".into(),
        ));
    };
    if standby == local {
        return Some(Refused::Invalid(
            "备机不能是「本机」节点：选一台普通节点".into(),
        ));
    }
    let Some((name, revoked)) = node else {
        return Some(Refused::NotFound(format!("没有节点 {standby}")));
    };
    if revoked {
        return Some(Refused::Conflict(format!("节点「{name}」已被移除")));
    }
    match proto {
        Some(proto) if proto < HA_SINCE => Some(Refused::Conflict(format!(
            "节点「{name}」的协议次版本是 {proto}，低于 {HA_SINCE}：它不认识场次消息，不能当备机。请先把它升级到与控制面相同的版本"
        ))),
        None if !same_standby => Some(Refused::Conflict(format!(
            "节点「{name}」不在线：指定备机时要确认它的协议次版本不低于 {HA_SINCE}，请等它连上再指定"
        ))),
        _ => None,
    }
}

impl Pairing {
    pub fn new(services: ServiceRegister) -> Self {
        Pairing {
            services,
            active: Mutex::default(),
            ready: watch::channel(false).0,
            busy: tokio::sync::Mutex::default(),
            watcher: Mutex::default(),
        }
    }

    fn active(&self) -> Option<Active> {
        self.active.lock().unwrap().clone()
    }

    /// 生效中配对的主机，以及 `node` 是不是它的备机
    fn primary_for(&self, node: i64) -> Option<Arc<Primary>> {
        let active = self.active.lock().unwrap();
        let active = active.as_ref()?;
        (active.pair.standby_node_id == node).then(|| active.primary.clone())
    }

    fn config(&self) -> Config {
        self.services.config.read().unwrap().clone()
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

    /// 控制面启动时（「本机」节点恢复之后，`local` 是它的 id）。载入之后才给备机下发期望状态
    pub async fn resume(self: &Arc<Self>, controller: &Arc<Controller>, local: Option<i64>) {
        self.load(controller, local).await;
        self.ready.send_replace(true);
        let task = tokio::spawn(watch_config(
            Arc::downgrade(self),
            Arc::downgrade(controller),
        ));
        if let Some(previous) = self.watcher.lock().unwrap().replace(task) {
            previous.abort();
        }
    }

    async fn load(&self, controller: &Controller, local: Option<i64>) {
        let pair = match store::pair(controller.pool()).await {
            Ok(Some(pair)) => pair,
            Ok(None) => return,
            Err(e) => {
                error!(error = ?e, "HA：读不了配对设置，本次不启用一主一备");
                return;
            }
        };
        if local != Some(pair.primary_node_id) {
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

    async fn wait_ready(&self) {
        let mut ready = self.ready.subscribe();
        if tokio::time::timeout(READY_WAIT, ready.wait_for(|ready| *ready))
            .await
            .is_err()
        {
            warn!("HA：配对还没载入完，先按没有配对下发");
            self.ready.send_replace(true);
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
        let (rooms, _) =
            assignments::desired_state(controller.pool(), pair.primary_node_id).await?;
        let (kept, excluded) = split(&self.config(), rooms);
        for room in &excluded {
            warn!(room = room.id, url = room.url, "HA：{}", room.reason);
        }
        primary.set_rooms(room_map(&kept));
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

    fn deactivate(&self) {
        if let Some(active) = self.active.lock().unwrap().take() {
            active.primary.stop();
            set_role(None);
            info!(standby = active.pair.standby_node_id, "HA：配对已解除");
        }
    }

    /// 给节点下发期望状态时（`Controller::push_locked`）。主机「本机」：刷新主机钩子认的房间；
    /// 次版本 ≥ 4 的备机：加上镜像的房间与模板，返回要带的 [`HaAssignment`]；其余节点原样
    pub async fn desired(
        &self,
        controller: &Controller,
        node: i64,
        proto: u32,
        rooms: &mut Vec<DesiredRoom>,
        templates: &mut Vec<DesiredTemplate>,
    ) -> AppResult<Option<HaAssignment>> {
        self.wait_ready().await;
        let Some(active) = self.active() else {
            return Ok(None);
        };
        if node == active.pair.primary_node_id {
            let config = self.config();
            let kept: Vec<DesiredRoom> = rooms
                .iter()
                .filter(|room| !sync_downloader(&config, room.spec.override_cfg.clone()))
                .cloned()
                .collect();
            active.primary.set_rooms(room_map(&kept));
            return Ok(None);
        }
        if node != active.pair.standby_node_id || proto < HA_SINCE {
            return Ok(None);
        }
        let (primary_rooms, primary_templates) =
            assignments::desired_state(controller.pool(), active.pair.primary_node_id).await?;
        let (kept, excluded) = split(&self.config(), primary_rooms);
        if !excluded.is_empty() {
            debug!(
                rooms = ?excluded.iter().map(|room| room.id).collect::<Vec<_>>(),
                "HA：边录边传的房间不镜像给备机"
            );
        }
        Ok(Some(mirror(
            &active.pair,
            kept,
            primary_templates,
            rooms,
            templates,
        )))
    }

    /// 要给主机「本机」重发期望状态时，备机的镜像也跟着重发
    pub fn mirror_target(&self, nodes: &[i64]) -> Option<i64> {
        let active = self.active.lock().unwrap();
        let pair = &active.as_ref()?.pair;
        nodes
            .contains(&pair.primary_node_id)
            .then_some(pair.standby_node_id)
    }

    /// 指定备机、改模式或参数。备机已经连着时先接上主机，再给它下发带配对的期望状态
    pub async fn designate(
        &self,
        controller: &Controller,
        request: Designate,
    ) -> AppResult<Result<Pair, Refused>> {
        self.wait_ready().await;
        let _busy = self.busy.lock().await;
        let pool = controller.pool();
        let previous = store::pair(pool).await?;
        let params = request
            .params
            .or(previous.as_ref().map(|pair| pair.params))
            .unwrap_or_default();
        if let Err(e) = params.validate() {
            return Ok(Err(Refused::Invalid(e)));
        }
        let local = controller.local_node_id();
        let node = crate::server::fleet::store::node(pool, request.standby).await?;
        let same_standby = previous
            .as_ref()
            .is_some_and(|pair| pair.standby_node_id == request.standby);
        let link = controller.node_link(request.standby);
        if let Some(refused) = refusal(
            local,
            request.standby,
            node.as_ref()
                .map(|node| (node.name.as_str(), node.revoked_at.is_some())),
            link.as_ref().map(|(proto, _)| *proto),
            same_standby,
        ) {
            return Ok(Err(refused));
        }
        let Some(local) = local else {
            unreachable!("refusal() checks the local node");
        };
        let pair = store::save_pair(
            pool,
            local,
            request.standby,
            request.mode,
            &params,
            now_ms(),
        )
        .await?;
        let current = self.active();
        let replaced = current
            .as_ref()
            .map(|active| active.pair.standby_node_id)
            .filter(|standby| *standby != pair.standby_node_id);
        match current {
            Some(active)
                if replaced.is_none() && active.pair.primary_node_id == pair.primary_node_id =>
            {
                active.primary.configure(pair.mode, pair.params);
                *self.active.lock().unwrap() = Some(Active {
                    pair: pair.clone(),
                    primary: active.primary,
                });
            }
            _ => {
                if let Err(e) = self.activate(controller, &pair).await {
                    let _ = store::clear_pair(pool).await;
                    self.deactivate();
                    return Err(e);
                }
                if let Some((proto, outbox)) = &link {
                    self.node_connected(pair.standby_node_id, *proto, outbox);
                }
            }
        }
        info!(
            primary = pair.primary_node_id,
            standby = pair.standby_node_id,
            mode = %pair.mode,
            "HA：已指定备机"
        );
        controller
            .push_many([replaced, Some(pair.standby_node_id)])
            .await;
        Ok(Ok(pair))
    }

    /// 解除配对：主机停下，给备机下发不带配对、不带镜像房间的期望状态。本来就没有配对时返回 `false`
    pub async fn dissolve(&self, controller: &Controller) -> AppResult<bool> {
        self.wait_ready().await;
        let _busy = self.busy.lock().await;
        let pair = store::pair(controller.pool()).await?;
        store::clear_pair(controller.pool()).await?;
        self.deactivate();
        let Some(pair) = pair else {
            return Ok(false);
        };
        controller.push_many([Some(pair.standby_node_id)]).await;
        Ok(true)
    }

    /// 节点被移除（吊销）时：它在配对里就解除配对（之后的全量下发会让备机解除）
    pub async fn node_removed(&self, controller: &Controller, node: i64) {
        let _busy = self.busy.lock().await;
        match store::pair(controller.pool()).await {
            Ok(Some(pair)) if pair.primary_node_id == node || pair.standby_node_id == node => {
                if let Err(e) = store::clear_pair(controller.pool()).await {
                    warn!(error = ?e, "HA：没能删掉配对设置");
                }
                self.deactivate();
                warn!(node, "HA：配对里的节点被移除，配对已解除");
            }
            Ok(_) => {}
            Err(e) => warn!(error = ?e, "HA：读不了配对设置"),
        }
    }

    /// 主机面板上点的人工处理，转给备机执行
    pub fn manual(&self, key: &str, action: ManualAction) -> Result<(), Refused> {
        let Some(active) = self.active() else {
            return Err(Refused::Conflict("没有生效中的配对".into()));
        };
        let message = HaMessage::Manual {
            key: key.to_string(),
            action,
        };
        if active.primary.forward(message) {
            info!(key, ?action, "HA：人工处理已转给备机");
            Ok(())
        } else {
            Err(Refused::Conflict(
                "备机不在线：到备机本地的节点页上处理这一场".into(),
            ))
        }
    }

    /// `GET /v1/fleet/ha`
    pub async fn view(&self, controller: &Controller) -> AppResult<Value> {
        let pool = controller.pool();
        let pair = store::pair(pool).await?;
        let active = self.active();
        let local = controller.local_node_id();
        let standby = pair.as_ref().map(|pair| {
            let link = controller.node_link(pair.standby_node_id);
            json!({
                "id": pair.standby_node_id,
                "online": link.is_some(),
                "proto": link.map(|(proto, _)| proto),
                "linked": active.as_ref().is_some_and(|active| active.primary.linked()),
                "reported": active.as_ref().is_some_and(|active| active.primary.reported()),
            })
        });
        let excluded = match pair.as_ref().map(|pair| pair.primary_node_id).or(local) {
            Some(node) => {
                let (rooms, _) = assignments::desired_state(pool, node).await?;
                split(&self.config(), rooms).1
            }
            None => Vec::new(),
        };
        let sessions = store::recent_sessions(pool, RECENT_SESSIONS).await?;
        Ok(json!({
            "pair": pair,
            "active": active.is_some(),
            "local_node": local,
            "standby": standby,
            "min_proto": HA_SINCE,
            "excluded": excluded,
            "sessions": sessions,
        }))
    }

    pub fn shutdown(&self) {
        if let Some(task) = self.watcher.lock().unwrap().take() {
            task.abort();
        }
        self.deactivate();
    }
}

async fn watch_config(pairing: Weak<Pairing>, controller: Weak<Controller>) {
    let mut changes = super::config_changes();
    while changes.changed().await.is_ok() {
        let (Some(pairing), Some(controller)) = (pairing.upgrade(), controller.upgrade()) else {
            return;
        };
        if let Some(active) = pairing.active() {
            debug!("HA：控制面的配置改了，重发主机与备机的期望状态");
            controller
                .push_many([Some(active.pair.primary_node_id)])
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn room(id: i64, url: &str, template: Option<i64>, downloader: Option<&str>) -> DesiredRoom {
        serde_json::from_value(json!({
            "id": id, "epoch": 1, "template_id": template, "url": url, "remark": format!("房间{id}"),
            "override": downloader.map(|downloader| json!({ "downloader": downloader })),
        }))
        .unwrap()
    }

    fn template(id: i64) -> DesiredTemplate {
        serde_json::from_value(json!({ "id": id, "template_name": format!("模板{id}") })).unwrap()
    }

    fn pair() -> Pair {
        serde_json::from_value(json!({
            "primary_node_id": 1, "standby_node_id": 5, "mode": 1, "params": {},
            "created_at": 0, "updated_at": 0,
        }))
        .unwrap()
    }

    #[test]
    fn a_standby_must_be_an_online_current_node_other_than_the_local_one() {
        let alive = Some(("备", false));
        assert!(matches!(
            refusal(None, 5, alive, Some(4), false),
            Some(Refused::Conflict(m)) if m.contains("启用「本机」")
        ));
        assert!(matches!(
            refusal(Some(5), 5, alive, Some(4), false),
            Some(Refused::Invalid(m)) if m.contains("不能是「本机」")
        ));
        assert!(matches!(
            refusal(Some(1), 5, None, None, false),
            Some(Refused::NotFound(_))
        ));
        assert!(matches!(
            refusal(Some(1), 5, Some(("备", true)), Some(4), false),
            Some(Refused::Conflict(m)) if m.contains("已被移除")
        ));
        assert!(matches!(
            refusal(Some(1), 5, alive, None, false),
            Some(Refused::Conflict(m)) if m.contains("不在线")
        ));
        // 旧版本节点：明确说出次版本与原因
        assert!(matches!(
            refusal(Some(1), 5, alive, Some(3), false),
            Some(Refused::Conflict(m)) if m.contains("协议次版本是 3") && m.contains("低于 4")
        ));
        assert!(matches!(
            refusal(Some(1), 5, alive, Some(3), true),
            Some(Refused::Conflict(_))
        ));
        assert_eq!(refusal(Some(1), 5, alive, Some(4), false), None);
        // 只改参数：备机不在线也行
        assert_eq!(refusal(Some(1), 5, alive, None, true), None);
    }

    #[test]
    fn sync_downloader_rooms_are_neither_paired_nor_mirrored() {
        let config = Config::default();
        let rooms = vec![
            room(7, "https://a/7", Some(1), None),
            room(8, "https://a/8", Some(2), Some("sync-downloader")),
            room(9, "https://a/9", None, Some("ffmpeg")),
        ];
        let (kept, excluded) = split(&config, rooms.clone());
        assert_eq!(kept.iter().map(|r| r.id).collect::<Vec<_>>(), [7, 9]);
        assert_eq!(excluded.len(), 1);
        assert_eq!(excluded[0].id, 8);
        assert!(excluded[0].reason.contains("边录边传"));
        assert_eq!(
            room_map(&kept),
            HashMap::from([
                ("https://a/7".to_string(), 7),
                ("https://a/9".to_string(), 9)
            ])
        );

        // 全局就是边录边传：没有覆写的房间都不纳入，覆写成别的下载器的照常
        let sync = Config {
            downloader: Some(crate::server::core::downloader::DownloaderType::SyncDownloader),
            ..Config::default()
        };
        let (kept, excluded) = split(&sync, rooms);
        assert_eq!(kept.iter().map(|r| r.id).collect::<Vec<_>>(), [9]);
        assert_eq!(excluded.len(), 2);
    }

    #[test]
    fn the_standby_gets_the_mirrored_rooms_and_their_templates_once() {
        let mut rooms = vec![room(3, "https://b/3", Some(2), None)];
        let mut templates = vec![template(2)];
        let kept = vec![
            room(7, "https://a/7", Some(1), None),
            room(9, "https://a/9", Some(2), None),
            room(10, "https://a/10", None, None),
        ];
        let assignment = mirror(
            &pair(),
            kept,
            vec![template(1), template(2), template(4)],
            &mut rooms,
            &mut templates,
        );
        assert_eq!(assignment.rooms, [7, 9, 10]);
        assert_eq!(assignment.primary, 1);
        assert_eq!(assignment.mode, HaMode::DualRecord);
        assert_eq!(
            rooms.iter().map(|r| r.id).collect::<Vec<_>>(),
            [3, 7, 9, 10]
        );
        assert_eq!(
            templates.iter().map(|t| t.id).collect::<Vec<_>>(),
            [2, 1],
            "只带镜像房间用到的模板，已有的不重复"
        );
    }
}
