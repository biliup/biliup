//! 把配对之外的本地主播与模板加入配对（ha-pair 方案 H2）。
//!
//! 指定备机时，备机上已有的本地主播与模板缺省全部纳入（`PUT /v1/fleet/ha` 的 `adopt` 可以勾掉其中一些，
//! 勾掉的留作备机的本地行）；配对之后也能按 id 把两台上还没纳入的本地行加进来（`POST /v1/fleet/ha/join`、
//! `POST /v1/node/ha/join`）。清单与不能加入的原因见 [`local_rows`] 与 [`evaluate`]。
//!
//! 备机上的行走现成的「节点新建的主播」路径：排进同步队列、控制面仲裁（[`super::rooms::apply_room`]），
//! 收下后备机按控制面 id 原地认下，不重建监控；收不下的留作本地行，原因记下来。主机「本机」上的行由控制面
//! 直接建 Fleet 房间，「本机」落地时按提示原地认下原来那一行（`Reconciler::adopt_local`）。
//!
//! 不中断录制、不出双稿：主播只在空闲（没在录、没在投）时加入，正在录的等它录完投完。加入之前先按地址
//! 挡住开录（[`holding`]），隔一个扫描周期确认仍然空闲才发出，一直挡到两台都按配对的房间落地。于是加入之前
//! 开录的一场整场按本地行录完投完，加入之后开录的一场两台都按配对的场次走（谁投由场次消息决定）。
//! 录完还在上传池里排队（投稿流程没开始，看着是空闲）的一场，备机按开始加入的时刻认出它是本地行录的
//! （[`joined_after`]），照本地行投。
//! 发出后迟迟落不了地（连接断了）时过一会儿放开，宁可这一场按本地行录，也不漏录（§6 D）。

use super::Hold;
use super::agent::UNIT_KEEP_MS;
use super::member::{Member, State, identity};
use super::outbox::PairFile;
use super::rooms::{self, SYNC_DOWNLOADER, URL_TAKEN, internal};
use super::sync::{AdoptRequest, LocalRow, ROOM, RowState, Side, TEMPLATE};
use super::sync_downloader;
use crate::server::config::{Config, ConfigPatch};
use crate::server::fleet::accounts::{self, LocalAccount};
use crate::server::fleet::assignments;
use crate::server::fleet::controller::{Controller, check_room_spec};
use crate::server::fleet::model::RoomSpec;
use crate::server::fleet::now_ms;
use crate::server::fleet::reconcile::{self, FleetState};
use crate::server::fleet::store;
use crate::server::infrastructure::context::WorkerStatus;
use crate::server::infrastructure::models::live_streamer::LiveStreamer;
use crate::server::infrastructure::models::upload_streamer::UploadStreamer;
use crate::server::infrastructure::service_register::ServiceRegister;
use ormlite::Model;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{info, warn};

/// 挡住开录之后至少隔这么久、仍然空闲才加入：检测到开播、还没来得及标成「在录」的那一刻不会漏看
pub(super) const SETTLE_MS: i64 = 3_000;
/// 发出之后这么久还没两台落地（连接断了）就放开开录
pub(super) const HOLD_MS: i64 = 60_000;

const HOOKS_NODE: &str =
    "带 run 命令（能执行任意命令）的主播不能从节点加入配对：在这台上去掉 run 步骤之后再加";
const HOOKS_LOCAL: &str =
    "「本机」节点启用时没有勾选「允许钩子」，带 run 命令（能执行任意命令）的主播不能加入配对";
const MISSING: &str = "这台机器上没有这个本地行，或它已经在配对里";

/// `pair-outbox.json` 里要加入配对的本地行（都是这台机器上的本地 id）
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Adoption {
    /// 节点：处理过的控制面请求
    #[serde(default)]
    pub generation: u64,
    /// 控制面：请节点加入的本地行，随期望状态带给它
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<AdoptRequest>,
    /// 等空闲的主播
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub rooms: BTreeSet<i64>,
    /// 要单独加入的模板
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub templates: BTreeSet<i64>,
    /// 正在加入的主播：挡着开录，直到两台落地
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub joining: BTreeMap<i64, Joining>,
    /// 上次加入时没被收下的原因
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub refused_rooms: BTreeMap<i64, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub refused_templates: BTreeMap<i64, String>,
    /// 节点：单独加入的模板的账本键（发出时带 `pin`）
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub pins: BTreeSet<String>,
    /// 控制面：单独加入配对的 Fleet 模板，没有房间用也留在配对里
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub pinned: BTreeSet<i64>,
    /// 控制面：「本机」落地时原地认下的本地行，Fleet id → 本地 id
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub hint_rooms: BTreeMap<i64, i64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub hint_templates: BTreeMap<i64, i64>,
    /// 节点：第一次配对时这台机器上的本地行（不含 Fleet 托管行）
    #[serde(default, skip_serializing_if = "Before::is_empty")]
    pub before: Before,
    /// 节点：纳入配对的、配对之前就在这台上的行的账本键（发出时带 `returns`，解除配对时交还给这台）
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub adopted: BTreeSet<String>,
    /// 控制面：节点标了 `returns` 的账本键（[`super::handback`]）
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub returns: BTreeSet<String>,
}

/// 节点第一次配对时这台机器上的本地行
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Before {
    /// 本地主播 id → 地址。库里的 id 会被复用，地址也对得上才算配对之前的那一行
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub rooms: BTreeMap<i64, String>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub templates: BTreeSet<i64>,
}

impl Before {
    fn is_empty(&self) -> bool {
        self.rooms.is_empty() && self.templates.is_empty()
    }
}

impl Adoption {
    /// 有没有要往前走的
    pub fn pending(&self) -> bool {
        !self.rooms.is_empty() || !self.templates.is_empty() || !self.joining.is_empty()
    }

    /// 排上要加入的本地行，之前没被收下的原因划掉
    pub fn queue(&mut self, rooms: &[i64], templates: &[i64]) -> bool {
        for id in rooms {
            self.refused_rooms.remove(id);
            self.rooms.insert(*id);
        }
        for id in templates {
            self.refused_templates.remove(id);
            self.templates.insert(*id);
        }
        !rooms.is_empty() || !templates.is_empty()
    }

