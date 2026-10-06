//! 解除配对时把从备机纳入配对的行交还给备机（ha-pair 方案 H2，做法 B：恢复配对之前的归属）。
//!
//! 哪些行交还：节点第一次配对时记下它当时的本地主播与模板（[`super::adopt::Before`]）；这些行经纳入配对
//! （指定备机时的纳入，或之后的「加入配对」）进来时，节点给账本键标上交还，发给控制面时带着
//! （[`super::sync::PairEdit::returns`]），控制面记在同步账本里（`Adoption::returns`）。配对期间在任一台上
//! 新建的行、本来就是主机的行没有这个标记：解除配对后留在主机（控制面的 Fleet 房间与模板），备机上撤掉。
//!
//! 怎么交还才不出双录、双稿：
//! 1. 解除配对时控制面把要交还的房间与模板记进 `data/pair-handback.json`（[`Ledger`]），配对照常解除，
//!    录到一半的一场按 H1 的规矩收尾：上传主机投它那一份，另一台留着自己录的（不投）。
//! 2. 交还中（[`Stage::Hold`]）：给那台节点的期望状态仍带这些房间（设置跟着主机上的修改），并在
//!    [`Handback::hold`] 里列出：节点按原来那一行落地（不重建监控），但挡着开录（`data/pair-holds.json`，
//!    重启后接着挡，[`NodeHolds`]）。这期间只有主机「本机」录这些房间、照常投稿。
//! 3. 节点应答了带着挡开录的期望状态（[`Stage::Held`]）、且在线时，等主机「本机」上这个房间空闲
//!    （没在录、没在投）：先挡住「本机」开录，隔 [`SETTLE_MS`] 仍然空闲就删掉 Fleet 房间（「本机」按 F2 的
//!    释放流程撤掉那一行），再在 [`Handback::rooms`] 里告诉节点放开开录、把那一行转成本地行
//!    （[`Stage::Released`]）。「本机」上的挡一直留到控制面确认它不再持有这个房间。任何时刻只有一台会开录。
//! 4. 模板在用它的交还房间都交完之后交还：删掉 Fleet 模板（配对期间主机上的房间也在用它时留着），
//!    节点上转成本地模板。
//!
//! 节点离线（解除配对时不在线，或之后断开）时停在第 3 步之前：房间留在主机「本机」照常录，等它回来应答了
//! 再往下走（「待归还」）。节点被移除时不再交还（留在主机，即做法 A）。再次把同一台指定为备机时，
//! 还没交的房间回到配对里，仍记成交还给它的。
//!
//! 面板上的两个手动动作（[`Handbacks::act`]，H3）：
//! - 放弃交还：还没交完的房间或模板从账本里去掉、留在主机（这一行按做法 A）。节点那一行在它下次落地时撤掉，
//!   撤掉之后才放开它挡的开录（[`NodeHolds::settle`]），所以任何时刻仍只有一台会开录。
//! - 立即交还：节点挡住了开录、主机「本机」只在投（没在录）的房间不等投完就交接：按第 3 步挡住「本机」开录、
//!   隔 [`SETTLE_MS`] 仍没在录就删掉 Fleet 房间。F2 的释放只撤掉监控与那一行，在投的一场在上传池里接着投完
//!   （它拿着开录时的上下文，不再读那一行），不中断、不重传。

use super::adopt::{self, HoldBy, SETTLE_MS};
use super::member::{Member, identity};
use super::pairing::Refused;
use super::sync::{ROOM, TEMPLATE, room_key, template_key};
use crate::server::errors::{AppError, AppResult};
use crate::server::fleet::assignments::{self, DeleteTemplate};
use crate::server::fleet::controller::{Controller, DispatchError};
use crate::server::fleet::model::{DesiredRoom, DesiredTemplate};
use crate::server::fleet::protocol::{DesiredState, PAIR_SINCE};
use crate::server::fleet::reconcile::FleetState;
use crate::server::infrastructure::connection_pool::ConnectionPool;
use crate::server::infrastructure::service_register::ServiceRegister;
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::{info, warn};

/// 控制面：还没交完的
pub const FILE_NAME: &str = "pair-handback.json";
/// 节点：交还中挡着开录的房间
pub const HOLDS_FILE_NAME: &str = "pair-holds.json";

/// 期望状态里给交还中的那台节点的（自次版本 5 起，只给它）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handback {
    /// 还在交还的房间（在期望状态里）：按原来那一行落地，挡着开录
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hold: Vec<i64>,
    /// 还在交还的模板（在期望状态里）：没有房间用也留着
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hold_templates: Vec<i64>,
    /// 交完的房间：放开开录，转成本机的本地行（库里的行与监控不动）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rooms: Vec<i64>,
    /// 交完的模板：转成本机的本地模板
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub templates: Vec<i64>,
}

/// 交还走到哪一步
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// 等节点应答挡开录（节点离线时停在这里，房间照常由主机「本机」录）
    #[default]
    Hold,
    /// 节点挡住了开录，等主机「本机」上这个房间空闲
    Held,
    /// 控制面上删掉了，等节点转成本地行、「本机」撤掉那一行
    Released,
}

/// 一个要交还的房间或模板
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// 房间的直播间地址（模板为空）
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    /// 房间用的 Fleet 模板（交完之前以控制面此刻的为准）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<i64>,
    #[serde(default)]
    pub stage: Stage,
    /// 主机「本机」空闲、开始挡开录的时刻
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<i64>,
    /// 点了「立即交还」：「本机」只要没在录就交接，不等在投的投完
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub force: bool,
    /// 第一个带着这一步的期望状态版本；不落盘，控制面重启后重发
    #[serde(skip)]
    sent: Option<u64>,
    /// 节点应答了这一步
    #[serde(skip)]
    acked: bool,
}

impl Entry {
    fn room(url: &str, template: Option<i64>) -> Self {
        Entry {
            url: url.to_string(),
            template,
            ..Entry::default()
        }
    }

    fn advance(&mut self, stage: Stage) {
        self.stage = stage;
        self.since = None;
        self.force = false;
        self.sent = None;
        self.acked = false;
    }

    /// `version` 的应答确认了这一步
    fn confirmed_by(&self, version: u64) -> bool {
        self.sent.is_some_and(|sent| sent <= version)
    }
}

