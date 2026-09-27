//! 控制面上的配对：主副指定（`/v1/fleet/ha`）、按 `ha_pair` 起停上传的一侧（通常是主机 [`Primary`]），
//! 以及把主机「本机」持有的房间与模板镜像给备机。
//!
//! 镜像不改 `fleet_rooms` 的分派（不占 F2 的「一房间一节点」）：给备机下发期望状态时，把主机「本机」
//! 此刻的房间与它们用的模板一起放进去，再带上 [`HaAssignment`]。边录边传的房间不镜像（§5.1）。
//!
//! 配对只在「本机」节点（F5）启用、且它就是 `ha_pair` 里的主机时生效；没有配对时这里什么都不做，
//! 下发的期望状态与以前逐字相同。
//!
//! 备机次版本 ≥ 5 时两台双向同步（H2，[`super::member`]）：镜像的房间与模板在备机上可以改，备机的修改由这里
//! 仲裁后改 Fleet 房间（[`super::rooms`]）；每次给配对里的一台下发之前先认出控制面上的改动（[`Pairing::rescan`]）。
//!
//! 上传主备可以对调（[`Pairing::switch`]，H2）：两台都在线、各自没有做到一半的场次时，控制面把 `ha_pair` 的
//! 上传主机改成那台节点，自己改跑备机（[`Standby`]，场次记在 `data/ha-state.json`），再给节点下发带
//! `leader` 的配对。控制面仍是控制面、房间仍归「本机」节点，只是「这一场谁先投」换了过来。

use super::adopt;
use super::agent::{self, Mirrored, STATE_FILE_NAME, Standby};
use super::member::Member;
use super::params::{HaMode, HaParams};
use super::primary::Primary;
use super::rooms::{Arbiter, PairSet};
use super::store::{self, Pair};
use super::sync::{
    HaChange, Inventory, InventoryAsk, LocalRow, PairMessage, PairState, ROOM, Side, TEMPLATE,
};
use super::wire::{HaAssignment, HaMessage, ManualAction};
use super::{Link, Role, set_role, sync_downloader};
use crate::server::config::Config;
use crate::server::errors::{AppError, AppResult};
use crate::server::fleet::assignments;
use crate::server::fleet::controller::Controller;
use crate::server::fleet::model::{DesiredRoom, DesiredTemplate};
use crate::server::fleet::now_ms;
use crate::server::fleet::protocol::{ControllerMessage, HA_SINCE, PAIR_SINCE};
use crate::server::infrastructure::service_register::ServiceRegister;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// 控制面启动时，给节点下发期望状态之前最多等配对载入这么久
const READY_WAIT: Duration = Duration::from_secs(30);
/// `GET /v1/fleet/ha` 带多少条最近的场次
const RECENT_SESSIONS: i64 = 200;
/// 问节点本地行清单最多等多久
const INVENTORY_WAIT: Duration = Duration::from_secs(10);

pub struct Pairing {
    services: ServiceRegister,
    /// `data/`：同步账本 `pair-outbox.json` 与控制面当备机时的 `ha-state.json` 放在这里
    dir: PathBuf,
    active: Mutex<Option<Active>>,
    /// 解除配对时控制面正当备机：录到一半的一段只留在本地（[`Standby::retire`]），角色留到下次指定或退出
    retired: Mutex<Option<Arc<Standby>>>,
    /// 启动时载入过 `ha_pair`。之前连上来的备机先等一等，免得收到不带配对的期望状态而解除
    ready: watch::Sender<bool>,
    /// 指定、解除与换上传主机一个一个来
    busy: tokio::sync::Mutex<()>,
    /// 控制面的配置改了就给主机与备机重发期望状态（边录边传的房间可能变了）
    watcher: Mutex<Option<JoinHandle<()>>>,
    /// 问节点本地行清单还没回话的：（节点, 问的 id）→ 等回话的
    inventories: Mutex<HashMap<(i64, u64), oneshot::Sender<Inventory>>>,
    next_ask: AtomicU64,
}

/// 生效中的配对
#[derive(Clone)]
struct Active {
    pair: Pair,
    upload: Upload,
    /// 与配对节点的双向同步（H2）
    member: Arc<Member>,
}

/// 控制面进程在上传上的一侧：通常是主机，上传主机换到节点后是备机
#[derive(Clone)]
enum Upload {
    Primary(Arc<Primary>),
    Standby(Arc<Standby>),
}

impl Upload {
    fn stop(&self) {
        match self {
            Upload::Primary(primary) => primary.stop(),
            Upload::Standby(standby) => standby.stop(),
        }
    }

    /// 与节点那一侧的场次连接接着
    fn linked(&self) -> bool {
        match self {
            Upload::Primary(primary) => primary.linked(),
            Upload::Standby(standby) => standby.linked(),
        }
    }

    /// 主机收到了备机的上报（控制面当备机时连上就上报了）
    fn reported(&self) -> bool {
        match self {
            Upload::Primary(primary) => primary.reported(),
            Upload::Standby(standby) => standby.linked(),
        }
    }

    fn busy(&self) -> Option<String> {
        match self {
            Upload::Primary(primary) => primary.busy(),
            Upload::Standby(standby) => standby.busy(),
        }
    }