    /// 节点：期望状态里控制面的加入请求，只收上次之后才请求的行
    pub fn take(&mut self, request: Option<&AdoptRequest>) -> bool {
        let Some(request) = request.filter(|request| request.generation > self.generation) else {
            return false;
        };
        let (rooms, templates) = request.since(self.generation);
        self.generation = request.generation;
        self.queue(&rooms, &templates);
        info!(?rooms, ?templates, "配对：控制面请本机把这些本地行加入配对");
        true
    }
}

/// 正在加入的一个主播
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Joining {
    /// 挡着开录的地址
    pub url: String,
    /// 开始挡的时刻
    pub since: i64,
    /// 节点：发出的账本键
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// 控制面：建好的 Fleet 房间
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fleet: Option<i64>,
}

impl Joining {
    fn new(url: String, since: i64) -> Self {
        Joining {
            url,
            since,
            ..Joining::default()
        }
    }

    fn sent(&self) -> bool {
        self.key.is_some() || self.fleet.is_some()
    }
}

/// 谁挡着开录。都按这台机器的 [`identity`] 分开，同一进程里的两台（测试）互不放开对方挡的
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HoldBy {
    /// 同步端：正在加入配对
    Join(usize),
    /// 解除配对后主机「本机」：正在把主播交还给备机（[`super::handback`]）
    Leaving(usize),
    /// 解除配对后的备机：主播正在交还回来，主机那边交接完之前不录
    Returning(usize),
}

impl HoldBy {
    fn reason(self) -> &'static str {
        match self {
            HoldBy::Join(_) => "正在加入配对：两台都按配对的房间落地之后再录",
            HoldBy::Leaving(_) => "配对已解除，这个主播正在交还给备机：交接完之后由备机录",
            HoldBy::Returning(_) => {
                "配对已解除，这个主播正在交还给这台：主机那边这一场录完投完、交接完之后再录"
            }
        }
    }
}

static HOLDING: AtomicBool = AtomicBool::new(false);
/// 挡着开录的地址与挡它的
static HOLDS: Mutex<Vec<(HoldBy, String)>> = Mutex::new(Vec::new());
/// 开始加入配对的主播：（同步端, 地址）→ 最近一次开始挡开录的时刻（[`joined_after`]）
static JOINED: Mutex<BTreeMap<(usize, String), i64>> = Mutex::new(BTreeMap::new());

/// 监控循环开录之前（[`super::hold_recording`]）：正在加入配对、正在交还的主播先不录。
/// 都没有时只读一次原子变量
pub(super) fn holding(url: &str) -> Option<Hold> {
    if !HOLDING.load(Ordering::Acquire) {
        return None;
    }
    let by = HOLDS
        .lock()
        .unwrap()
        .iter()
        .find(|(_, held)| held == url)
        .map(|(by, _)| *by)?;
    Some(Hold {
        reason: by.reason().into(),
        // 交还回来的要等主机那边录完投完，可能很久，按检测周期再看
        quick: !matches!(by, HoldBy::Returning(_)),
    })
}

/// 按地址挡住开录（同一个挡的重复调用只算一次）
pub(super) fn block(by: HoldBy, url: &str) {
    let mut holds = HOLDS.lock().unwrap();
    if !holds
        .iter()
        .any(|(held_by, held)| *held_by == by && held == url)
    {
        holds.push((by, url.to_string()));
    }
    HOLDING.store(true, Ordering::Release);
}

pub(super) fn unblock(by: HoldBy, url: &str) {
    let mut holds = HOLDS.lock().unwrap();
    holds.retain(|(held_by, held)| !(*held_by == by && held == url));
    HOLDING.store(!holds.is_empty(), Ordering::Release);
}

/// 备机投稿流程开始时（[`super::agent::Standby::plan`]）：这一段开录之后这台机器才开始把这个地址加入配对，
/// 即它是本地行录的。录完在上传池里排着队、加入之后才轮到投的这一段照本地行投，不当成开录时没认下的镜像房间
pub(super) fn joined_after(services: &ServiceRegister, url: &str, started_at: i64) -> bool {
    JOINED
        .lock()
        .unwrap()
        .get(&(identity(services), url.to_string()))
        .is_some_and(|since| *since >= started_at)
}

pub(super) fn hold(owner: usize, url: &str, since: i64) {
    {
        let mut joined = JOINED.lock().unwrap();
        joined.retain(|_, at| *at >= since - UNIT_KEEP_MS);
        joined.insert((owner, url.to_string()), since);
    }
    block(HoldBy::Join(owner), url);
}

fn release(owner: usize, url: &str) {
    unblock(HoldBy::Join(owner), url);
}

/// 同步端停下：它为加入配对挡的都放开，加入记录也清掉（交还挡的不在这里放）
pub(super) fn release_all(owner: usize) {
    JOINED.lock().unwrap().retain(|(by, _), _| *by != owner);
    let mut holds = HOLDS.lock().unwrap();
    holds.retain(|(by, _)| *by != HoldBy::Join(owner));
    HOLDING.store(!holds.is_empty(), Ordering::Release);
}

/// 这台机器上 `fleet-state.json` 记的 Fleet 托管行（含配对里的行）；没有这个文件时为空
pub fn fleet_state(dir: &Path) -> FleetState {
    reconcile::read_state(&dir.join(reconcile::STATE_FILE_NAME)).unwrap_or_default()
}

/// 正在录，或上一场还没投完
pub(super) async fn busy(services: &ServiceRegister, local: i64) -> bool {
    let Some(worker) = services.managers.get_room_by_id(local).await else {
        return false;
    };
    let recording = matches!(
        *worker.downloader_status.read().unwrap(),
        WorkerStatus::Working(_)
    );
    recording
        || matches!(
            *worker.uploader_status.read().unwrap(),
            WorkerStatus::Pending
        )
}

/// 正在录（不管有没有在投）
pub(super) async fn recording(services: &ServiceRegister, local: i64) -> bool {
    let Some(worker) = services.managers.get_room_by_id(local).await else {
        return false;
    };
    matches!(
        *worker.downloader_status.read().unwrap(),
        WorkerStatus::Working(_)
    )
}