/// 交还给一台节点的
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Returning {
    /// 解除配对时的主机「本机」节点
    pub primary: i64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub rooms: BTreeMap<i64, Entry>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub templates: BTreeMap<i64, Entry>,
}

impl Returning {
    fn is_empty(&self) -> bool {
        self.rooms.is_empty() && self.templates.is_empty()
    }

    /// 还有挡着开录的房间（主机上改了房间要重发给这台）
    fn holding(&self) -> bool {
        self.rooms
            .values()
            .any(|entry| entry.stage != Stage::Released)
    }
}

/// `data/pair-handback.json`：节点 id → 交还给它的
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    #[serde(default)]
    pub nodes: BTreeMap<i64, Returning>,
}

impl Ledger {
    /// 给 `node` 的期望状态里带的
    pub fn handback(&self, node: i64) -> Option<Handback> {
        let returning = self.nodes.get(&node)?;
        let mut handback = Handback::default();
        for (id, entry) in &returning.rooms {
            match entry.stage {
                Stage::Hold | Stage::Held => handback.hold.push(*id),
                Stage::Released => handback.rooms.push(*id),
            }
        }
        for (id, entry) in &returning.templates {
            match entry.stage {
                Stage::Hold | Stage::Held => handback.hold_templates.push(*id),
                Stage::Released => handback.templates.push(*id),
            }
        }
        Some(handback)
    }

    /// 发出了带着 `handback` 的期望状态 `version`：这一步在等它的应答
    pub fn sent(&mut self, node: i64, handback: &Handback, version: u64) {
        let Some(returning) = self.nodes.get_mut(&node) else {
            return;
        };
        for (id, entry) in &mut returning.rooms {
            let carried = match entry.stage {
                Stage::Hold => handback.hold.contains(id),
                Stage::Held => false,
                Stage::Released => handback.rooms.contains(id),
            };
            if carried && entry.sent.is_none() {
                entry.sent = Some(version);
            }
        }
        for (id, entry) in &mut returning.templates {
            if entry.stage == Stage::Released
                && handback.templates.contains(id)
                && entry.sent.is_none()
            {
                entry.sent = Some(version);
            }
        }
    }

    /// 节点应答了期望状态 `version`（在它之前的都已落地）。返回要不要落盘
    pub fn acked(&mut self, node: i64, version: u64) -> bool {
        let Some(returning) = self.nodes.get_mut(&node) else {
            return false;
        };
        let mut changed = false;
        for (id, entry) in &mut returning.rooms {
            if !entry.confirmed_by(version) {
                continue;
            }
            match entry.stage {
                Stage::Hold => {
                    info!(node, room = id, "配对交还：备机挡住了这个房间的开录");
                    entry.advance(Stage::Held);
                    changed = true;
                }
                Stage::Released => entry.acked = true,
                Stage::Held => {}
            }
        }
        for entry in returning.templates.values_mut() {
            if entry.stage == Stage::Released && entry.confirmed_by(version) {
                entry.acked = true;
            }
        }
        changed
    }

    /// 同一台又被指定为备机：还没交的房间与模板不交了（它们回到配对里），交完的照常收尾
    pub fn cancel(&mut self, node: i64) -> (Vec<i64>, Vec<i64>) {
        let Some(returning) = self.nodes.get_mut(&node) else {
            return (Vec::new(), Vec::new());
        };
        let pending = |entries: &BTreeMap<i64, Entry>| -> Vec<i64> {
            entries
                .iter()
                .filter(|(_, entry)| entry.stage != Stage::Released)
                .map(|(id, _)| *id)
                .collect()
        };
        let rooms = pending(&returning.rooms);
        let templates = pending(&returning.templates);
        returning.rooms.retain(|id, _| !rooms.contains(id));
        returning.templates.retain(|id, _| !templates.contains(id));
        if returning.is_empty() {
            self.nodes.remove(&node);
        }
        (rooms, templates)
    }
}

/// 控制面上的交还账本
pub struct Handbacks {
    path: PathBuf,
    /// 主机「本机」挡开录用的
    leaving: HoldBy,
    ledger: Mutex<Ledger>,
    /// 定时的一步与面板上的手动动作一个一个来：一步里删了 Fleet 房间之后，账本上这一行不能已经被放弃
    turn: tokio::sync::Mutex<()>,
}

/// 面板上对一行交还的手动动作（`POST /v1/fleet/ha/handback/{kind}/{id}/{action}`）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// 放弃交还，留在主机
    Abandon,
    /// 立即交还：不等「本机」上在投的一场投完
    Force,
}

impl Action {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "abandon" => Some(Action::Abandon),
            "force" => Some(Action::Force),
            _ => None,
        }
    }
}

/// 交还中的一行是房间还是模板
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Room,
    Template,
}

impl Kind {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "rooms" => Some(Kind::Room),
            "templates" => Some(Kind::Template),
            _ => None,
        }
    }
}

/// 「本机」上这个房间此刻在做什么（面板上「立即交还」能不能点）
async fn local_activity(services: &ServiceRegister, local: Option<i64>) -> &'static str {
    let Some(local) = local else {
        return "gone";
    };
    if adopt::recording(services, local).await {
        "recording"
    } else if adopt::busy(services, local).await {
        "uploading"
    } else {
        "idle"
    }
}

/// 一次扫描里对一行要做的
enum Step {
    DropRoom(i64),
    Settle(i64, i64),
    Unsettle(i64),
    ReleaseRoom(i64, Option<i64>),
    FinishRoom(i64),
    ReleaseTemplate(i64),
    DropTemplate(i64),
}

impl Handbacks {
    pub fn new(dir: &Path, services: &ServiceRegister) -> Self {
        Handbacks {
            path: dir.join(FILE_NAME),
            leaving: HoldBy::Leaving(identity(services)),
            ledger: Mutex::default(),
            turn: tokio::sync::Mutex::default(),
        }
    }