    fn link_down(&self) {
        match self {
            Upload::Primary(primary) => primary.disconnected(),
            Upload::Standby(standby) => standby.link_down(),
        }
    }

    fn configure(&self, mode: HaMode, params: HaParams) {
        match self {
            Upload::Primary(primary) => primary.configure(mode, params),
            Upload::Standby(standby) => standby.configure(mode, params),
        }
    }

    /// 刷新钩子认的房间（主机「本机」此刻纳入配对的房间）
    fn set_rooms(&self, kept: &[DesiredRoom]) {
        match self {
            Upload::Primary(primary) => primary.set_rooms(room_map(kept)),
            Upload::Standby(standby) => standby.set_rooms(standby_rooms(kept)),
        }
    }

    fn side(&self) -> &'static str {
        match self {
            Upload::Primary(_) => "primary",
            Upload::Standby(_) => "standby",
        }
    }
}

/// 控制面上同步账本的对端标识；控制面当备机时 `ha-state.json` 也按它认归属
fn peer(standby: i64) -> String {
    format!("node:{standby}")
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
    /// 备机上已有的本地主播与模板里纳入配对的。指定一台新的备机时不填就是全部；
    /// 备机不变（只改模式或参数）时不填就不纳入
    #[serde(default)]
    pub adopt: Option<Selection>,
}

/// 纳入配对的本地主播与模板 id；某一项不填就是这一项全部
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    #[serde(default)]
    pub streamers: Option<Vec<i64>>,
    #[serde(default)]
    pub templates: Option<Vec<i64>>,
}

/// `POST /v1/fleet/ha/role`
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Switch {
    /// 换成由这一台上传：`controller`（「本机」节点）或 `node`（配对里的那台节点）
    pub primary: Side,
}

/// 不能指定、不能转发的原因
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    Invalid(String),
    NotFound(String),
    Conflict(String),
}