/// 测试：同一进程里的两台各自挡着哪些地址
#[cfg(test)]
pub(super) fn blocked(owner: usize, url: &str) -> bool {
    HOLDS.lock().unwrap().iter().any(|(by, held)| {
        held == url
            && matches!(by, HoldBy::Join(id) | HoldBy::Leaving(id) | HoldBy::Returning(id) if *id == owner)
    })
}

fn downloader_of(spec: &RoomSpec) -> Option<Value> {
    let patch = serde_json::to_value(spec.override_cfg.as_ref()?).ok()?;
    patch
        .get("downloader")
        .filter(|value| !value.is_null())
        .cloned()
}

/// 模板不能加入的原因（两台都一样的那部分）
async fn template_reason(
    services: &ServiceRegister,
    accounts: &[LocalAccount],
    row: &UploadStreamer,
) -> Option<String> {
    if rooms::node_template(services, accounts, row.id)
        .await
        .is_none()
    {
        return Some("读不出这个投稿模板".into());
    }
    if row.template_name.trim().is_empty() {
        return Some("模板名为空".into());
    }
    match row.user_cookie.as_deref().filter(|path| !path.is_empty()) {
        Some(path) if !accounts.iter().any(|account| account.path == path) => {
            Some("模板用的凭据文件没有登记成 B 站账号：到「账号」页登录或登记之后再加".into())
        }
        _ => None,
    }
}

fn sent(file: Option<&PairFile>, prefix: &str, local: i64) -> bool {
    file.and_then(|file| {
        let key = file.book.key_of_local(prefix, local)?;
        file.book.get(&key).map(|record| !record.deleted())
    })
    .unwrap_or(false)
}

/// 本机还没纳入配对的主播与模板：除掉 `fleet` 里的托管行（含配对里的行）。`file` 是配对中的同步账本
/// （没配对时为空），按它标出等空闲的、已经发出的与上次没被收下的。原因只含两台都一样的那部分
pub async fn local_rows(
    services: &ServiceRegister,
    fleet: &FleetState,
    file: Option<&PairFile>,
) -> (Vec<LocalRow>, Vec<LocalRow>) {
    let accounts = accounts::scan(&services.pool).await;
    let adoption = file.map(|file| &file.adoption);
    let managed: BTreeSet<i64> = fleet.templates.values().map(|t| t.local_id).collect();
    let mut templates = Vec::new();
    let mut unusable: HashMap<i64, String> = HashMap::new();
    let rows = UploadStreamer::select()
        .fetch_all(&services.pool)
        .await
        .unwrap_or_default();
    for row in rows.into_iter().filter(|row| !managed.contains(&row.id)) {
        let mut item = LocalRow {
            id: row.id,
            name: row.template_name.clone(),
            ..LocalRow::default()
        };
        item.reason = template_reason(services, &accounts, &row).await;
        if let Some(reason) = &item.reason {
            unusable.insert(row.id, reason.clone());
        }
        item.state = if sent(file, TEMPLATE, row.id) {
            RowState::Joining
        } else if adoption.is_some_and(|a| a.templates.contains(&row.id)) {
            RowState::Waiting
        } else {
            RowState::Local
        };
        item.refused = adoption.and_then(|a| a.refused_templates.get(&row.id).cloned());
        templates.push(item);
    }

    let managed: BTreeSet<i64> = fleet.rooms.values().map(|room| room.local_id).collect();
    let adding: BTreeSet<&str> = fleet.adding.values().map(String::as_str).collect();
    let mut rooms = Vec::new();
    let rows = LiveStreamer::select()
        .fetch_all(&services.pool)
        .await
        .unwrap_or_default();
    for row in rows {
        if managed.contains(&row.id) || adding.contains(row.url.as_str()) {
            continue;
        }
        let mut item = LocalRow {
            id: row.id,
            name: row.remark.clone(),
            url: Some(row.url.clone()),
            template: row.upload_streamers_id,
            busy: busy(services, row.id).await,
            ..LocalRow::default()
        };
        match reconcile::room_spec_of(&row) {
            Some((spec, _)) => {
                item.hooks = spec.has_hooks();
                item.downloader = downloader_of(&spec);
            }
            None => item.reason = Some("读不出这个主播的设置".into()),
        }
        if item.reason.is_none()
            && let Some(reason) = row.upload_streamers_id.and_then(|t| unusable.get(&t))
        {
            item.reason = Some(format!("它用的投稿模板不能加入：{reason}"));
        }
        let joining = adoption.and_then(|a| a.joining.get(&row.id));
        item.state = if sent(file, ROOM, row.id) || joining.is_some_and(Joining::sent) {
            RowState::Joining
        } else if joining.is_some() || adoption.is_some_and(|a| a.rooms.contains(&row.id)) {
            RowState::Waiting
        } else {
            RowState::Local
        };
        item.refused = adoption.and_then(|a| a.refused_rooms.get(&row.id).cloned());
        rooms.push(item);
    }
    (rooms, templates)
}

/// 节点自己能判断的：带 run 命令的主播控制面不收
pub fn node_checks(rooms: &mut [LocalRow]) {
    for row in rooms.iter_mut().filter(|row| row.reason.is_none()) {
        if row.hooks {
            row.reason = Some(HOOKS_NODE.into());
        }
    }
}

/// 控制面判断 `side` 那台上的本地主播能不能加入配对：`fleet_urls` 是控制面房间列表里的地址 → 房间 id，
/// `other` 是另一台上还没纳入配对的本地主播（地址 → 本地 id，读不到另一台时为空），
/// `hooks_allowed` 是主机「本机」节点允许带 run 命令的房间。与控制面仲裁（[`super::rooms::apply_room`]）同样的规矩
pub fn evaluate(
    rooms: &mut [LocalRow],
    side: Side,
    config: &Config,
    fleet_urls: &HashMap<String, i64>,
    other: &HashMap<String, i64>,
    hooks_allowed: bool,
) {
    for row in rooms
        .iter_mut()
        .filter(|row| row.state == RowState::Local && row.reason.is_none())
    {
        row.reason = room_reason(row, side, config, fleet_urls, other, hooks_allowed);
    }
}