    /// 控制面启动时：接着上次没交完的。等「本机」开始挡的重新等空闲；已经删掉的房间在「本机」撤掉那一行
    /// 之前接着挡住开录（「本机」重启后、落地新的期望状态之前可能又监控起来）
    pub fn load(&self) {
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            return;
        };
        let mut ledger: Ledger = match serde_json::from_str(&text) {
            Ok(ledger) => ledger,
            Err(e) => {
                warn!(error = %e, "{} is not valid, ignoring it", self.path.display());
                return;
            }
        };
        for returning in ledger.nodes.values_mut() {
            for entry in returning.rooms.values_mut() {
                entry.since = None;
                if entry.stage == Stage::Released {
                    adopt::block(self.leaving, &entry.url);
                }
            }
        }
        info!(
            nodes = ?ledger.nodes.keys().collect::<Vec<_>>(),
            "配对交还：接着交还上次没交完的行"
        );
        *self.ledger.lock().unwrap() = ledger;
    }

    fn save(&self, ledger: &Ledger) {
        if ledger.nodes.is_empty() {
            remove_file(&self.path);
        } else if let Err(e) = save_json(&self.path, ledger) {
            warn!(error = ?e, "could not write {}", self.path.display());
        }
    }

    /// 解除配对时记下要交还给 `node` 的
    pub fn start(&self, node: i64, returning: Returning) {
        if returning.is_empty() {
            return;
        }
        info!(
            node,
            rooms = ?returning.rooms.keys().collect::<Vec<_>>(),
            templates = ?returning.templates.keys().collect::<Vec<_>>(),
            "配对交还：解除配对，从备机纳入的行交还给它（主机那边空闲之后交接）"
        );
        let mut ledger = self.ledger.lock().unwrap();
        ledger.nodes.insert(node, returning);
        self.save(&ledger);
    }

    fn unblock(&self, entry: &Entry) {
        if entry.since.is_some() || entry.stage == Stage::Released {
            adopt::unblock(self.leaving, &entry.url);
        }
    }

    /// 节点被移除：不再交还，行留在主机
    pub fn forget(&self, node: i64) {
        let mut ledger = self.ledger.lock().unwrap();
        let Some(returning) = ledger.nodes.remove(&node) else {
            return;
        };
        for entry in returning.rooms.values() {
            self.unblock(entry);
        }
        warn!(node, "配对交还：节点被移除，没交完的行留在主机");
        self.save(&ledger);
    }

    /// 同一台又被指定为备机：还没交的回到配对里（[`Ledger::cancel`]），返回它们的房间与模板
    pub fn cancel(&self, node: i64) -> (Vec<i64>, Vec<i64>) {
        let mut ledger = self.ledger.lock().unwrap();
        let settling: Vec<Entry> = ledger
            .nodes
            .get(&node)
            .map(|returning| returning.rooms.values().cloned().collect())
            .unwrap_or_default();
        let (rooms, templates) = ledger.cancel(node);
        if rooms.is_empty() && templates.is_empty() {
            return (rooms, templates);
        }
        for entry in settling
            .iter()
            .filter(|entry| entry.stage != Stage::Released)
        {
            self.unblock(entry);
        }
        info!(
            node,
            ?rooms,
            ?templates,
            "配对交还：又配对了，还没交的行回到配对里"
        );
        self.save(&ledger);
        (rooms, templates)
    }

    /// 给主机「本机」重发期望状态时也要重发的节点：还有挡着开录的房间（设置跟着主机上的修改）
    pub fn targets(&self, nodes: &[i64]) -> Vec<i64> {
        self.ledger
            .lock()
            .unwrap()
            .nodes
            .iter()
            .filter(|(_, returning)| returning.holding() && nodes.contains(&returning.primary))
            .map(|(node, _)| *node)
            .collect()
    }

    /// 给 `node` 下发期望状态时：还在交还的房间（按主机「本机」此刻的设置）与模板加进去，返回要带的 [`Handback`]
    pub async fn desired(
        &self,
        pool: &ConnectionPool,
        node: i64,
        proto: u32,
        rooms: &mut Vec<DesiredRoom>,
        templates: &mut Vec<DesiredTemplate>,
    ) -> AppResult<Option<Handback>> {
        if proto < PAIR_SINCE {
            return Ok(None);
        }
        let (handback, primary) = {
            let ledger = self.ledger.lock().unwrap();
            let primary = ledger.nodes.get(&node).map(|returning| returning.primary);
            (ledger.handback(node), primary)
        };
        let (Some(handback), Some(primary)) = (handback, primary) else {
            return Ok(None);
        };
        if !handback.hold.is_empty() {
            let (primary_rooms, primary_templates) =
                assignments::desired_state(pool, primary).await?;
            let kept: Vec<DesiredRoom> = primary_rooms
                .into_iter()
                .filter(|room| handback.hold.contains(&room.id))
                .collect();
            let used: BTreeSet<i64> = kept.iter().filter_map(|room| room.template_id).collect();
            for template in primary_templates {
                if used.contains(&template.id) && !templates.iter().any(|t| t.id == template.id) {
                    templates.push(template);
                }
            }
            for room in kept {
                if !rooms.iter().any(|r| r.id == room.id) {
                    rooms.push(room);
                }
            }
        }
        for id in &handback.hold_templates {
            if templates.iter().any(|t| t.id == *id) {
                continue;
            }
            if let Some(template) = assignments::template(pool, *id).await? {
                templates.push(template.desired());
            }
        }
        Ok(Some(handback))
    }

    /// 带着 `handback` 的期望状态 `version` 发给了 `node`
    pub fn sent(&self, node: i64, handback: &Handback, version: u64) {
        self.ledger.lock().unwrap().sent(node, handback, version);
    }

    /// `node` 应答了期望状态 `version`
    pub fn acked(&self, node: i64, version: u64) {
        let mut ledger = self.ledger.lock().unwrap();
        if ledger.acked(node, version) {
            self.save(&ledger);
        }
    }

    /// 定时往前走一步（[`super::pairing::Pairing::handback_tick`]）：见模块说明的第 3、4 步。
    /// 返回要重发期望状态的节点
    pub async fn tick(
        &self,
        controller: &Controller,
        services: &ServiceRegister,
        now: i64,
    ) -> Vec<i64> {
        if self.ledger.lock().unwrap().nodes.is_empty() {
            return Vec::new();
        }
        let _turn = self.turn.lock().await;
        let snapshot = self.ledger.lock().unwrap().clone();
        let pool = controller.pool();
        let local = controller
            .local()
            .map(|local| local.fleet_state())
            .unwrap_or_default();
        let mut steps: Vec<(i64, Step)> = Vec::new();
        let mut push: BTreeSet<i64> = BTreeSet::new();
        for (&node, returning) in &snapshot.nodes {
            let online = controller.node_link(node).is_some();
            let current: BTreeMap<i64, DesiredRoom> =
                match assignments::desired_state(pool, returning.primary).await {
                    Ok((rooms, _)) => rooms.into_iter().map(|room| (room.id, room)).collect(),
                    Err(e) => {
                        warn!(error = ?e, "配对交还：读不了主机「本机」的房间，这次不往下走");
                        continue;
                    }
                };
            let mut used: BTreeSet<i64> = BTreeSet::new();
            for (&id, entry) in &returning.rooms {
                match entry.stage {
                    Stage::Released => {
                        used.extend(entry.template);
                        if entry.acked && matches!(assignments::room(pool, id).await, Ok(None)) {
                            steps.push((node, Step::FinishRoom(id)));
                        }
                        continue;
                    }
                    _ if !current.contains_key(&id) => {
                        steps.push((node, Step::DropRoom(id)));
                        continue;
                    }
                    _ => used.extend(current[&id].template_id),
                }
                if entry.stage != Stage::Held {
                    continue;
                }
                let busy = match local.rooms.get(&id) {
                    Some(room) if entry.force => adopt::recording(services, room.local_id).await,
                    Some(room) => adopt::busy(services, room.local_id).await,
                    None => false,
                };
                if busy || !online {
                    if entry.since.is_some() {
                        steps.push((node, Step::Unsettle(id)));
                    }
                    continue;
                }
                match entry.since {
                    None => steps.push((node, Step::Settle(id, now))),
                    Some(since) if now - since >= SETTLE_MS => {
                        match controller.delete_room_unpushed(id).await {
                            Ok(()) | Err(DispatchError::NotFound(_)) => {
                                let template = current[&id].template_id;
                                steps.push((node, Step::ReleaseRoom(id, template)));
                                push.extend([node, returning.primary]);
                            }
                            Err(e) => warn!(room = id, error = ?e, "配对交还：删不掉 Fleet 房间"),
                        }
                    }
                    Some(_) => {}
                }
            }
            for (&id, entry) in &returning.templates {
                match entry.stage {
                    Stage::Released => {
                        if entry.acked {
                            steps.push((node, Step::DropTemplate(id)));
                        }
                    }
                    _ if used.contains(&id) => {}
                    _ => match assignments::delete_template(pool, id).await {
                        Ok(DeleteTemplate::Deleted | DeleteTemplate::InUse(_)) => {
                            steps.push((node, Step::ReleaseTemplate(id)));
                            push.extend([node, returning.primary]);
                        }
                        Ok(DeleteTemplate::NotFound) => steps.push((node, Step::DropTemplate(id))),
                        Err(e) => warn!(template = id, error = ?e, "配对交还：删不掉 Fleet 模板"),
                    },
                }
            }
        }
        if !steps.is_empty() {
            let mut ledger = self.ledger.lock().unwrap();
            for (node, step) in steps {
                self.apply(&mut ledger, node, step);
            }
            ledger.nodes.retain(|_, returning| !returning.is_empty());
            self.save(&ledger);
        }
        push.into_iter().collect()
    }

    fn apply(&self, ledger: &mut Ledger, node: i64, step: Step) {
        let Some(returning) = ledger.nodes.get_mut(&node) else {
            return;
        };
        match step {
            Step::DropRoom(id) => {
                if let Some(entry) = returning.rooms.remove(&id) {
                    self.unblock(&entry);
                    info!(
                        node,
                        room = id,
                        "配对交还：房间在主机上删掉或改派了，不再交还"
                    );
                }
            }
            Step::Settle(id, now) => {
                if let Some(entry) = returning.rooms.get_mut(&id)
                    && entry.stage == Stage::Held
                    && entry.since.is_none()
                {
                    adopt::block(self.leaving, &entry.url);
                    entry.since = Some(now);
                }
            }
            Step::Unsettle(id) => {
                if let Some(entry) = returning.rooms.get_mut(&id)
                    && entry.stage == Stage::Held
                {
                    self.unblock(entry);
                    entry.since = None;
                }
            }
            Step::ReleaseRoom(id, template) => {
                if let Some(entry) = returning.rooms.get_mut(&id) {
                    info!(
                        node,
                        room = id,
                        url = entry.url,
                        "配对交还：主机这边空闲，房间交给备机"
                    );
                    entry.advance(Stage::Released);
                    entry.template = template;
                }
            }
            Step::FinishRoom(id) => {
                if let Some(entry) = returning.rooms.remove(&id) {
                    self.unblock(&entry);
                    info!(node, room = id, "配对交还：房间交接完了");
                }
            }
            Step::ReleaseTemplate(id) => {
                if let Some(entry) = returning.templates.get_mut(&id) {
                    info!(node, template = id, "配对交还：模板交给备机");
                    entry.advance(Stage::Released);
                }
            }
            Step::DropTemplate(id) => {
                returning.templates.remove(&id);
            }
        }
    }

    /// `GET /v1/fleet/ha` 里的 `handback`：节点 id → 还没交完的行；都交完了为空
    pub fn view(&self) -> Option<Value> {
        let ledger = self.ledger.lock().unwrap();
        (!ledger.nodes.is_empty()).then(|| serde_json::to_value(&ledger.nodes).unwrap_or_default())
    }

    /// [`Self::view`] 再给面板补上：那台节点在不在线（`online`），每个房间在「本机」上此刻在录、在投还是空闲
    /// （`local`：`recording` / `uploading` / `idle`，「本机」上已经没有这一行时为 `gone`）
    pub async fn annotated(
        &self,
        controller: &Controller,
        services: &ServiceRegister,
    ) -> Option<Value> {
        let mut view = self.view()?;
        let local = controller
            .local()
            .map(|local| local.fleet_state())
            .unwrap_or_default();
        let Some(nodes) = view.as_object_mut() else {
            return Some(view);
        };
        for (node, returning) in nodes.iter_mut() {
            let online = node
                .parse::<i64>()
                .is_ok_and(|node| controller.node_link(node).is_some());
            returning["online"] = online.into();
            let Some(rooms) = returning.get_mut("rooms").and_then(Value::as_object_mut) else {
                continue;
            };
            for (id, entry) in rooms.iter_mut() {
                let row = id
                    .parse::<i64>()
                    .ok()
                    .and_then(|id| local.rooms.get(&id))
                    .map(|room| room.local_id);
                entry["local"] = local_activity(services, row).await.into();
            }
        }
        Some(view)
    }

    /// 面板上的「放弃交还」「立即交还」。成功时返回要重发期望状态的节点
    pub async fn act(
        &self,
        controller: &Controller,
        services: &ServiceRegister,
        kind: Kind,
        id: i64,
        action: Action,
    ) -> Result<i64, Refused> {
        let _turn = self.turn.lock().await;
        let found = {
            let ledger = self.ledger.lock().unwrap();
            ledger.nodes.iter().find_map(|(node, returning)| {
                let entry = match kind {
                    Kind::Room => returning.rooms.get(&id),
                    Kind::Template => returning.templates.get(&id),
                }?;
                let users: Vec<i64> = returning
                    .rooms
                    .iter()
                    .filter(|(_, room)| room.stage != Stage::Released && room.template == Some(id))
                    .map(|(room, _)| *room)
                    .collect();
                Some((*node, returning.primary, entry.clone(), users))
            })
        };
        let Some((node, primary, entry, users)) = found else {
            return Err(Refused::NotFound(match kind {
                Kind::Room => "这个房间不在交还中（可能刚交完或已经放弃）".into(),
                Kind::Template => "这个模板不在交还中（可能刚交完或已经放弃）".into(),
            }));
        };
        if entry.stage == Stage::Released {
            return Err(Refused::Conflict(
                "已经交给备机了，只差它确认，不能再放弃或重复交还".into(),
            ));
        }
        match (action, kind) {
            (Action::Abandon, Kind::Template) if !users.is_empty() => {
                Err(Refused::Conflict(format!(
                    "交还中的房间 {} 还在用这个模板：先放弃它们，或等它们交完",
                    users
                        .iter()
                        .map(i64::to_string)
                        .collect::<Vec<_>>()
                        .join("、")
                )))
            }
            (Action::Abandon, _) => {
                let mut ledger = self.ledger.lock().unwrap();
                if let Some(returning) = ledger.nodes.get_mut(&node) {
                    match kind {
                        Kind::Room => {
                            if let Some(entry) = returning.rooms.remove(&id) {
                                self.unblock(&entry);
                            }
                        }
                        Kind::Template => {
                            returning.templates.remove(&id);
                        }
                    }
                }
                ledger.nodes.retain(|_, returning| !returning.is_empty());
                self.save(&ledger);
                info!(
                    node,
                    ?kind,
                    id,
                    "配对交还：放弃交还，这一行留在主机（备机那一行在它下次落地时撤掉）"
                );
                Ok(node)
            }
            (Action::Force, Kind::Template) => Err(Refused::Invalid(
                "模板没有「立即交还」：用它的房间都交完之后它自动交还".into(),
            )),
            (Action::Force, Kind::Room) => {
                if entry.stage == Stage::Hold {
                    return Err(Refused::Conflict(
                        "备机还没确认挡住这个房间的开录（它可能离线），现在不能立即交还；可以放弃交还".into(),
                    ));
                }
                if controller.node_link(node).is_none() {
                    return Err(Refused::Conflict(
                        "备机离线时不能立即交还，等它回来；也可以放弃交还、留在主机".into(),
                    ));
                }
                let local = controller
                    .local()
                    .filter(|local| local.node_id() == Some(primary))
                    .and_then(|local| local.fleet_state().rooms.get(&id).map(|room| room.local_id));
                if let Some(local) = local
                    && adopt::recording(services, local).await
                {
                    return Err(Refused::Conflict(
                        "本机正在录这个房间：现在交还会让这一场被两台各录一段，等它录完（只剩投稿）再立即交还".into(),
                    ));
                }
                let mut ledger = self.ledger.lock().unwrap();
                if let Some(entry) = ledger
                    .nodes
                    .get_mut(&node)
                    .and_then(|returning| returning.rooms.get_mut(&id))
                {
                    entry.force = true;
                }
                self.save(&ledger);
                info!(
                    node,
                    room = id,
                    "配对交还：立即交还，本机没在录就交接，不等在投的投完"
                );
                Ok(node)
            }
        }
    }
}