impl Refused {
    pub fn message(self) -> String {
        match self {
            Refused::Invalid(message) | Refused::NotFound(message) | Refused::Conflict(message) => {
                message
            }
        }
    }
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

/// 控制面当备机时钩子认的房间：主播地址 → 房间 id 与覆写
fn standby_rooms(rooms: &[DesiredRoom]) -> HashMap<String, Mirrored> {
    rooms
        .iter()
        .map(|room| {
            (
                room.spec.url.clone(),
                Mirrored::new(room.id, room.spec.override_cfg.clone()),
            )
        })
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

/// 期望状态里加上这些模板，已有的不重复
fn add_templates(templates: &mut Vec<DesiredTemplate>, extra: Vec<DesiredTemplate>) {
    for template in extra {
        if !templates.iter().any(|t| t.id == template.id) {
            templates.push(template);
        }
    }
}

/// 单独加入配对的模板（没有房间用也在配对里，[`adopt`]）；控制面上已经删掉的不算
async fn pinned_templates(
    controller: &Controller,
    member: &Member,
) -> AppResult<Vec<DesiredTemplate>> {
    let mut templates = Vec::new();
    for id in member.pinned().await {
        if let Some(template) = assignments::template(controller.pool(), id).await? {
            templates.push(template.desired());
        }
    }
    Ok(templates)
}

/// 控制面房间列表里的直播间地址 → 房间 id（删到一半的也算）
async fn fleet_urls(controller: &Controller) -> AppResult<HashMap<String, i64>> {
    Ok(assignments::list_rooms(controller.pool())
        .await?
        .into_iter()
        .map(|room| (room.spec.url, room.id))
        .collect())
}

/// 一台机器上的本地行清单与这一次纳入的（`GET /v1/fleet/ha/candidates`、指定备机与加入接口的应答）
fn rows_view(node: Option<i64>, (streamers, templates): (Vec<LocalRow>, Vec<LocalRow>)) -> Value {
    json!({ "node": node, "streamers": streamers, "templates": templates })
}

fn rows_error(node: Option<i64>, error: String) -> Value {
    json!({ "node": node, "error": error })
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

/// 换上传主机之前的检查：配对里的节点在线、次版本 ≥ 5，两台之间的同步与场次连接都接着，
/// 控制面这边没有做到一半的场次（节点那边由它自己回话）。只剩一台在线时换了，另一台回来时会以为自己还是主机
fn switch_refusal(
    proto: Option<u32>,
    synced: bool,
    linked: bool,
    busy: Option<String>,
) -> Option<Refused> {
    let Some(proto) = proto else {
        return Some(Refused::Conflict(
            "配对里的节点不在线：只剩一台在线时不能换上传主机（免得两台都以为自己是主机），等两台都在线再换"
                .into(),
        ));
    };
    if proto < PAIR_SINCE {
        return Some(Refused::Conflict(format!(
            "节点的协议次版本是 {proto}，低于 {PAIR_SINCE}：它不认识上传主备对调。请先把它升级到与控制面相同的版本"
        )));
    }
    if !synced || !linked {
        return Some(Refused::Conflict(
            "两台之间的配对连接还没接上（节点刚连上时要等它落地期望状态），稍后再试".into(),
        ));
    }
    busy.map(|busy| Refused::Conflict(format!("{busy}：等它了结再换上传主机")))
}

impl Pairing {
    pub fn new(services: ServiceRegister, dir: &Path) -> Self {
        Pairing {
            services,
            dir: dir.to_path_buf(),
            active: Mutex::default(),
            retired: Mutex::default(),
            ready: watch::channel(false).0,
            busy: tokio::sync::Mutex::default(),
            watcher: Mutex::default(),
            inventories: Mutex::default(),
            next_ask: AtomicU64::new(1),
        }
    }

    fn active(&self) -> Option<Active> {
        self.active.lock().unwrap().clone()
    }

    /// 生效中配对里控制面这一侧，以及 `node` 是不是配对里的节点
    fn upload_for(&self, node: i64) -> Option<Upload> {
        let active = self.active.lock().unwrap();
        let active = active.as_ref()?;
        (active.pair.standby_node_id == node).then(|| active.upload.clone())
    }

    fn config(&self) -> Config {
        self.services.config.read().unwrap().clone()
    }

    /// 节点连上控制面（`Controller::session`）：控制面是主机时接上连接等它上报。次版本低于 4 的节点
    /// 不认识场次消息，不接。控制面是备机时等节点落地期望状态（起好主机）之后再上报（[`Self::node_acked`]）
    pub fn node_connected(
        &self,
        node: i64,
        proto: u32,
        outbox: &mpsc::UnboundedSender<ControllerMessage>,
    ) {
        let Some(upload) = self.upload_for(node) else {
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
        if let Upload::Primary(primary) = upload {
            primary.connected(Link::Controller(outbox.clone()));
        }
    }

    /// 节点离线（它的连接从在线表里移除之后）
    pub fn node_offline(&self, node: i64) {
        if let Some(upload) = self.upload_for(node) {
            upload.link_down();
        }
        if let Some(member) = self.member_for(node) {
            member.link_down();
        }
    }

    /// 生效中配对的同步端，以及 `node` 是不是配对节点
    fn member_for(&self, node: i64) -> Option<Arc<Member>> {
        let active = self.active.lock().unwrap();
        let active = active.as_ref()?;
        (active.pair.standby_node_id == node).then(|| active.member.clone())
    }

    /// 配对节点次版本 ≥ 5 时与控制面双向同步：给它的期望状态不带 F3 的配置，带 `pair`
    pub fn syncs(&self, node: i64, proto: u32) -> bool {
        proto >= PAIR_SINCE && self.member_for(node).is_some()
    }

    /// 节点应答了期望状态（`Controller::ack`）。配对节点这时已经按带 `pair` 的期望状态建好了同步端，
    /// 控制面这边接上连接、发出排着的修改；控制面当备机时再上报手里的场次（节点这时已经起好了主机）
    pub async fn node_acked(&self, controller: &Controller, node: i64) {
        let Some(active) = self.active().filter(|a| a.pair.standby_node_id == node) else {
            return;
        };
        let Some((proto, outbox)) = controller.node_link(node) else {
            return;
        };
        if proto < PAIR_SINCE {
            if active.pair.leader() == Side::Node {
                self.fall_back(controller).await;
            }
            return;
        }
        active
            .member
            .link_up(Link::Controller(outbox.clone()))
            .await;
        if let Upload::Standby(standby) = &active.upload
            && !standby.linked()
        {
            let link = Link::Controller(outbox);
            let report = standby.link_up(link.clone());
            link.ha(report);
        }
    }

    /// 配对节点降到了次版本 5 以下（不认识 `leader`，只会当备机）而上传主机还在它那边：退回控制面，
    /// 免得两台都当备机、各自按主机离线处理
    async fn fall_back(&self, controller: &Controller) {
        let Ok(_busy) = self.busy.try_lock() else {
            return;
        };
        let Some(active) = self.active().filter(|a| a.pair.leader() == Side::Node) else {
            return;
        };
        warn!(
            "HA：配对里的节点降到了协议次版本 {PAIR_SINCE} 以下，不认识上传主备对调，上传主机退回控制面"
        );
        if let Err(e) = self.commit(controller, &active, Side::Controller).await {
            error!(error = ?e, "HA：上传主机没能退回控制面");
        }
    }

    /// 配对节点发来的同步消息；不是配对节点的丢掉。节点改的房间与模板由控制面仲裁、改 Fleet，
    /// 之后给两台重发期望状态。节点请求换上传主机、改模式与参数时照办或说明原因
    pub async fn pair_message(&self, controller: &Controller, node: i64, message: PairMessage) {
        // 本地行清单：指定备机之前也问，那时它还不是配对节点
        if let PairMessage::Inventory(inventory) = message {
            let waiting = self
                .inventories
                .lock()
                .unwrap()
                .remove(&(node, inventory.id));
            if let Some(waiting) = waiting {
                let _ = waiting.send(inventory);
            }
            return;
        }
        let Some(active) = self.active().filter(|a| a.pair.standby_node_id == node) else {
            debug!(
                node,
                op = message.op(),
                "pair frame from a node that is not paired"
            );
            return;
        };
        // 重连后节点先重发离线期间排下的修改、再应答期望状态（`node_acked` 才接上连接）；它发来配对帧说明
        // 它的同步端已经起好了，这时就接上，免得对这些修改的应答无处可发、节点的队列一直不出队
        if let Some((proto, outbox)) = controller.node_link(node)
            && proto >= PAIR_SINCE
        {
            active.member.link_up(Link::Controller(outbox)).await;
        }
        if let PairMessage::Ha(change) = message {
            let result = self.requested(controller, change).await;
            if let Err(reason) = &result {
                info!(reason, "HA：节点请求的配对修改没有照办");
            }
            active.member.answer(change.id, result.err());
            return;
        }
        let rows = matches!(&message, PairMessage::Edit(edit)
            if edit.key.starts_with(ROOM) || edit.key.starts_with(TEMPLATE));
        let set = if rows {
            match self.pair_set(controller, &active).await {
                Ok(set) => set,
                Err(e) => {
                    warn!(error = ?e, "配对同步：读不了配对里的房间，这条修改等节点重发");
                    return;
                }
            }
        } else {
            PairSet::default()
        };
        let arbiter = Arbiter {
            controller,
            node: active.pair.primary_node_id,
            set,
        };
        if active.member.receive(message, Some(&arbiter)).await {
            controller
                .push_many([Some(active.pair.primary_node_id)])
                .await;
        }
    }

    /// 节点上请求的配对修改（它那边已经确认自己没有做到一半的场次）。控制面正在改配对设置时不等，直接请它稍后再试
    async fn requested(&self, controller: &Controller, change: HaChange) -> Result<(), String> {
        let Ok(_busy) = self.busy.try_lock() else {
            return Err("控制面正在改配对设置，稍后再试".into());
        };
        let failed = |e: error_stack::Report<AppError>| {
            warn!(error = ?e, "HA：没能保存节点请求的配对修改");
            "控制面没能保存配对设置，看控制面的日志".to_string()
        };
        if let Some(value) = change.ha {
            let Some(active) = self.active() else {
                return Err("没有生效中的配对".into());
            };
            let standby = active.pair.standby_node_id;
            self.designate_locked(controller, standby, value.mode, Some(value.params))
                .await
                .map_err(failed)?
                .map_err(Refused::message)?;
        }
        if let Some(primary) = change.primary {
            self.switch_locked(controller, primary, false)
                .await
                .map_err(failed)?
                .map_err(Refused::message)?;
        }
        Ok(())
    }

    /// 配对里的房间（主机「本机」持有、不是边录边传）与它们用的模板，以及单独加入配对的模板
    async fn pair_set(&self, controller: &Controller, active: &Active) -> AppResult<PairSet> {
        let (rooms, templates) =
            assignments::desired_state(controller.pool(), active.pair.primary_node_id).await?;
        let (rooms, _) = split(&self.config(), rooms);
        let used: BTreeSet<i64> = rooms.iter().filter_map(|room| room.template_id).collect();
        let mut templates: Vec<DesiredTemplate> = templates
            .into_iter()
            .filter(|template| used.contains(&template.id))
            .collect();
        add_templates(
            &mut templates,
            pinned_templates(controller, &active.member).await?,
        );
        Ok(PairSet { rooms, templates })
    }

    /// 控制面要给主机「本机」下发之前（`Controller::push_many`）：认出配对里房间与模板在控制面上的改动
    pub async fn rescan(&self, controller: &Controller) {
        let Some(active) = self.active() else {
            return;
        };
        match self.pair_set(controller, &active).await {
            Ok(set) => active.member.scan_fleet(&set).await,
            Err(e) => warn!(error = ?e, "配对同步：读不了配对里的房间，这次不认控制面上的改动"),
        }
    }

    /// 节点发来的场次消息；不是配对节点的丢掉
    pub fn node_message(&self, node: i64, message: HaMessage) {
        match self.upload_for(node) {
            Some(Upload::Primary(primary)) => primary.standby_message(message),
            Some(Upload::Standby(standby)) => standby.primary_message(message),
            None => debug!(
                node,
                kind = message.kind(),
                "HA frame from a node that is not paired"
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

    /// 按配对起控制面这一侧：上传主机是「本机」时起主机，是节点时起备机（`ha-state.json` 里上次当备机时
    /// 留下的场次属于同一台节点才接着用）。设好钩子的角色
    async fn start_upload(
        &self,
        controller: &Controller,
        pair: &Pair,
        kept: &[DesiredRoom],
    ) -> AppResult<Upload> {
        let upload = match pair.leader() {
            Side::Controller => {
                let primary = Primary::start(
                    controller.pool().clone(),
                    self.services.clone(),
                    pair.mode,
                    pair.params,
                    self.window(),
                )
                .await?;
                primary.set_rooms(room_map(kept));
                set_role(Some(Role::Primary(primary.clone())));
                Upload::Primary(primary)
            }
            Side::Node => {
                let path = self.dir.join(STATE_FILE_NAME);
                let owner = peer(pair.standby_node_id);
                let previous = agent::load(&path).filter(|state| state.controller == owner);
                let assignment = HaAssignment::of(pair, kept.iter().map(|room| room.id).collect());
                let standby =
                    Standby::start(path, &owner, assignment, previous, self.services.clone());
                standby.set_rooms(standby_rooms(kept));
                set_role(Some(Role::Standby(standby.clone())));
                Upload::Standby(standby)
            }
        };
        self.retired.lock().unwrap().take();
        Ok(upload)
    }

    async fn activate(&self, controller: &Controller, pair: &Pair) -> AppResult<()> {
        let (rooms, _) =
            assignments::desired_state(controller.pool(), pair.primary_node_id).await?;
        let (kept, excluded) = split(&self.config(), rooms);
        for room in &excluded {
            warn!(room = room.id, url = room.url, "HA：{}", room.reason);
        }
        let upload = self.start_upload(controller, pair, &kept).await?;
        let previous = self.active.lock().unwrap().take();
        if let Some(previous) = &previous {
            previous.upload.stop();
            if previous.pair.standby_node_id == pair.standby_node_id {
                previous.member.stop();
            } else {
                previous.member.dissolve();
            }
        }
        let member = Member::start(
            Side::Controller,
            &self.dir,
            &peer(pair.standby_node_id),
            self.services.clone(),
            pair.leader(),
        )
        .await;
        *self.active.lock().unwrap() = Some(Active {
            pair: pair.clone(),
            upload,
            member,
        });
        self.rescan(controller).await;
        Ok(())
    }

    /// 停下配对；`dissolved` 为真（解除配对、节点被移除）时同步账本也删掉
    fn deactivate(&self, dissolved: bool) {
        let Some(active) = self.active.lock().unwrap().take() else {
            return;
        };
        match &active.upload {
            Upload::Standby(standby) if dissolved => {
                standby.retire();
                *self.retired.lock().unwrap() = Some(standby.clone());
            }
            upload => {
                upload.stop();
                set_role(None);
            }
        }
        if dissolved {
            active.member.dissolve();
            info!(standby = active.pair.standby_node_id, "HA：配对已解除");
        } else {
            active.member.stop();
        }
    }

    /// 给节点下发期望状态时（`Controller::push_locked`）。主机「本机」：刷新钩子认的房间；
    /// 次版本 ≥ 4 的备机：加上镜像的房间与模板，返回要带的 [`HaAssignment`]，次版本 ≥ 5 时再带上
    /// 同步版本（[`PairState`]）；其余节点原样
    pub async fn desired(
        &self,
        controller: &Controller,
        node: i64,
        proto: u32,
        rooms: &mut Vec<DesiredRoom>,
        templates: &mut Vec<DesiredTemplate>,
    ) -> AppResult<(Option<HaAssignment>, Option<PairState>)> {
        self.wait_ready().await;
        let Some(active) = self.active() else {
            return Ok((None, None));
        };
        if node == active.pair.primary_node_id {
            let config = self.config();
            let kept: Vec<DesiredRoom> = rooms
                .iter()
                .filter(|room| !sync_downloader(&config, room.spec.override_cfg.clone()))
                .cloned()
                .collect();
            active.upload.set_rooms(&kept);
            add_templates(
                templates,
                pinned_templates(controller, &active.member).await?,
            );
            return Ok((None, None));
        }
        if node != active.pair.standby_node_id || proto < HA_SINCE {
            return Ok((None, None));
        }
        let (primary_rooms, primary_templates) =
            assignments::desired_state(controller.pool(), active.pair.primary_node_id).await?;
        let (mut kept, excluded) = split(&self.config(), primary_rooms);
        if !excluded.is_empty() {
            debug!(
                rooms = ?excluded.iter().map(|room| room.id).collect::<Vec<_>>(),
                "HA：边录边传的房间不镜像给备机"
            );
        }
        let mut pinned = Vec::new();
        let pair = if proto >= PAIR_SINCE {
            pinned = pinned_templates(controller, &active.member).await?;
            let mut set_templates = primary_templates.clone();
            add_templates(&mut set_templates, pinned.clone());
            let set = PairSet {
                rooms: kept.clone(),
                templates: set_templates,
            };
            let mut state = active.member.pair_state(&set).await;
            state.adopt = active.member.adopt_request().await.map(Box::new);
            // 还没记进账本的房间（节点新建、控制面正在收下）这次先不镜像，免得备机按控制面 id 再建一行
            kept.retain(|room| state.room(room.id).is_some());
            pinned.retain(|template| state.template(template.id).is_some());
            Some(state)
        } else {
            None
        };
        let assignment = mirror(&active.pair, kept, primary_templates, rooms, templates);
        add_templates(templates, pinned);
        Ok((Some(assignment), pair))
    }

    /// 要下发的节点里有配对里的一台
    pub fn involves(&self, nodes: &[i64]) -> bool {
        let active = self.active.lock().unwrap();
        active.as_ref().is_some_and(|active| {
            nodes.contains(&active.pair.primary_node_id)
                || nodes.contains(&active.pair.standby_node_id)
        })
    }

    /// 要给主机「本机」重发期望状态时，备机的镜像也跟着重发
    pub fn mirror_target(&self, nodes: &[i64]) -> Option<i64> {
        let active = self.active.lock().unwrap();
        let pair = &active.as_ref()?.pair;
        nodes
            .contains(&pair.primary_node_id)
            .then_some(pair.standby_node_id)
    }

    /// 指定备机、改模式或参数。备机已经连着时先接上主机，再给它下发带配对的期望状态。
    /// 指定一台新的备机时，它上面已有的本地主播与模板按 `adopt` 纳入配对（不填就是全部），
    /// 返回的第二项是逐条的清单（[`Self::adopt_standby`]）；备机不变、也没填 `adopt` 时为空
    pub async fn designate(
        &self,
        controller: &Controller,
        request: Designate,
    ) -> AppResult<Result<(Pair, Option<Value>), Refused>> {
        self.wait_ready().await;
        let _busy = self.busy.lock().await;
        let fresh = store::pair(controller.pool())
            .await?
            .is_none_or(|pair| pair.standby_node_id != request.standby);
        let pair = match self
            .designate_locked(controller, request.standby, request.mode, request.params)
            .await?
        {
            Ok(pair) => pair,
            Err(refused) => return Ok(Err(refused)),
        };
        let selection = match request.adopt {
            Some(selection) => selection,
            None if fresh => Selection::default(),
            None => return Ok(Ok((pair, None))),
        };
        let Some(active) = self.active() else {
            return Ok(Ok((pair, None)));
        };
        let adoption = self
            .adopt_standby(
                controller,
                &active,
                selection.streamers.as_deref(),
                selection.templates.as_deref(),
            )
            .await?;
        Ok(Ok((pair, Some(adoption))))
    }

    /// 问节点还没纳入配对的本地主播与模板（节点不在配对里也回）
    async fn inventory(&self, controller: &Controller, node: i64) -> Result<Inventory, String> {
        let Some((proto, outbox)) = controller.node_link(node) else {
            return Err("备机不在线，读不到它上面的本地主播与模板".into());
        };
        if proto < PAIR_SINCE {
            return Err(format!(
                "备机的协议次版本是 {proto}，低于 {PAIR_SINCE}：它不认识配对同步，上面的本地主播与模板不能纳入配对（仍是它的本地行）"
            ));
        }
        let id = self.next_ask.fetch_add(1, Ordering::Relaxed);
        let (answer, answered) = oneshot::channel();
        self.inventories.lock().unwrap().insert((node, id), answer);
        let ask = ControllerMessage::Pair(PairMessage::InventoryAsk(InventoryAsk { id }));
        let result = match outbox.send(ask) {
            Ok(()) => tokio::time::timeout(INVENTORY_WAIT, answered)
                .await
                .ok()
                .map(|answer| answer.map_err(drop)),
            Err(_) => Some(Err(())),
        };
        self.inventories.lock().unwrap().remove(&(node, id));
        match result {
            Some(Ok(inventory)) => Ok(inventory),
            Some(Err(_)) => Err("备机的连接断了，读不到它上面的本地主播与模板".into()),
            None => Err(format!(
                "备机 {} 秒内没有回话，读不到它上面的本地主播与模板",
                INVENTORY_WAIT.as_secs()
            )),
        }
    }

    /// 主机「本机」上还没纳入配对的本地行
    async fn local_rows(
        &self,
        controller: &Controller,
        active: Option<&Active>,
    ) -> (Vec<LocalRow>, Vec<LocalRow>) {
        let fleet = controller
            .local()
            .map(|local| local.fleet_state())
            .unwrap_or_default();
        match active {
            Some(active) => active.member.local_rows(&fleet).await,
            None => adopt::local_rows(&self.services, &fleet, None).await,
        }
    }

    /// 请备机把它上面的本地行加入配对：问清单、按控制面的规矩逐条判断（[`adopt::evaluate`]），
    /// 挑出这一次纳入的，随期望状态带过去。不纳入的逐条带原因、留作备机的本地行
    async fn adopt_standby(
        &self,
        controller: &Controller,
        active: &Active,
        streamers: Option<&[i64]>,
        templates: Option<&[i64]>,
    ) -> AppResult<Value> {
        let standby = active.pair.standby_node_id;
        let inventory = match self.inventory(controller, standby).await {
            Ok(inventory) => inventory,
            Err(reason) => {
                warn!(standby, reason, "配对：备机上已有的本地行这次没能纳入配对");
                return Ok(rows_error(Some(standby), reason));
            }
        };
        let (primary_rooms, _) = self.local_rows(controller, Some(active)).await;
        let (mut rooms, mut template_rows) = (inventory.rooms, inventory.templates);
        adopt::evaluate(
            &mut rooms,
            &self.config(),
            &fleet_urls(controller).await?,
            &adopt::urls(&primary_rooms),
        );
        let room_ids = adopt::select(&mut rooms, streamers);
        let template_ids = adopt::select_templates(&mut template_rows, templates, &rooms);
        if !room_ids.is_empty() || !template_ids.is_empty() {
            info!(
                standby,
                rooms = ?room_ids,
                templates = ?template_ids,
                "配对：请备机把这些本地行加入配对"
            );
            active.member.request(&room_ids, &template_ids).await;
            controller.push_many([Some(standby)]).await;
        }
        Ok(rows_view(Some(standby), (rooms, template_rows)))
    }

    /// `GET /v1/fleet/ha/candidates`：备机上还没纳入配对的本地主播与模板，逐条标出能不能加入、为什么，
    /// `included` 是缺省纳入的（能加入的都纳入）。`standby` 不填时是配对里的节点；
    /// 指定备机之前填上候选节点，确认弹层按它列出缺省纳入的清单
    pub async fn candidates(
        &self,
        controller: &Controller,
        standby: Option<i64>,
    ) -> AppResult<Result<Value, Refused>> {
        self.wait_ready().await;
        let active = self.active();
        let local = controller.local_node_id();
        let standby = standby.or(active.as_ref().map(|a| a.pair.standby_node_id));
        if standby.is_some() && standby == local {
            return Ok(Err(Refused::Invalid(
                "备机不能是「本机」节点：选一台普通节点".into(),
            )));
        }
        let standby_view = match standby {
            Some(node) => match self.inventory(controller, node).await {
                Ok(inventory) => {
                    let (primary_rooms, _) = self.local_rows(controller, active.as_ref()).await;
                    let (mut rooms, mut templates) = (inventory.rooms, inventory.templates);
                    adopt::evaluate(
                        &mut rooms,
                        &self.config(),
                        &fleet_urls(controller).await?,
                        &adopt::urls(&primary_rooms),
                    );
                    adopt::select(&mut rooms, None);
                    adopt::select_templates(&mut templates, None, &rooms);
                    rows_view(standby, (rooms, templates))
                }
                Err(reason) => rows_error(standby, reason),
            },
            None => Value::Null,
        };
        Ok(Ok(json!({
            "paired": active.is_some(),
            "standby": standby_view,
        })))
    }

    async fn designate_locked(
        &self,
        controller: &Controller,
        standby: i64,
        mode: HaMode,
        params: Option<HaParams>,
    ) -> AppResult<Result<Pair, Refused>> {
        let pool = controller.pool();
        let previous = store::pair(pool).await?;
        let params = params
            .or(previous.as_ref().map(|pair| pair.params))
            .unwrap_or_default();
        if let Err(e) = params.validate() {
            return Ok(Err(Refused::Invalid(e)));
        }
        let local = controller.local_node_id();
        let node = crate::server::fleet::store::node(pool, standby).await?;
        let same_standby = previous
            .as_ref()
            .is_some_and(|pair| pair.standby_node_id == standby);
        let link = controller.node_link(standby);
        if let Some(refused) = refusal(
            local,
            standby,
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
        let pair = store::save_pair(pool, local, standby, mode, &params, now_ms()).await?;
        let current = self.active();
        let replaced = current
            .as_ref()
            .map(|active| active.pair.standby_node_id)
            .filter(|standby| *standby != pair.standby_node_id);
        match current {
            Some(active)
                if replaced.is_none() && active.pair.primary_node_id == pair.primary_node_id =>
            {
                active.upload.configure(pair.mode, pair.params);
                *self.active.lock().unwrap() = Some(Active {
                    pair: pair.clone(),
                    upload: active.upload,
                    member: active.member,
                });
            }
            _ => {
                if let Err(e) = self.activate(controller, &pair).await {
                    let _ = store::clear_pair(pool).await;
                    self.deactivate(true);
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

    /// 换上传主机（`POST /v1/fleet/ha/role`）：两台都在线、各自没有做到一半的场次时才换。
    /// 先问节点（它回话前查自己的场次），再由控制面落库、换好自己这一侧、给节点下发带 `leader` 的配对。
    /// 本来就是这一台时什么都不做
    pub async fn switch(
        &self,
        controller: &Controller,
        primary: Side,
    ) -> AppResult<Result<Pair, Refused>> {
        self.wait_ready().await;
        let _busy = self.busy.lock().await;
        self.switch_locked(controller, primary, true).await
    }

    /// `ask` 为假时是节点自己请求的，已经查过它那边
    async fn switch_locked(
        &self,
        controller: &Controller,
        primary: Side,
        ask: bool,
    ) -> AppResult<Result<Pair, Refused>> {
        let Some(active) = self.active() else {
            return Ok(Err(Refused::Conflict("没有生效中的配对".into())));
        };
        if active.pair.leader() == primary {
            return Ok(Ok(active.pair));
        }
        let proto = controller
            .node_link(active.pair.standby_node_id)
            .map(|(proto, _)| proto);
        if let Some(refused) = switch_refusal(
            proto,
            active.member.linked(),
            active.upload.linked(),
            active.upload.busy(),
        ) {
            return Ok(Err(refused));
        }
        if ask && let Err(reason) = active.member.ask(Some(primary), None).await {
            return Ok(Err(Refused::Conflict(reason)));
        }
        Ok(Ok(self.commit(controller, &active, primary).await?))
    }

    /// 落库并换好控制面这一侧，再给节点下发（节点按 `leader` 换它那一侧，落地后控制面当备机时上报）
    async fn commit(
        &self,
        controller: &Controller,
        active: &Active,
        primary: Side,
    ) -> AppResult<Pair> {
        let pool = controller.pool();
        let pair = store::set_leader(pool, primary, now_ms())
            .await?
            .ok_or_else(|| error_stack::Report::new(AppError::Custom("ha_pair 不见了".into())))?;
        let (rooms, _) = assignments::desired_state(pool, pair.primary_node_id).await?;
        let (kept, _) = split(&self.config(), rooms);
        let upload = self.start_upload(controller, &pair, &kept).await?;
        active.upload.stop();
        active.member.set_primary(primary);
        if let (Upload::Primary(primary), Some((_, outbox))) =
            (&upload, controller.node_link(pair.standby_node_id))
        {
            primary.connected(Link::Controller(outbox));
        }
        *self.active.lock().unwrap() = Some(Active {
            pair: pair.clone(),
            upload,
            member: active.member.clone(),
        });
        info!(
            leader = pair.leader().as_str(),
            node = pair.leader_id(),
            "HA：上传主机换好了"
        );
        controller.push_many([Some(pair.standby_node_id)]).await;
        Ok(pair)
    }

    /// 解除配对：主机停下，给备机下发不带配对、不带镜像房间的期望状态。本来就没有配对时返回 `false`
    pub async fn dissolve(&self, controller: &Controller) -> AppResult<bool> {
        self.wait_ready().await;
        let _busy = self.busy.lock().await;
        let pair = store::pair(controller.pool()).await?;
        store::clear_pair(controller.pool()).await?;
        self.deactivate(true);
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
                self.deactivate(true);
                warn!(node, "HA：配对里的节点被移除，配对已解除");
            }
            Ok(_) => {}
            Err(e) => warn!(error = ?e, "HA：读不了配对设置"),
        }
    }

    /// 控制面面板上点的人工处理：控制面是主机时转给备机执行，是备机时就在本机执行
    pub fn manual(&self, key: &str, action: ManualAction) -> Result<(), Refused> {
        let Some(active) = self.active() else {
            return Err(Refused::Conflict("没有生效中的配对".into()));
        };
        match &active.upload {
            Upload::Primary(primary) => {
                let message = HaMessage::Manual {
                    key: key.to_string(),
                    action,
                };
                if primary.forward(message) {
                    info!(key, ?action, "HA：人工处理已转给备机");
                    Ok(())
                } else {
                    Err(Refused::Conflict(
                        "备机不在线：到备机本地的节点页上处理这一场".into(),
                    ))
                }
            }
            Upload::Standby(standby) => standby.manual(key, action).map_err(Refused::Conflict),
        }
    }

    /// `GET /v1/fleet/ha`。`standby` 是配对里那台节点（机器身份，与此刻谁上传无关），`leader` 是此刻的上传主机；
    /// 控制面当备机时 `local_standby` 是它手里的场次
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
                "linked": active.as_ref().is_some_and(|active| active.upload.linked()),
                "reported": active.as_ref().is_some_and(|active| active.upload.reported()),
            })
        });
        let sync = match &active {
            Some(active) => json!({
                "min_proto": PAIR_SINCE,
                "linked": active.member.linked(),
                "pending": active.member.pending().await,
            }),
            None => json!({ "min_proto": PAIR_SINCE }),
        };
        let excluded = match pair.as_ref().map(|pair| pair.primary_node_id).or(local) {
            Some(node) => {
                let (rooms, _) = assignments::desired_state(pool, node).await?;
                split(&self.config(), rooms).1
            }
            None => Vec::new(),
        };
        let local_standby = match active.as_ref().map(|active| &active.upload) {
            Some(Upload::Standby(standby)) => standby.view(),
            _ => Value::Null,
        };
        let sessions = store::recent_sessions(pool, RECENT_SESSIONS).await?;
        Ok(json!({
            "pair": pair,
            "active": active.is_some(),
            "local_node": local,
            "leader": pair.as_ref().map(Pair::leader),
            "local_role": active.as_ref().map(|active| active.upload.side()),
            "busy": active.as_ref().and_then(|active| active.upload.busy()),
            "standby": standby,
            "min_proto": HA_SINCE,
            "sync": sync,
            "excluded": excluded,
            "sessions": sessions,
            "local_standby": local_standby,
        }))
    }

    pub fn shutdown(&self) {
        if let Some(task) = self.watcher.lock().unwrap().take() {
            task.abort();
        }
        self.deactivate(false);
        if let Some(retired) = self.retired.lock().unwrap().take()
            && matches!(super::role(), Some(Role::Standby(current)) if Arc::ptr_eq(&current, &retired))
        {
            set_role(None);
        }
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

    /// 换上传主机只在两台都在线、连接接好、控制面没有做到一半的场次时才行，每种都说清原因
    #[test]
    fn the_upload_primary_switches_only_while_both_sides_are_online_and_idle() {
        assert!(matches!(
            switch_refusal(None, true, true, None),
            Some(Refused::Conflict(m)) if m.contains("只剩一台在线") && m.contains("两台都在线")
        ));
        assert!(matches!(
            switch_refusal(Some(4), true, true, None),
            Some(Refused::Conflict(m)) if m.contains("协议次版本是 4") && m.contains("低于 5")
        ));
        assert!(matches!(
            switch_refusal(Some(5), false, true, None),
            Some(Refused::Conflict(m)) if m.contains("还没接上")
        ));
        assert!(matches!(
            switch_refusal(Some(5), true, false, None),
            Some(Refused::Conflict(m)) if m.contains("还没接上")
        ));
        assert!(matches!(
            switch_refusal(Some(5), true, true, Some("主机上房间 7 的一场还没了结（recording）".into())),
            Some(Refused::Conflict(m)) if m.contains("房间 7") && m.contains("等它了结")
        ));
        assert_eq!(switch_refusal(Some(5), true, true, None), None);
    }

    #[test]
    fn a_mirrored_assignment_names_the_leader_only_when_it_is_the_node() {
        let mut pair = pair();
        let at_controller = serde_json::to_value(HaAssignment::of(&pair, vec![7])).unwrap();
        assert!(at_controller.get("leader").is_none());
        assert_eq!(at_controller["primary"], 1);
        pair.leader_node_id = Some(pair.standby_node_id);
        let at_node = HaAssignment::of(&pair, vec![7]);
        assert_eq!(at_node.leader, Side::Node);
        assert_eq!(at_node.primary, 5, "`primary` 是此刻上传主机的节点 id");
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