fn room_reason(
    row: &LocalRow,
    side: Side,
    config: &Config,
    fleet_urls: &HashMap<String, i64>,
    other: &HashMap<String, i64>,
    hooks_allowed: bool,
) -> Option<String> {
    if row.hooks {
        match side {
            Side::Node => return Some(HOOKS_NODE.into()),
            Side::Controller if !hooks_allowed => return Some(HOOKS_LOCAL.into()),
            Side::Controller => {}
        }
    }
    let patch = row.downloader.clone().and_then(|downloader| {
        serde_json::from_value::<ConfigPatch>(serde_json::json!({ "downloader": downloader })).ok()
    });
    if sync_downloader(config, patch) {
        return Some(SYNC_DOWNLOADER.into());
    }
    let url = row.url.as_deref()?;
    if let Some(id) = fleet_urls.get(url) {
        return Some(format!("{URL_TAKEN}（房间 {id}）"));
    }
    let machine = match side {
        Side::Node => "主机",
        Side::Controller => "备机",
    };
    other.get(url).map(|id| {
        format!(
            "{machine}上也有同一地址的本地主播（id {id}）：两台是同一个主播，先删掉其中一台上的这一行，再把留下的那行加入配对"
        )
    })
}

/// 本地主播的地址 → 本地 id（判断另一台上有没有同一个主播）
pub fn urls(rooms: &[LocalRow]) -> HashMap<String, i64> {
    rooms
        .iter()
        .filter_map(|row| Some((row.url.clone()?, row.id)))
        .collect()
}

/// 按请求挑出这一次要加入的（`wanted` 为空表示全部），逐条标上 `included`；请求里有、清单里没有的 id 也列出来
pub fn select(rows: &mut Vec<LocalRow>, wanted: Option<&[i64]>) -> Vec<i64> {
    let mut picked = Vec::new();
    for row in rows.iter_mut() {
        let asked = wanted.is_none_or(|ids| ids.contains(&row.id));
        let included = asked && row.reason.is_none();
        row.included = Some(included);
        if included && row.state == RowState::Local {
            picked.push(row.id);
        }
    }
    for id in wanted.unwrap_or_default() {
        if !rows.iter().any(|row| row.id == *id) {
            rows.push(LocalRow {
                id: *id,
                reason: Some(MISSING.into()),
                included: Some(false),
                ..LocalRow::default()
            });
        }
    }
    picked
}

/// 模板：这次加入的主播用到的随主播一起加入（勾掉了也一样，配对里的房间不能用本地模板），
/// 其余选中的单独加入
pub fn select_templates(
    templates: &mut Vec<LocalRow>,
    wanted: Option<&[i64]>,
    rooms: &[LocalRow],
) -> Vec<i64> {
    let used: BTreeSet<i64> = rooms
        .iter()
        .filter(|room| room.included == Some(true))
        .filter_map(|room| room.template)
        .collect();
    let mut picked = select(templates, wanted);
    picked.retain(|id| !used.contains(id));
    for row in templates.iter_mut().filter(|row| used.contains(&row.id)) {
        row.included = Some(true);
    }
    picked
}

impl Member {
    /// 节点第一次配对时：记下这台机器上此刻的本地主播与模板（Fleet 托管行不算）。之后纳入配对的这些行
    /// 解除配对时交还给这台（[`super::handback`]）；配对期间在这台上新建的不在里面，解除配对后留在主机
    pub(super) async fn remember_local(&self, state: &mut State) {
        let fleet = fleet_state(&self.dir);
        let managed_rooms: BTreeSet<i64> = fleet.rooms.values().map(|room| room.local_id).collect();
        let managed_templates: BTreeSet<i64> =
            fleet.templates.values().map(|t| t.local_id).collect();
        let rooms = LiveStreamer::select()
            .fetch_all(&self.services.pool)
            .await
            .unwrap_or_default();
        let templates = UploadStreamer::select()
            .fetch_all(&self.services.pool)
            .await
            .unwrap_or_default();
        let before = &mut state.file.adoption.before;
        before.rooms = rooms
            .into_iter()
            .filter(|row| !managed_rooms.contains(&row.id))
            .map(|row| (row.id, row.url))
            .collect();
        before.templates = templates
            .into_iter()
            .map(|row| row.id)
            .filter(|id| !managed_templates.contains(id))
            .collect();
    }

    /// 重启后接着挡住正在加入的主播
    pub(super) fn rearm(&self, state: &State) {
        let owner = identity(&self.services);
        for joining in state.file.adoption.joining.values() {
            hold(owner, &joining.url, joining.since);
        }
    }

    /// 对端没收下本机加入的行：放开开录、记下原因，行留作本机的
    pub(super) fn refused(&self, state: &mut State, key: &str, local: i64, reason: &str) {
        let adoption = &mut state.file.adoption;
        if key.starts_with(ROOM) {
            if let Some(joining) = adoption.joining.remove(&local) {
                release(identity(&self.services), &joining.url);
            }
            adoption.refused_rooms.insert(local, reason.to_string());
        } else {
            adoption.pins.remove(key);
            adoption.refused_templates.insert(local, reason.to_string());
        }
    }

    /// 加入的主播两台都按配对的房间落地了（节点：期望状态落地之后；控制面：「本机」落地之后）：放开开录。
    /// 控制面上认下了的提示不再给
    pub(super) fn landed(&self, state: &mut State, fleet: &FleetState) -> bool {
        let owner = identity(&self.services);
        let file = &mut state.file;
        let done: Vec<i64> = file
            .adoption
            .joining
            .iter()
            .filter(|(local, joining)| {
                let record = joining.key.as_ref().map(|key| file.book.get(key));
                let id = joining
                    .fleet
                    .or_else(|| record.flatten().and_then(|record| record.fleet));
                let dropped = matches!(record, Some(None));
                dropped
                    || id
                        .is_some_and(|id| fleet.rooms.get(&id).map(|r| r.local_id) == Some(**local))
            })
            .map(|(local, _)| *local)
            .collect();
        let mut changed = !done.is_empty();
        for local in done {
            if let Some(joining) = file.adoption.joining.remove(&local) {
                release(owner, &joining.url);
                info!(streamer = local, "配对：加入的主播两台都落地了，放开开录");
            }
        }
        let hints = (
            file.adoption.hint_rooms.len(),
            file.adoption.hint_templates.len(),
        );
        file.adoption
            .hint_rooms
            .retain(|id, local| fleet.rooms.get(id).map(|r| r.local_id) != Some(*local));
        file.adoption
            .hint_templates
            .retain(|id, local| fleet.templates.get(id).map(|t| t.local_id) != Some(*local));
        changed |= hints
            != (
                file.adoption.hint_rooms.len(),
                file.adoption.hint_templates.len(),
            );
        changed
    }