impl Member {
    /// 控制面：节点标了交还、此刻还在配对里的房间与模板（Fleet id）
    pub async fn returning(&self) -> (BTreeSet<i64>, BTreeSet<i64>) {
        let state = self.state.lock().await;
        let file = &state.file;
        let mut rooms = BTreeSet::new();
        let mut templates = BTreeSet::new();
        for key in &file.adoption.returns {
            let Some(id) = file
                .book
                .get(key)
                .filter(|record| !record.deleted())
                .and_then(|record| record.fleet)
            else {
                continue;
            };
            if key.starts_with(ROOM) {
                rooms.insert(id);
            } else if key.starts_with(TEMPLATE) {
                templates.insert(id);
            }
        }
        (rooms, templates)
    }

    /// 控制面：又把同一台指定为备机时，没交的行仍记成交还给它的
    pub async fn mark_returns(&self, rooms: &[i64], templates: &[i64]) {
        let mut state = self.state.lock().await;
        let returns = &mut state.file.adoption.returns;
        returns.extend(rooms.iter().map(|id| room_key(&id.to_string())));
        returns.extend(templates.iter().map(|id| template_key(&id.to_string())));
        self.persist(&state);
    }
}

/// 解除配对时：节点标了交还、此刻还分派给主机「本机」`primary` 的房间，与还在的模板
pub async fn collect(pool: &ConnectionPool, member: &Member, primary: i64) -> AppResult<Returning> {
    let (rooms, templates) = member.returning().await;
    let mut returning = Returning {
        primary,
        ..Returning::default()
    };
    if rooms.is_empty() && templates.is_empty() {
        return Ok(returning);
    }
    let (current, _) = assignments::desired_state(pool, primary).await?;
    for room in current.iter().filter(|room| rooms.contains(&room.id)) {
        returning
            .rooms
            .insert(room.id, Entry::room(&room.spec.url, room.template_id));
    }
    for id in templates {
        if assignments::template(pool, id).await?.is_some() {
            returning.templates.insert(id, Entry::default());
        }
    }
    Ok(returning)
}