    /// 等空闲的主播：已经在配对里（或行没了）的划掉；空闲的挡住开录、记下开始挡的时刻
    async fn hold_idle(
        &self,
        state: &mut State,
        fleet: &FleetState,
        now: i64,
        ready: bool,
    ) -> bool {
        let owner = identity(&self.services);
        let managed: BTreeSet<i64> = fleet.rooms.values().map(|room| room.local_id).collect();
        let waiting: Vec<i64> = state
            .file
            .adoption
            .rooms
            .iter()
            .copied()
            .filter(|id| !state.file.adoption.joining.contains_key(id))
            .collect();
        let mut changed = false;
        for local in waiting {
            let paired =
                managed.contains(&local) || state.file.book.key_of_local(ROOM, local).is_some();
            let row = match paired {
                true => None,
                false => reconcile::local_row(&self.services, local).await,
            };
            let Some(row) = row else {
                state.file.adoption.rooms.remove(&local);
                changed = true;
                continue;
            };
            if !ready || busy(&self.services, local).await {
                continue;
            }
            hold(owner, &row.url, now);
            state
                .file
                .adoption
                .joining
                .insert(local, Joining::new(row.url, now));
            changed = true;
        }
        changed
    }

    /// 发出很久还没两台落地（连接断了）：放开开录，加入接着等对端收下
    fn expire(&self, state: &mut State, local: i64) {
        if let Some(joining) = state.file.adoption.joining.remove(&local) {
            release(identity(&self.services), &joining.url);
            warn!(
                streamer = local,
                "配对：加入的主播 {} 秒还没在两台落地，先放开开录（这期间开录的一场按本地行投）",
                HOLD_MS / 1000
            );
        }
    }

    /// 节点：要加入的本机行往前走一步（每次扫描）。模板直接排进队列；空闲的主播先挡住开录，
    /// 隔一个周期仍然空闲就排进队列（与本机新建的主播同一条路）。返回有没有改动
    pub(super) async fn advance(&self, state: &mut State, fleet: &FleetState, now: i64) -> bool {
        if !state.file.adoption.pending() {
            return false;
        }
        let owner = identity(&self.services);
        let managed: BTreeSet<i64> = fleet.templates.values().map(|t| t.local_id).collect();
        let mut changed = false;
        for local in std::mem::take(&mut state.file.adoption.templates) {
            changed = true;
            if managed.contains(&local) || state.file.book.key_of_local(TEMPLATE, local).is_some() {
                continue;
            }
            match rooms::join_template_alone(&mut state.file, &self.services, local, now).await {
                Some(key) => {
                    info!(template = local, key, "配对：本机的投稿模板加入配对");
                    state.file.adoption.pins.insert(key);
                }
                None => warn!(template = local, "配对：读不出要加入的投稿模板，不加入了"),
            }
        }
        let linked = self.linked();
        changed |= self.hold_idle(state, fleet, now, linked).await;
        for (local, joining) in state.file.adoption.joining.clone() {
            if joining.sent() {
                if now - joining.since >= HOLD_MS {
                    self.expire(state, local);
                    changed = true;
                }
                continue;
            }
            if now - joining.since < SETTLE_MS {
                continue;
            }
            changed = true;
            if !linked || busy(&self.services, local).await {
                release(owner, &joining.url);
                state.file.adoption.joining.remove(&local);
                continue;
            }
            match rooms::join_room(&mut state.file, &self.services, local, now).await {
                Some(key) => {
                    info!(streamer = local, key, "配对：本机的主播空闲，加入配对");
                    let adoption = &mut state.file.adoption;
                    if adoption.before.rooms.get(&local) == Some(&joining.url) {
                        adoption.adopted.insert(key.clone());
                    }
                    adoption.rooms.remove(&local);
                    adoption.refused_rooms.remove(&local);
                    if let Some(entry) = adoption.joining.get_mut(&local) {
                        entry.key = Some(key);
                    }
                }
                None => {
                    release(owner, &joining.url);
                    state.file.adoption.joining.remove(&local);
                }
            }
        }
        changed
    }

    /// 控制面：主机「本机」上要加入的本地行往前走一步（[`super::pairing::Pairing::adopt_tick`]）。模板直接建成
    /// Fleet 模板；空闲的主播先挡住开录，隔一个周期仍然空闲就建成分派给「本机」的 Fleet 房间、记下提示，
    /// 等「本机」落地时原地认下原来那一行。返回真时要给主机与备机重发期望状态
    pub async fn advance_local(
        &self,
        controller: &Controller,
        node: i64,
        fleet: &FleetState,
        now: i64,
    ) -> bool {
        let _fleet = self.fleet.lock().await;
        let mut state = self.state.lock().await;
        if !state.file.adoption.pending() {
            return false;
        }
        let owner = identity(&self.services);
        let mut push = false;
        let mut changed = false;
        for local in std::mem::take(&mut state.file.adoption.templates) {
            changed = true;
            match self
                .fleet_template(controller, &mut state, fleet, local, now)
                .await
            {
                Ok(id) => {
                    info!(template = local, fleet = id, "配对：本机的投稿模板加入配对");
                    state.file.adoption.pinned.insert(id);
                    push = true;
                }
                Err(reason) => {
                    warn!(template = local, reason, "配对：本机的投稿模板没能加入配对");
                    state.file.adoption.refused_templates.insert(local, reason);
                }
            }
        }
        changed |= self.hold_idle(&mut state, fleet, now, true).await;
        for (local, joining) in state.file.adoption.joining.clone() {
            if joining.sent() {
                if now - joining.since >= HOLD_MS {
                    self.expire(&mut state, local);
                    changed = true;
                }
                continue;
            }
            if now - joining.since < SETTLE_MS {
                continue;
            }
            changed = true;
            if busy(&self.services, local).await {
                release(owner, &joining.url);
                state.file.adoption.joining.remove(&local);
                continue;
            }
            match self
                .fleet_room(controller, node, &mut state, fleet, local, now)
                .await
            {
                Ok(id) => {
                    info!(
                        streamer = local,
                        room = id,
                        "配对：本机的主播空闲，建成配对里的房间"
                    );
                    let adoption = &mut state.file.adoption;
                    adoption.rooms.remove(&local);
                    adoption.refused_rooms.remove(&local);
                    adoption.hint_rooms.insert(id, local);
                    if let Some(entry) = adoption.joining.get_mut(&local) {
                        entry.fleet = Some(id);
                    }
                    push = true;
                }
                Err(reason) => {
                    warn!(streamer = local, reason, "配对：本机的主播没能加入配对");
                    release(owner, &joining.url);
                    let adoption = &mut state.file.adoption;
                    adoption.joining.remove(&local);
                    adoption.rooms.remove(&local);
                    adoption.refused_rooms.insert(local, reason);
                }
            }
        }
        if changed || push {
            self.persist(&state);
        }
        push
    }