/// 节点：`data/pair-holds.json`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct HoldsFile {
    controller: String,
    #[serde(default)]
    rooms: BTreeMap<i64, String>,
}

/// 节点上交还中挡着开录的房间（控制面房间 id → 地址）。按期望状态里的 [`Handback::hold`] 挡，
/// 记在文件里：重启后、连上控制面之前也接着挡
pub struct NodeHolds {
    path: PathBuf,
    controller: String,
    by: HoldBy,
    rooms: BTreeMap<i64, String>,
    /// 不再交还、期望状态里也没有了的房间（主机上放弃交还、删掉或改派）：落地撤掉那一行之后才放开
    leaving: BTreeSet<i64>,
}

impl NodeHolds {
    pub fn resume(dir: &Path, controller: &str, services: &ServiceRegister) -> Self {
        let path = dir.join(HOLDS_FILE_NAME);
        let saved = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<HoldsFile>(&text).ok());
        let rooms = match saved {
            Some(file) if file.controller == controller => file.rooms,
            Some(_) => {
                warn!("{} belongs to another controller", path.display());
                remove_file(&path);
                BTreeMap::new()
            }
            None => BTreeMap::new(),
        };
        let holds = NodeHolds {
            path,
            controller: controller.to_string(),
            by: HoldBy::Returning(identity(services)),
            rooms,
            leaving: BTreeSet::new(),
        };
        for url in holds.rooms.values() {
            adopt::block(holds.by, url);
        }
        if !holds.rooms.is_empty() {
            info!(rooms = ?holds.rooms.keys().collect::<Vec<_>>(), "配对交还：接着挡住还在交还的房间");
        }
        holds
    }

    /// 期望状态落地之前：按 `handback.hold` 挡（地址取期望状态里的房间）；不带时一个都不挡。
    /// 交完的（`handback.rooms`）与仍在期望状态里的当场放开；期望状态里没有了的（主机上放弃交还、删掉或改派，
    /// 这次落地会撤掉那一行）接着挡到撤掉之后（[`Self::settle`]），免得撤掉之前开录一场
    pub fn apply(&mut self, desired: &DesiredState) {
        let mut wanted: BTreeMap<i64, String> = desired
            .handback
            .iter()
            .flat_map(|handback| &handback.hold)
            .filter_map(|id| {
                let room = desired.rooms.iter().find(|room| room.id == *id)?;
                Some((*id, room.spec.url.clone()))
            })
            .collect();
        let handed: &[i64] = desired
            .handback
            .as_deref()
            .map_or(&[], |handback| &handback.rooms);
        self.leaving.clear();
        for (id, url) in &self.rooms {
            if wanted.contains_key(id) {
                continue;
            }
            if !handed.contains(id) && !desired.rooms.iter().any(|room| room.id == *id) {
                wanted.insert(*id, url.clone());
                self.leaving.insert(*id);
            }
        }
        if wanted == self.rooms {
            return;
        }
        for (id, url) in &self.rooms {
            if wanted.get(id) != Some(url) {
                adopt::unblock(self.by, url);
            }
        }
        for url in wanted.values() {
            adopt::block(self.by, url);
        }
        info!(
            rooms = ?wanted.keys().collect::<Vec<_>>(),
            leaving = ?self.leaving,
            "配对交还：主播交还给本机，主机那边交接完之前挡着开录"
        );
        self.rooms = wanted;
        self.save();
    }

    /// 期望状态落地之后（与启动时）：不再交还的房间那一行撤掉了就放开
    pub fn settle(&mut self, fleet: &FleetState) {
        let gone: Vec<i64> = self
            .leaving
            .iter()
            .filter(|id| !fleet.rooms.contains_key(id))
            .copied()
            .collect();
        if gone.is_empty() {
            return;
        }
        for id in &gone {
            self.leaving.remove(id);
            if let Some(url) = self.rooms.remove(id) {
                adopt::unblock(self.by, &url);
            }
        }
        info!(rooms = ?gone, "配对交还：不再交还的房间在本机撤掉了，放开开录");
        self.save();
    }

    fn save(&self) {
        if self.rooms.is_empty() {
            remove_file(&self.path);
            return;
        }
        let file = HoldsFile {
            controller: self.controller.clone(),
            rooms: self.rooms.clone(),
        };
        if let Err(e) = save_json(&self.path, &file) {
            warn!(error = ?e, "could not write {}", self.path.display());
        }
    }

    /// 离开控制面、被移除：都放开，文件删掉
    pub fn forget(&mut self) {
        for url in self.rooms.values() {
            adopt::unblock(self.by, url);
        }
        self.rooms.clear();
        self.leaving.clear();
        remove_file(&self.path);
    }
}

fn save_json(path: &Path, value: &impl Serialize) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).change_context(AppError::Unknown)?;
    }
    let tmp = path.with_extension("json.tmp");
    let body = serde_json::to_vec_pretty(value).change_context(AppError::Unknown)?;
    {
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp)
            .change_context(AppError::Unknown)
            .attach_with(|| format!("could not write {}", tmp.display()))?;
        file.write_all(&body).change_context(AppError::Unknown)?;
        file.sync_all().change_context(AppError::Unknown)?;
    }
    std::fs::rename(&tmp, path)
        .change_context(AppError::Unknown)
        .attach_with(|| format!("could not write {}", path.display()))
}

fn remove_file(path: &Path) {
    match std::fs::remove_file(path) {
        Ok(()) => info!("{} removed", path.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warn!(error = %e, "could not remove {}", path.display()),
    }
}

/// 测试：同一进程里的两台机器，`services` 那台此刻挡着 `url` 开录没有
#[cfg(test)]
pub(crate) fn blocked_on(services: &ServiceRegister, url: &str) -> bool {
    adopt::blocked(identity(services), url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::fleet::ha::adopt::holding;
    use crate::server::fleet::ha::member::tests::services;
    use serde_json::json;

    fn ledger() -> Ledger {
        let mut returning = Returning {
            primary: 1,
            ..Returning::default()
        };
        returning
            .rooms
            .insert(7, Entry::room("https://handback.example/7", Some(3)));
        returning
            .rooms
            .insert(8, Entry::room("https://handback.example/8", None));
        returning.templates.insert(3, Entry::default());
        Ledger {
            nodes: BTreeMap::from([(5, returning)]),
        }
    }

    fn room(ledger: &mut Ledger, id: i64) -> &mut Entry {
        ledger
            .nodes
            .get_mut(&5)
            .and_then(|returning| returning.rooms.get_mut(&id))
            .unwrap()
    }

    /// 每一步都要节点应答了带着它的期望状态才算：离线时停在原地，应答更早的版本不算，同一步只认第一次发出
    #[test]
    fn a_step_counts_only_once_the_node_acks_a_desired_state_that_carried_it() {
        let mut ledger = ledger();
        assert!(ledger.handback(6).is_none());
        let handback = ledger.handback(5).unwrap();
        assert_eq!(
            handback,
            Handback {
                hold: vec![7, 8],
                hold_templates: vec![3],
                ..Handback::default()
            }
        );

        assert!(!ledger.acked(5, 10), "节点离线、没收到挡开录");
        assert_eq!(room(&mut ledger, 7).stage, Stage::Hold);
        ledger.sent(5, &handback, 12);
        ledger.sent(5, &handback, 13);
        assert!(!ledger.acked(5, 11));
        assert!(ledger.acked(5, 12));
        assert_eq!(room(&mut ledger, 7).stage, Stage::Held);
        assert_eq!(room(&mut ledger, 8).stage, Stage::Held);

        room(&mut ledger, 7).advance(Stage::Released);
        let handback = ledger.handback(5).unwrap();
        assert_eq!((&handback.hold, &handback.rooms), (&vec![8], &vec![7]));
        ledger.acked(5, 13);
        assert!(!room(&mut ledger, 7).acked, "带着交接这一步之前的版本不算");
        ledger.sent(5, &handback, 14);
        ledger.acked(5, 14);
        assert!(room(&mut ledger, 7).acked);
        assert_eq!(room(&mut ledger, 8).stage, Stage::Held);
    }

    /// 没有交还时期望状态里没有这个字段（与以前逐字相同）；有时原样来回
    #[test]
    fn the_wire_form_is_absent_without_a_handback_and_round_trips() {
        let plain = serde_json::to_value(DesiredState {
            version: 3,
            ..DesiredState::default()
        })
        .unwrap();
        assert!(plain.get("handback").is_none());
        let desired = DesiredState {
            version: 4,
            handback: Some(Box::new(Handback {
                hold: vec![7],
                rooms: vec![8],
                ..Handback::default()
            })),
            ..DesiredState::default()
        };
        let wire = serde_json::to_value(&desired).unwrap();
        assert_eq!(wire["handback"], json!({ "hold": [7], "rooms": [8] }));
        let back: DesiredState = serde_json::from_value(wire).unwrap();
        assert_eq!(back.handback, desired.handback);
    }

    /// 同一台又被指定为备机：还没交的回到配对里，交完的照常收尾
    #[test]
    fn a_new_pairing_takes_back_what_was_not_handed_over_yet() {
        let mut ledger = ledger();
        room(&mut ledger, 7).advance(Stage::Released);
        assert_eq!(ledger.cancel(5), (vec![8], vec![3]));
        assert_eq!(
            ledger.nodes[&5].rooms.keys().copied().collect::<Vec<_>>(),
            [7]
        );
        assert_eq!(ledger.cancel(5), (vec![], vec![]));
        ledger.nodes.get_mut(&5).unwrap().rooms.clear();
        ledger.cancel(5);
        assert!(ledger.nodes.is_empty());
        assert_eq!(ledger.cancel(6), (vec![], vec![]));
    }

    /// 控制面重启：账本还在，每一步重发一遍；已经删掉的房间在「本机」撤掉之前接着挡住它开录，
    /// 正在等空闲的重新等。节点被移除时都放开、文件删掉
    #[tokio::test]
    async fn the_ledger_survives_a_restart_and_steps_are_sent_again() {
        let dir = tempfile::tempdir().unwrap();
        let services = services(&dir.path().join("controller")).await;
        let (released, settling) = (
            "https://handback.example/restart-7",
            "https://handback.example/restart-8",
        );
        let mut returning = Returning {
            primary: 1,
            ..Returning::default()
        };
        let mut entry = Entry::room(released, None);
        entry.advance(Stage::Released);
        entry.sent = Some(3);
        entry.acked = true;
        returning.rooms.insert(7, entry);
        let mut entry = Entry::room(settling, None);
        entry.advance(Stage::Held);
        entry.since = Some(1);
        returning.rooms.insert(8, entry);
        let handbacks = Handbacks::new(dir.path(), &services);
        handbacks.start(5, returning);
        handbacks.start(6, Returning::default());
        assert!(dir.path().join(FILE_NAME).exists());
        assert!(holding(released).is_none());

        let again = Handbacks::new(dir.path(), &services);
        again.load();
        let mut ledger = again.ledger.lock().unwrap().clone();
        assert_eq!(ledger.nodes.keys().copied().collect::<Vec<_>>(), [5]);
        let entry = room(&mut ledger, 7).clone();
        assert_eq!(
            (entry.stage, entry.sent, entry.acked),
            (Stage::Released, None, false)
        );
        let entry = room(&mut ledger, 8).clone();
        assert_eq!((entry.stage, entry.since), (Stage::Held, None));
        let hold = holding(released).expect("「本机」撤掉之前接着挡");
        assert!(hold.quick);
        assert!(holding(settling).is_none());
        assert_eq!(again.targets(&[1]), [5]);
        assert!(again.targets(&[2]).is_empty());
        assert_eq!(again.view().unwrap()["5"]["rooms"]["8"]["stage"], "held");

        again.forget(5);
        assert!(holding(released).is_none());
        assert!(!dir.path().join(FILE_NAME).exists());
        assert!(again.view().is_none());
    }

    /// 节点按期望状态里的 `hold` 挡开录，一个不多一个不少；记在文件里，重启后连上控制面之前也接着挡；
    /// 期望状态不再带时都放开
    #[tokio::test]
    async fn a_node_holds_exactly_the_listed_rooms_across_restarts() {
        let dir = tempfile::tempdir().unwrap();
        let services = services(&dir.path().join("node")).await;
        let (a, b) = (
            "https://handback.example/node-a",
            "https://handback.example/node-b",
        );
        let desired = |hold: &[i64]| DesiredState {
            rooms: [(1, a), (2, b)]
                .into_iter()
                .map(|(id, url)| {
                    serde_json::from_value(
                        json!({ "id": id, "epoch": 1, "url": url, "remark": "r" }),
                    )
                    .unwrap()
                })
                .collect(),
            handback: (!hold.is_empty()).then(|| {
                Box::new(Handback {
                    hold: hold.to_vec(),
                    ..Handback::default()
                })
            }),
            ..DesiredState::default()
        };
        let mut holds = NodeHolds::resume(dir.path(), "controller", &services);
        holds.apply(&desired(&[1, 2, 3]));
        let hold = holding(a).expect("交还中挡着开录");
        assert!(!hold.quick, "可能要等很久，按检测周期再看");
        assert!(holding(b).is_some());
        holds.apply(&desired(&[2]));
        assert!(holding(a).is_none() && holding(b).is_some());
        assert!(dir.path().join(HOLDS_FILE_NAME).exists());

        adopt::unblock(holds.by, b);
        let mut holds = NodeHolds::resume(dir.path(), "controller", &services);
        assert!(holding(b).is_some(), "重启后接着挡");
        holds.apply(&desired(&[]));
        assert!(holding(b).is_none());
        assert!(!dir.path().join(HOLDS_FILE_NAME).exists());

        holds.apply(&desired(&[1]));
        adopt::unblock(holds.by, a);
        let _other = NodeHolds::resume(dir.path(), "another", &services);
        assert!(holding(a).is_none(), "别的控制面留下的不挡");
        assert!(!dir.path().join(HOLDS_FILE_NAME).exists());
        holds.forget();
        assert!(holding(a).is_none());
    }

    /// 主机上放弃交还（或删掉、改派）之后，期望状态里既没有这个房间、也不在交完的里：节点先接着挡，
    /// 落地撤掉那一行之后才放开，撤掉之前不会开录一场。交完的与仍在期望状态里的当场放开
    #[tokio::test]
    async fn a_room_no_longer_handed_back_stays_held_until_its_row_is_gone() {
        let dir = tempfile::tempdir().unwrap();
        let services = services(&dir.path().join("node")).await;
        let (a, b, c) = (
            "https://handback.example/settle-a",
            "https://handback.example/settle-b",
            "https://handback.example/settle-c",
        );
        let room = |id: i64, url: &str| -> DesiredRoom {
            serde_json::from_value(json!({ "id": id, "epoch": 1, "url": url, "remark": "r" }))
                .unwrap()
        };
        let managed = |ids: &[(i64, &str)]| FleetState {
            rooms: ids
                .iter()
                .map(|(id, url)| {
                    let row =
                        serde_json::from_value(json!({ "local_id": id, "epoch": 1, "url": url }))
                            .unwrap();
                    (*id, row)
                })
                .collect(),
            ..FleetState::default()
        };
        let mut holds = NodeHolds::resume(dir.path(), "controller", &services);
        holds.apply(&DesiredState {
            rooms: vec![room(1, a), room(2, b), room(3, c)],
            handback: Some(Box::new(Handback {
                hold: vec![1, 2, 3],
                ..Handback::default()
            })),
            ..DesiredState::default()
        });
        assert!(holding(a).is_some() && holding(b).is_some() && holding(c).is_some());

        // a 放弃交还、b 交完了、c 还在交还
        holds.apply(&DesiredState {
            rooms: vec![room(3, c)],
            handback: Some(Box::new(Handback {
                hold: vec![3],
                rooms: vec![2],
                ..Handback::default()
            })),
            ..DesiredState::default()
        });
        assert!(holding(a).is_some(), "撤掉那一行之前接着挡");
        assert!(holding(b).is_none(), "交完的当场放开");
        assert!(holding(c).is_some());
        holds.settle(&managed(&[(1, a), (3, c)]));
        assert!(holding(a).is_some(), "那一行还在（落地失败）就接着挡");

        let mut restarted = NodeHolds::resume(dir.path(), "controller", &services);
        assert!(holding(a).is_some(), "重启后接着挡");
        restarted.apply(&DesiredState {
            rooms: vec![room(3, c)],
            handback: Some(Box::new(Handback {
                hold: vec![3],
                ..Handback::default()
            })),
            ..DesiredState::default()
        });
        restarted.settle(&managed(&[(3, c)]));
        assert!(holding(a).is_none(), "撤掉之后放开");
        assert!(holding(c).is_some());

        // 最后一行也放弃了：期望状态不再带交还，同样等撤掉
        restarted.apply(&DesiredState::default());
        assert!(holding(c).is_some());
        restarted.settle(&managed(&[]));
        assert!(holding(c).is_none());
        assert!(!dir.path().join(HOLDS_FILE_NAME).exists());
    }

    /// 放弃、立即交还只认 `rooms` / `templates` 与 `abandon` / `force`；账本里的 `force` 不改变没点过时的文件
    #[test]
    fn actions_parse_and_force_is_absent_until_used() {
        assert_eq!(Kind::parse("rooms"), Some(Kind::Room));
        assert_eq!(Kind::parse("templates"), Some(Kind::Template));
        assert_eq!(Kind::parse("room"), None);
        assert_eq!(Action::parse("abandon"), Some(Action::Abandon));
        assert_eq!(Action::parse("force"), Some(Action::Force));
        assert_eq!(Action::parse("drop"), None);
        let mut ledger = ledger();
        let plain = serde_json::to_value(&ledger).unwrap();
        assert!(plain["nodes"]["5"]["rooms"]["7"].get("force").is_none());
        room(&mut ledger, 7).advance(Stage::Held);
        room(&mut ledger, 7).force = true;
        let text = serde_json::to_string(&ledger).unwrap();
        let back: Ledger = serde_json::from_str(&text).unwrap();
        assert!(back.nodes[&5].rooms[&7].force);
        room(&mut ledger, 7).advance(Stage::Released);
        assert!(!room(&mut ledger, 7).force, "交接之后不再带");
    }
}