    /// 控制面：「本机」上的本地模板对应的 Fleet 模板；还没有就按它建一个，记下提示
    async fn fleet_template(
        &self,
        controller: &Controller,
        state: &mut State,
        fleet: &FleetState,
        local: i64,
        now: i64,
    ) -> Result<i64, String> {
        let managed = fleet.templates.iter().find(|(_, t)| t.local_id == local);
        if let Some((id, _)) = managed {
            return Ok(*id);
        }
        let hints = &state.file.adoption.hint_templates;
        if let Some((id, _)) = hints.iter().find(|(_, hinted)| **hinted == local) {
            return Ok(*id);
        }
        let accounts = accounts::scan(&self.services.pool).await;
        let spec = rooms::node_template(&self.services, &accounts, local)
            .await
            .ok_or("读不出这个投稿模板")?
            .normalized();
        if spec.template_name.is_empty() {
            return Err("模板名为空".into());
        }
        let template = assignments::insert_template(controller.pool(), &spec, now)
            .await
            .map_err(internal)?;
        state
            .file
            .adoption
            .hint_templates
            .insert(template.id, local);
        Ok(template.id)
    }

    /// 控制面：按「本机」上的本地主播建分派给「本机」的 Fleet 房间（与仲裁节点新建的主播同样的检查）
    async fn fleet_room(
        &self,
        controller: &Controller,
        node: i64,
        state: &mut State,
        fleet: &FleetState,
        local: i64,
        now: i64,
    ) -> Result<i64, String> {
        let room = rooms::node_room(&self.services, local)
            .await
            .map_err(|_| "读不出这个主播的设置（监控还没建好），稍后再加".to_string())?;
        let spec = room.spec.normalized();
        check_room_spec(&spec).map_err(|e| e.message())?;
        let config = self.services.config.read().unwrap().clone();
        if sync_downloader(&config, spec.override_cfg.clone()) {
            return Err(SYNC_DOWNLOADER.into());
        }
        let pool = controller.pool();
        let node_row = store::node(pool, node)
            .await
            .map_err(internal)?
            .ok_or("主机「本机」节点不在了")?;
        controller
            .check_target(&node_row, &spec, None, &[])
            .await
            .map_err(|e| e.message())?;
        let template = match room.template {
            Some(template) => Some(
                self.fleet_template(controller, state, fleet, template, now)
                    .await?,
            ),
            None => None,
        };
        let inserted =
            assignments::insert_room_with(pool, &spec, template, Some(node), room.paused, &[], now)
                .await
                .map_err(internal)?
                .map_err(|_| URL_TAKEN.to_string())?;
        Ok(inserted.id)
    }

    /// 本机还没纳入配对的主播与模板
    pub async fn local_rows(&self, fleet: &FleetState) -> (Vec<LocalRow>, Vec<LocalRow>) {
        let state = self.state.lock().await;
        local_rows(&self.services, fleet, Some(&state.file)).await
    }

    /// 节点：本机还没纳入配对的本地行（`GET /v1/node/ha/candidates`），能加入的缺省纳入
    pub async fn candidates(&self) -> (Vec<LocalRow>, Vec<LocalRow>) {
        let fleet = fleet_state(&self.dir);
        let (mut rooms, mut templates) = self.local_rows(&fleet).await;
        node_checks(&mut rooms);
        select(&mut rooms, None);
        select_templates(&mut templates, None, &rooms);
        (rooms, templates)
    }

    /// 排上本机要加入的本地行（控制面：「本机」上的；节点：本机的）
    pub async fn queue(&self, rooms: &[i64], templates: &[i64]) {
        let mut state = self.state.lock().await;
        if state.file.adoption.queue(rooms, templates) {
            self.persist(&state);
        }
    }

    /// 节点：本机上按 id 加入配对（`POST /v1/node/ha/join`）。控制面的判断在它收下时才做，
    /// 没收下的原因看清单里的 `refused`
    pub async fn join_local(
        &self,
        streamers: &[i64],
        templates: &[i64],
    ) -> (Vec<LocalRow>, Vec<LocalRow>) {
        let fleet = fleet_state(&self.dir);
        let mut state = self.state.lock().await;
        let (mut rooms, mut template_rows) =
            local_rows(&self.services, &fleet, Some(&state.file)).await;
        node_checks(&mut rooms);
        let room_ids = select(&mut rooms, Some(streamers));
        let template_ids = select_templates(&mut template_rows, Some(templates), &rooms);
        if state.file.adoption.queue(&room_ids, &template_ids) {
            self.persist(&state);
        }
        (rooms, template_rows)
    }

    /// 控制面：请节点加入这些本地行（随下一次期望状态带过去）
    pub async fn request(&self, rooms: &[i64], templates: &[i64]) {
        if rooms.is_empty() && templates.is_empty() {
            return;
        }
        let mut state = self.state.lock().await;
        let request = state.file.adoption.request.get_or_insert_default();
        request.add(rooms, templates, u64::try_from(now_ms()).unwrap_or(0));
        self.persist(&state);
    }

    /// 控制面：随期望状态带给节点的加入请求
    pub async fn adopt_request(&self) -> Option<AdoptRequest> {
        self.state.lock().await.file.adoption.request.clone()
    }

    /// 控制面：单独加入配对的 Fleet 模板
    pub async fn pinned(&self) -> BTreeSet<i64> {
        self.state.lock().await.file.adoption.pinned.clone()
    }

    /// 控制面：「本机」上有没有要往前走的
    pub async fn adopting(&self) -> bool {
        self.state.lock().await.file.adoption.pending()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: i64, url: &str) -> LocalRow {
        LocalRow {
            id,
            name: format!("主播{id}"),
            url: Some(url.into()),
            ..LocalRow::default()
        }
    }

    /// 缺省全部纳入；勾掉的留作本地；不能加入的逐条带原因；请求里有、清单里没有的也列出来
    #[test]
    fn selection_defaults_to_everything_and_explains_each_refusal() {
        let config = Config::default();
        let mut rooms = vec![
            row(1, "https://a/1"),
            row(2, "https://a/2"),
            row(3, "https://a/3"),
            row(4, "https://a/4"),
            row(5, "https://a/5"),
        ];
        rooms[0].template = Some(10);
        rooms[1].hooks = true;
        rooms[2].downloader = Some(serde_json::json!("sync-downloader"));
        let fleet = HashMap::from([("https://a/4".to_string(), 44)]);
        let other = HashMap::from([("https://a/5".to_string(), 55)]);
        evaluate(&mut rooms, Side::Node, &config, &fleet, &other, true);
        assert_eq!(rooms[0].reason, None);
        assert!(rooms[1].reason.as_deref().unwrap().contains("run 命令"));
        assert!(rooms[2].reason.as_deref().unwrap().contains("边录边传"));
        assert!(rooms[3].reason.as_deref().unwrap().contains("房间 44"));
        assert!(rooms[4].reason.as_deref().unwrap().contains("主机上也有"));

        let mut all = rooms.clone();
        assert_eq!(select(&mut all, None), [1]);
        assert_eq!(all[0].included, Some(true));
        assert!(all[1..].iter().all(|row| row.included == Some(false)));

        let mut none = rooms.clone();
        assert!(
            select(&mut none, Some(&[])).is_empty(),
            "全勾掉就一个都不加入"
        );
        let mut partial = rooms.clone();
        assert!(select(&mut partial, Some(&[2, 9])).is_empty());
        assert_eq!(partial.len(), 6);
        assert_eq!(partial[5].id, 9);
        assert_eq!(partial[5].reason.as_deref(), Some(MISSING));

        // 模板：被加入的主播用到的随它加入（勾掉了也一样），其余选中的单独加入
        let mut templates = vec![
            LocalRow {
                id: 10,
                ..LocalRow::default()
            },
            LocalRow {
                id: 11,
                ..LocalRow::default()
            },
            LocalRow {
                id: 12,
                ..LocalRow::default()
            },
        ];
        let alone = select_templates(&mut templates, Some(&[11]), &all);
        assert_eq!(alone, [11]);
        assert_eq!(
            templates.iter().map(|t| t.included).collect::<Vec<_>>(),
            [Some(true), Some(true), Some(false)]
        );

        // 主机上的行：钩子看「本机」允不允许；另一台是备机
        let mut local = vec![row(6, "https://a/6"), row(7, "https://a/5")];
        local[0].hooks = true;
        evaluate(&mut local, Side::Controller, &config, &fleet, &other, false);
        assert!(local[0].reason.as_deref().unwrap().contains("允许钩子"));
        assert!(local[1].reason.as_deref().unwrap().contains("备机上也有"));
        let mut allowed = vec![row(6, "https://a/6")];
        allowed[0].hooks = true;
        evaluate(
            &mut allowed,
            Side::Controller,
            &config,
            &fleet,
            &other,
            true,
        );
        assert_eq!(allowed[0].reason, None);

        // 已经在路上的不再判断、不再请求
        let mut waiting = vec![row(8, "https://a/4")];
        waiting[0].state = RowState::Waiting;
        evaluate(&mut waiting, Side::Node, &config, &fleet, &other, true);
        assert_eq!(waiting[0].reason, None);
        assert!(select(&mut waiting, None).is_empty());
    }

    #[test]
    fn requests_are_taken_once_and_holds_belong_to_their_owner() {
        let mut adoption = Adoption::default();
        let mut request: AdoptRequest = serde_json::from_value(serde_json::json!({
            "generation": 5, "rooms": [[1, 5], [2, 5]], "templates": [[3, 5]]
        }))
        .unwrap();
        adoption.refused_rooms.insert(1, "旧的原因".into());
        assert!(adoption.take(Some(&request)));
        assert!(!adoption.take(Some(&request)), "同一个请求只收一次");
        assert_eq!(adoption.rooms, BTreeSet::from([1, 2]));
        assert_eq!(adoption.templates, BTreeSet::from([3]));
        assert!(adoption.refused_rooms.is_empty(), "再加入时旧的原因划掉");
        assert!(adoption.pending());

        // 请求只增不减：后来加的只收新加的，前面收过、没被收下的不再重试，除非再请求一次
        adoption.rooms.clear();
        adoption.templates.clear();
        adoption.refused_rooms.insert(2, "控制面没收下".into());
        request.add(&[4], &[], 3);
        assert_eq!(request.generation, 6, "时钟回拨也不变小");
        assert!(adoption.take(Some(&request)));
        assert_eq!(adoption.rooms, BTreeSet::from([4]));
        assert!(adoption.templates.is_empty());
        assert!(adoption.refused_rooms.contains_key(&2));
        request.add(&[2], &[], 100);
        assert!(adoption.take(Some(&request)));
        assert_eq!(adoption.rooms, BTreeSet::from([2, 4]));
        assert!(!adoption.refused_rooms.contains_key(&2));

        let url = "https://adopt.example/hold";
        assert_eq!(holding(url), None);
        hold(1, url, 0);
        hold(2, url, 0);
        assert!(holding(url).is_some_and(|hold| hold.quick));
        release(1, url);
        assert!(holding(url).is_some(), "另一个同步端还挡着");
        release_all(2);
        assert_eq!(holding(url), None);
    }

    /// 只认 `https://adopt.example/` 的平台；检测一直不返回，不会向任何真实平台发请求
    struct Stuck;

    #[async_trait::async_trait]
    impl biliup::downloader::live::LivePlugin for Stuck {
        fn name(&self) -> &'static str {
            "stuck"
        }

        fn matches(&self, url: &str) -> bool {
            url.starts_with("https://adopt.example/")
        }

        async fn check_stream(
            &self,
            _request: biliup::downloader::live::LiveRequest,
        ) -> biliup::downloader::live::LiveResult<biliup::downloader::live::LiveStatus> {
            std::future::pending().await
        }
    }

    async fn streamer(
        services: &ServiceRegister,
        url: &str,
        template: Option<i64>,
    ) -> LiveStreamer {
        crate::server::services::streamers::add_streamer(
            services,
            serde_json::from_value(serde_json::json!({
                "url": url, "remark": url, "upload_streamers_id": template,
            }))
            .unwrap(),
        )
        .await
        .unwrap()
    }

    async fn template(services: &ServiceRegister, name: &str) -> UploadStreamer {
        let insert: crate::server::infrastructure::models::upload_streamer::InsertUploadStreamer =
            serde_json::from_value(serde_json::json!({ "template_name": name, "tags": [] }))
                .unwrap();
        ormlite::Insert::insert(insert, &services.pool)
            .await
            .unwrap()
    }

    /// 节点上的加入：控制面勾掉的不加入；在投的主播等它投完；空闲了先挡住开录，隔一个周期仍然空闲才发出，
    /// 一直挡到落地；单独加入的模板带 `pin` 发出；控制面没收下的放开开录、留作本地行并记下原因，
    /// 之后再请求一次才重新加入
    #[tokio::test]
    async fn a_node_row_joins_only_when_idle_and_stays_local_when_refused() {
        use crate::server::fleet::protocol::NodeMessage;
        use crate::server::infrastructure::context::WorkerStatus;

        let dir = tempfile::tempdir().unwrap();
        let services = super::super::member::tests::services(dir.path()).await;
        services
            .managers
            .add_plugin(std::sync::Arc::new(Stuck))
            .await;
        let alone = template(&services, "单独模板").await;
        let a = streamer(&services, "https://adopt.example/a", None).await;
        let b = streamer(&services, "https://adopt.example/b", None).await;
        let member = Member::start(
            Side::Node,
            dir.path(),
            "peer",
            services.clone(),
            Side::Controller,
        )
        .await;
        let (frames, _sent) = tokio::sync::mpsc::unbounded_channel::<NodeMessage>();
        member.link_up(super::super::Link::Node(frames)).await;

        let (rooms, templates) = member.candidates().await;
        assert_eq!(rooms.len(), 2);
        assert!(
            rooms
                .iter()
                .all(|row| row.included == Some(true) && row.state == RowState::Local)
        );
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].id, alone.id);

        let request: AdoptRequest = serde_json::from_value(serde_json::json!({
            "generation": 10, "rooms": [[a.id, 10]], "templates": [[alone.id, 10]],
        }))
        .unwrap();
        let worker = services.managers.get_room_by_id(a.id).await.unwrap();
        *worker.uploader_status.write().unwrap() = WorkerStatus::Pending;
        let fleet = FleetState::default();
        let now = 1_000_000;
        let url = "https://adopt.example/a";
        {
            let mut state = member.state.lock().await;
            assert!(state.file.adoption.take(Some(&request)));
            assert!(member.advance(&mut state, &fleet, now).await);
            let adoption = &state.file.adoption;
            assert!(adoption.joining.is_empty(), "上一场还没投完，不挡也不加入");
            assert!(adoption.rooms.contains(&a.id));
            assert!(!adoption.rooms.contains(&b.id), "勾掉的不加入");
            let key = state.file.book.key_of_local(TEMPLATE, alone.id).unwrap();
            assert!(adoption.pins.contains(&key), "单独加入的模板发出时带 pin");
        }
        assert_eq!(holding(url), None);

        *worker.uploader_status.write().unwrap() = WorkerStatus::Idle;
        let step = |at: i64| {
            let member = member.clone();
            async move {
                let mut state = member.state.lock().await;
                member.advance(&mut state, &FleetState::default(), at).await;
                state.file.adoption.joining.get(&a.id).cloned()
            }
        };
        let held = step(now + 5_000).await.expect("空闲了先挡住");
        assert!(!held.sent());
        assert!(holding(url).is_some_and(|hold| hold.quick));
        assert!(
            !step(now + 6_000).await.unwrap().sent(),
            "挡住之后至少隔一会儿"
        );
        let sent = step(now + 5_000 + SETTLE_MS).await.unwrap();
        let key = sent.key.clone().expect("仍然空闲就发出");
        assert!(holding(url).is_some(), "发出之后一直挡到落地");
        let (rooms, _) = member.candidates().await;
        let row = rooms.iter().find(|row| row.id == a.id).unwrap();
        assert_eq!(row.state, RowState::Joining);

        let seq = member.state.lock().await.file.seq;
        let rejected = serde_json::from_value(serde_json::json!({
            "op": "ack", "upto": seq, "rejected": [{ "key": key, "reason": SYNC_DOWNLOADER }],
        }))
        .unwrap();
        member.receive(rejected, None).await;
        assert_eq!(holding(url), None, "没收下就放开开录");
        let (rooms, _) = member.candidates().await;
        let row = rooms.iter().find(|row| row.id == a.id).unwrap();
        assert_eq!(row.state, RowState::Local, "留作本地行");
        assert_eq!(row.refused.as_deref(), Some(SYNC_DOWNLOADER));
        {
            let mut state = member.state.lock().await;
            assert!(state.file.book.key_of_local(ROOM, a.id).is_none());
            member.advance(&mut state, &fleet, now + 20_000).await;
            assert!(state.file.adoption.joining.is_empty(), "不自己重试");
            assert!(
                !state.file.adoption.take(Some(&request)),
                "同一个请求不再收"
            );
        }

        // 事后再加入（`POST /v1/node/ha/join`）：原因划掉，重新排上
        let (rooms, _) = member.join_local(&[a.id, 999], &[]).await;
        let row = rooms.iter().find(|row| row.id == a.id).unwrap();
        assert_eq!(row.included, Some(true));
        assert!(
            rooms
                .iter()
                .any(|row| row.id == 999 && row.reason.as_deref() == Some(MISSING))
        );
        let (rooms, _) = member.candidates().await;
        let row = rooms.iter().find(|row| row.id == a.id).unwrap();
        assert_eq!(row.state, RowState::Waiting);
        assert_eq!(row.refused, None);
        member.stop();
        assert_eq!(holding(url), None);
    }
}
