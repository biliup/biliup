//! 配对里的房间与模板怎么同步（ha-pair 方案 H2）。
//!
//! **控制面这一侧**：配对里的房间就是主机「本机」此刻持有、纳入配对的 Fleet 房间（与 H1 的镜像同一个集合，
//! [`PairSet`]），模板是它们用到的 Fleet 模板，真身都在 `fleet.sqlite3`。每次下发之前按内容摘要认出控制面上的
//! 改动（节点页、改派、删除、改成边录边传……不管哪条路径）盖上版本，离开配对集合的房间记墓碑（[`stamp_fleet`]）。
//! 控制面的版本随给备机的期望状态走（[`pair_state`]），不走待发队列。
//!
//! **节点这一侧**：配对里的行是镜像下来的本地行，加上配对中本机新建的主播（[`join_room`]）。本机改了、删了
//! 由扫描认出（[`scan_rows`]），排进队列发给控制面。控制面是仲裁者（[`apply_room`] / [`apply_template`]）：
//! 版本新的就改 Fleet 房间、再下发给两台；旧的不收；收不下（地址重复、带 run 命令、边录边传……）就把自己那份
//! 盖上新版本下发回去。节点落地期望状态时，本机版本更新的行先不动（[`plan`]），落地之后记下落地的版本（[`record_applied`]）。
//!
//! 模板不经配对删除：用它的都是配对里的房间，两边都不让删在用的模板。房间换用本机的模板时，那个模板随房间一起加入配对。

use super::outbox::PairFile;
use super::sync::{
    Book, Gone, PairRef, PairState, ROOM, RoomValue, Side, TEMPLATE, digest, new_uid, room_key,
    template_key,
};
use super::sync_downloader;
use crate::server::fleet::accounts::{self, LocalAccount};
use crate::server::fleet::assignments;
use crate::server::fleet::controller::{Controller, check_room_spec};
use crate::server::fleet::model::{DesiredRoom, DesiredTemplate, RoomSpec, TemplateSpec};
use crate::server::fleet::now_ms;
use crate::server::fleet::reconcile::{self, FleetState, PairPlan};
use crate::server::fleet::store;
use crate::server::infrastructure::context::WorkerStatus;
use crate::server::infrastructure::service_register::ServiceRegister;
use serde_json::Value;
use tracing::info;

/// 控制面上此刻配对里的房间（主机「本机」持有、不是边录边传）与它们用的模板
#[derive(Debug, Clone, Default)]
pub struct PairSet {
    pub rooms: Vec<DesiredRoom>,
    pub templates: Vec<DesiredTemplate>,
}

impl PairSet {
    pub fn room(&self, id: i64) -> Option<&DesiredRoom> {
        self.rooms.iter().find(|room| room.id == id)
    }

    pub fn template(&self, id: i64) -> Option<&DesiredTemplate> {
        self.templates.iter().find(|template| template.id == id)
    }
}

/// 控制面仲裁节点发来的房间与模板修改
pub struct Arbiter<'a> {
    pub controller: &'a Controller,
    /// 主机「本机」节点：配对里的房间都分派给它
    pub node: i64,
    pub set: PairSet,
}

fn uid_of<'a>(key: &'a str, prefix: &str) -> &'a str {
    &key[prefix.len()..]
}

/// 控制面上的模板在配对里的 uid：节点新建的是 `n…`，其余是控制面 id
pub fn template_uid(book: &Book, id: i64) -> String {
    book.key_of_fleet(TEMPLATE, id)
        .map(|key| uid_of(&key, TEMPLATE).to_string())
        .unwrap_or_else(|| id.to_string())
}

fn fleet_room_key(book: &Book, id: i64) -> String {
    book.key_of_fleet(ROOM, id)
        .unwrap_or_else(|| room_key(&id.to_string()))
}

fn fleet_template_key(book: &Book, id: i64) -> String {
    book.key_of_fleet(TEMPLATE, id)
        .unwrap_or_else(|| template_key(&id.to_string()))
}

fn room_value(spec: RoomSpec, template: Option<String>, paused: bool) -> Value {
    serde_json::to_value(RoomValue {
        spec,
        template,
        paused,
    })
    .unwrap_or(Value::Null)
}

/// 控制面上一个房间的内容（模板换成 uid）
pub fn fleet_room_value(book: &Book, room: &DesiredRoom) -> Value {
    room_value(
        room.spec.clone(),
        room.template_id.map(|id| template_uid(book, id)),
        room.paused,
    )
}

pub fn template_value(spec: &TemplateSpec) -> Value {
    serde_json::to_value(spec).unwrap_or(Value::Null)
}

/// 控制面：按此刻配对里的房间与模板认出改动，盖上控制面的版本；离开配对的房间记墓碑。返回改动了的键
pub fn stamp_fleet(book: &mut Book, set: &PairSet, now: i64) -> Vec<String> {
    let mut changed = Vec::new();
    for template in &set.templates {
        let key = fleet_template_key(book, template.id);
        let hash = digest(&template_value(&template.spec));
        if book.get(&key).and_then(|r| r.digest.as_deref()) != Some(hash.as_str()) {
            book.write(&key, Side::Controller, now, Some(hash));
            changed.push(key.clone());
        }
        if let Some(record) = book.get_mut(&key) {
            record.fleet = Some(template.id);
        }
    }
    for room in &set.rooms {
        let key = fleet_room_key(book, room.id);
        let hash = digest(&fleet_room_value(book, room));
        if book.get(&key).and_then(|r| r.digest.as_deref()) != Some(hash.as_str()) {
            book.write(&key, Side::Controller, now, Some(hash));
            changed.push(key.clone());
        }
        if let Some(record) = book.get_mut(&key) {
            record.fleet = Some(room.id);
        }
    }
    let gone: Vec<String> = book
        .with_prefix(ROOM)
        .filter(|(_, record)| {
            !record.deleted() && record.fleet.is_some_and(|id| set.room(id).is_none())
        })
        .map(|(key, _)| key.to_string())
        .collect();
    for key in gone {
        book.write(&key, Side::Controller, now, None);
        changed.push(key);
    }
    changed
}

/// 控制面：给备机的期望状态里带的同步版本
pub fn pair_state(book: &Book, set: &PairSet) -> PairState {
    let refs = |prefix: &str, ids: Vec<i64>| -> Vec<PairRef> {
        ids.into_iter()
            .filter_map(|id| {
                let key = book.key_of_fleet(prefix, id)?;
                Some(PairRef {
                    id,
                    uid: uid_of(&key, prefix).to_string(),
                    stamp: book.get(&key)?.stamp,
                })
            })
            .collect()
    };
    PairState {
        rooms: refs(ROOM, set.rooms.iter().map(|room| room.id).collect()),
        templates: refs(
            TEMPLATE,
            set.templates.iter().map(|template| template.id).collect(),
        ),
        gone: book
            .with_prefix(ROOM)
            .filter(|(_, record)| record.deleted())
            .map(|(key, record)| Gone {
                key: key.to_string(),
                stamp: record.stamp,
            })
            .collect(),
        ..PairState::default()
    }
}

/// 节点：落地期望状态之前，按本机账本定下哪些行这一次不动（本机的修改还没被控制面收下）、
/// 哪些本机行按控制面 id 认下（本机新建、控制面刚按它建好的）
pub fn plan(book: &mut Book, pair: &PairState, fleet: &FleetState, primary: Side) -> PairPlan {
    let mut plan = PairPlan::default();
    for (prefix, refs) in [(TEMPLATE, &pair.templates), (ROOM, &pair.rooms)] {
        for reference in refs {
            let room = prefix == ROOM;
            if room {
                plan.rows.rooms.insert(reference.id);
            } else {
                plan.rows.templates.insert(reference.id);
            }
            let key = format!("{prefix}{}", reference.uid);
            let Some(record) = book.get_mut(&key) else {
                continue;
            };
            record.fleet = Some(reference.id);
            let newer = record.stamp.beats(&reference.stamp, primary);
            let adopt = record.local.filter(|_| !record.deleted()).filter(|local| {
                let current = if room {
                    fleet.rooms.get(&reference.id).map(|r| r.local_id)
                } else {
                    fleet.templates.get(&reference.id).map(|t| t.local_id)
                };
                current != Some(*local)
            });
            match (room, newer) {
                (true, true) => plan.keep_rooms.insert(reference.id),
                (false, true) => plan.keep_templates.insert(reference.id),
                _ => false,
            };
            match (room, adopt) {
                (true, Some(local)) => plan.adopt_rooms.insert(reference.id, local),
                (false, Some(local)) => plan.adopt_templates.insert(reference.id, local),
                _ => None,
            };
        }
    }
    // 控制面删了、本机之后又改了：本机的修改赢，行先留着，等控制面按它重建
    for gone in &pair.gone {
        let Some(record) = book.get(&gone.key) else {
            continue;
        };
        if !record.deleted()
            && record.stamp.beats(&gone.stamp, primary)
            && let Some(id) = record.fleet.filter(|id| fleet.rooms.contains_key(id))
        {
            plan.keep_rooms.insert(id);
            plan.rows.rooms.insert(id);
        }
    }
    plan
}

/// 节点上一个配对行此刻的样子
pub(super) struct NodeRoom {
    pub spec: RoomSpec,
    pub template: Option<i64>,
    pub paused: bool,
}

/// 读不出配对行的原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Unreadable {
    /// 行没了（本机删掉了）
    Missing,
    /// 行在，监控还没建好（读不出暂停状态），这一轮不认
    Unknown,
}

pub(super) async fn node_room(
    services: &ServiceRegister,
    local: i64,
) -> Result<NodeRoom, Unreadable> {
    let row = reconcile::local_row(services, local)
        .await
        .ok_or(Unreadable::Missing)?;
    let (spec, template) = reconcile::room_spec_of(&row).ok_or(Unreadable::Unknown)?;
    let worker = services
        .managers
        .get_room_by_id(local)
        .await
        .ok_or(Unreadable::Unknown)?;
    let paused = matches!(
        *worker.downloader_status.read().unwrap(),
        WorkerStatus::Pause
    );
    Ok(NodeRoom {
        spec,
        template,
        paused,
    })
}

/// 节点上的模板行按控制面模板的形状：凭据路径换回 mid（按登记的账号，或直接读那个文件）
pub(super) async fn node_template(
    services: &ServiceRegister,
    accounts: &[LocalAccount],
    local: i64,
) -> Option<TemplateSpec> {
    let row = reconcile::local_template_row(services, local).await?;
    let mut value = reconcile::template_value_of(&row)?;
    let object = value.as_object_mut()?;
    object.remove("id");
    let cookie = object
        .remove("user_cookie")
        .and_then(|cookie| match cookie {
            Value::String(path) => Some(path),
            _ => None,
        });
    let mut mid = None;
    if let Some(path) = cookie {
        mid = match accounts.iter().find(|account| account.path == path) {
            Some(account) => Some(account.mid),
            None => tokio::fs::read_to_string(&path)
                .await
                .ok()
                .and_then(|text| accounts::parse_credential(&text))
                .map(|(mid, _)| mid),
        };
    }
    object.insert("account_mid".into(), serde_json::json!(mid));
    let spec: TemplateSpec = serde_json::from_value(value).ok()?;
    Some(spec.normalized())
}

/// 房间用的本机模板还不在配对里：给它起 uid、排进队列（排在房间前面）
async fn join_template(
    file: &mut PairFile,
    services: &ServiceRegister,
    accounts: &[LocalAccount],
    local: i64,
    now: i64,
) -> Option<String> {
    if let Some(key) = file.book.key_of_local(TEMPLATE, local) {
        return Some(uid_of(&key, TEMPLATE).to_string());
    }
    let spec = node_template(services, accounts, local).await?;
    let uid = new_uid();
    let key = template_key(&uid);
    file.book
        .write(&key, Side::Node, now, Some(digest(&template_value(&spec))));
    if let Some(record) = file.book.get_mut(&key) {
        record.local = Some(local);
    }
    if file.adoption.before.templates.contains(&local) {
        file.adoption.adopted.insert(key.clone());
    }
    file.enqueue(&key);
    info!(template = local, "配对同步：房间用的本机模板加入配对");
    Some(uid)
}

/// 节点：单独加入配对的本机模板（没有房间用它），返回它的键。已经在配对里的也返回键
pub async fn join_template_alone(
    file: &mut PairFile,
    services: &ServiceRegister,
    local: i64,
    now: i64,
) -> Option<String> {
    let accounts = accounts::scan(&services.pool).await;
    let uid = join_template(file, services, &accounts, local, now).await?;
    Some(template_key(&uid))
}

/// 节点上一个配对行的内容；房间用的模板不在配对里时随之加入
async fn node_room_value(
    file: &mut PairFile,
    services: &ServiceRegister,
    accounts: &[LocalAccount],
    local: i64,
    now: i64,
) -> Result<Value, Unreadable> {
    let room = node_room(services, local).await?;
    let uid = match room.template {
        Some(template) => Some(
            join_template(file, services, accounts, template, now)
                .await
                .ok_or(Unreadable::Unknown)?,
        ),
        None => None,
    };
    Ok(room_value(room.spec, uid, room.paused))
}

/// 节点：认出本机对配对行的改动（改了、删了），盖上本机的版本排进队列。返回改动了的键
pub async fn scan_rows(file: &mut PairFile, services: &ServiceRegister, now: i64) -> Vec<String> {
    let accounts = accounts::scan(&services.pool).await;
    let mut changed = Vec::new();
    let templates: Vec<(String, i64)> = file
        .book
        .with_prefix(TEMPLATE)
        .filter_map(|(key, record)| Some((key.to_string(), record.local?)))
        .collect();
    for (key, local) in templates {
        let Some(spec) = node_template(services, &accounts, local).await else {
            if reconcile::local_template_row(services, local)
                .await
                .is_none()
                && let Some(record) = file.book.get_mut(&key)
            {
                record.local = None;
            }
            continue;
        };
        let hash = digest(&template_value(&spec));
        if file.book.get(&key).and_then(|r| r.digest.as_deref()) != Some(hash.as_str()) {
            file.book.write(&key, Side::Node, now, Some(hash));
            file.enqueue(&key);
            changed.push(key);
        }
    }
    let rooms: Vec<(String, i64)> = file
        .book
        .with_prefix(ROOM)
        .filter(|(_, record)| !record.deleted())
        .filter_map(|(key, record)| Some((key.to_string(), record.local?)))
        .collect();
    for (key, local) in rooms {
        match node_room_value(file, services, &accounts, local, now).await {
            Ok(value) => {
                let hash = digest(&value);
                if file.book.get(&key).and_then(|r| r.digest.as_deref()) != Some(hash.as_str()) {
                    file.book.write(&key, Side::Node, now, Some(hash));
                    file.enqueue(&key);
                    changed.push(key);
                }
            }
            Err(Unreadable::Missing) => {
                file.book.write(&key, Side::Node, now, None);
                file.enqueue(&key);
                changed.push(key);
            }
            Err(Unreadable::Unknown) => {}
        }
    }
    changed
}

/// 节点：配对中本机新建的主播加入配对。已经在配对里的返回 `None`
pub async fn join_room(
    file: &mut PairFile,
    services: &ServiceRegister,
    local: i64,
    now: i64,
) -> Option<String> {
    if file.book.key_of_local(ROOM, local).is_some() {
        return None;
    }
    let accounts = accounts::scan(&services.pool).await;
    let value = node_room_value(file, services, &accounts, local, now)
        .await
        .ok()?;
    let key = room_key(&new_uid());
    file.book.write(&key, Side::Node, now, Some(digest(&value)));
    if let Some(record) = file.book.get_mut(&key) {
        record.local = Some(local);
    }
    file.enqueue(&key);
    Some(key)
}

/// 节点：按本机此刻的内容组 `PairEdit` 的值。行没了、或房间此刻用的模板还不在配对里时发不了
/// （下一轮扫描会把模板加入配对、房间重新排队）
pub async fn node_value(file: &PairFile, services: &ServiceRegister, key: &str) -> Option<Value> {
    let local = file.book.get(key)?.local?;
    if key.starts_with(TEMPLATE) {
        let accounts = accounts::scan(&services.pool).await;
        return node_template(services, &accounts, local)
            .await
            .map(|spec| template_value(&spec));
    }
    let (spec, template, paused) = match node_room(services, local).await {
        Ok(room) => (room.spec, room.template, room.paused),
        Err(Unreadable::Unknown) => {
            let row = reconcile::local_row(services, local).await?;
            let (spec, template) = reconcile::room_spec_of(&row)?;
            (spec, template, false)
        }
        Err(Unreadable::Missing) => return None,
    };
    let uid = match template {
        Some(id) => Some(
            file.book
                .key_of_local(TEMPLATE, id)
                .map(|key| uid_of(&key, TEMPLATE).to_string())?,
        ),
        None => None,
    };
    Some(room_value(spec, uid, paused))
}

/// 控制面：节点发来的房间里的模板 uid 换成控制面 id
pub fn fleet_template_id(book: &Book, uid: &str) -> Result<i64, String> {
    if let Some(id) = book.get(&template_key(uid)).and_then(|record| record.fleet) {
        return Ok(id);
    }
    uid.parse()
        .map_err(|_| "房间用的投稿模板还没同步到控制面".to_string())
}

/// 节点：期望状态落地之后，记下配对行落地的版本与本机行此刻的摘要（之后的扫描拿它比）。
/// 本机行已经不是落地写下的样子（落地期间本机又改了）时摘要留空，下一次扫描把它当成本机的修改发出去
pub async fn record_applied(
    file: &mut PairFile,
    services: &ServiceRegister,
    desired: &Applied<'_>,
    plan: &PairPlan,
    fleet: &FleetState,
    primary: Side,
) {
    let accounts = accounts::scan(&services.pool).await;
    for reference in &desired.pair.templates {
        let key = template_key(&reference.uid);
        if plan.keep_templates.contains(&reference.id)
            || file.book.newer_than(&key, &reference.stamp, primary)
        {
            continue;
        }
        let local = fleet.templates.get(&reference.id).map(|t| t.local_id);
        let wanted = desired
            .templates
            .iter()
            .find(|template| template.id == reference.id)
            .map(|template| digest(&template_value(&template.spec.clone().normalized())));
        let mut hash = String::new();
        if let Some(local) = local
            && let Some(spec) = node_template(services, &accounts, local).await
        {
            let current = digest(&template_value(&spec));
            if wanted.as_ref() == Some(&current) {
                hash = current;
            }
        }
        file.book.put(&key, reference.stamp, Some(hash));
        if let Some(record) = file.book.get_mut(&key) {
            record.fleet = Some(reference.id);
            record.local = local;
        }
    }
    let now = now_ms();
    for reference in &desired.pair.rooms {
        let key = room_key(&reference.uid);
        if plan.keep_rooms.contains(&reference.id)
            || file.book.newer_than(&key, &reference.stamp, primary)
        {
            continue;
        }
        let local = fleet
            .rooms
            .get(&reference.id)
            .filter(|room| room.error.is_none())
            .map(|room| room.local_id);
        let mut applied = None;
        if let Some(local) = local
            && let Ok(value) = node_room_value(file, services, &accounts, local, now).await
        {
            let faithful = match desired.rooms.iter().find(|room| room.id == reference.id) {
                Some(room) => {
                    let template = room
                        .template_id
                        .and_then(|id| fleet.templates.get(&id))
                        .map(|template| template.local_id);
                    value.get("paused").and_then(Value::as_bool) == Some(room.paused)
                        && reconcile::row_matches(services, local, &room.spec, template).await
                }
                None => false,
            };
            let hash = if faithful {
                digest(&value)
            } else {
                String::new()
            };
            applied = Some((local, hash));
        }
        file.book.put(
            &key,
            reference.stamp,
            Some(
                applied
                    .as_ref()
                    .map(|(_, hash)| hash.clone())
                    .unwrap_or_default(),
            ),
        );
        if let Some(record) = file.book.get_mut(&key) {
            record.fleet = Some(reference.id);
            record.local = applied.map(|(local, _)| local);
        }
    }
    for gone in &desired.pair.gone {
        if !file.book.newer_than(&gone.key, &gone.stamp, primary) {
            file.book.put(&gone.key, gone.stamp, None);
        }
    }
}

/// 节点这一次落地的期望状态里与配对有关的部分
pub struct Applied<'a> {
    pub pair: &'a PairState,
    pub rooms: &'a [DesiredRoom],
    pub templates: &'a [DesiredTemplate],
}

fn hooks(spec: &RoomSpec) -> Value {
    serde_json::json!([
        spec.preprocessor,
        spec.segment_processor,
        spec.downloaded_processor,
        spec.postprocessor
    ])
}

pub(super) const HOOKS: &str = "节点上改的房间带 run 命令（能执行任意命令），配对同步不接受；钩子请到控制面的「节点 › 房间」修改";
pub(super) const SYNC_DOWNLOADER: &str = "边录边传（sync-downloader）的房间不纳入一主一备，两台同时录会出两份稿件；请到控制面的「节点 › 房间」设置";
pub(super) const URL_TAKEN: &str = "这个直播间地址已经在控制面的房间列表里了";

pub(super) fn internal(report: error_stack::Report<crate::server::errors::AppError>) -> String {
    format!("控制面出错：{report}")
}

/// 控制面：落地节点发来的一个房间（`value` 为空是删除），返回之后它的控制面 id（删掉了为空）。
/// `template` 是按 uid 换好的控制面模板 id
pub async fn apply_room(
    arbiter: &Arbiter<'_>,
    current: Option<i64>,
    value: Option<RoomValue>,
    template: Option<i64>,
) -> Result<Option<i64>, String> {
    let controller = arbiter.controller;
    let pool = controller.pool();
    let now = now_ms();
    let existing = match current {
        Some(id) => assignments::room(pool, id)
            .await
            .map_err(internal)?
            .filter(|room| room.deleted_at.is_none()),
        None => None,
    };
    let Some(value) = value else {
        if let Some(room) = existing.filter(|room| arbiter.set.room(room.id).is_some()) {
            controller
                .delete_room_unpushed(room.id)
                .await
                .map_err(|e| e.message())?;
            info!(room = room.id, "配对同步：按节点的删除删掉了房间");
        }
        return Ok(None);
    };
    let spec = value.spec.normalized();
    check_room_spec(&spec).map_err(|e| e.message())?;
    if sync_downloader(&controller_config(arbiter), spec.override_cfg.clone()) {
        return Err(SYNC_DOWNLOADER.into());
    }
    let node = store::node(pool, arbiter.node)
        .await
        .map_err(internal)?
        .ok_or("主机「本机」节点不在了")?;
    match existing {
        Some(room) if arbiter.set.room(room.id).is_some() => {
            if spec.has_hooks() && hooks(&spec) != hooks(&room.spec) {
                return Err(HOOKS.into());
            }
            controller
                .check_target(&node, &spec, None, &[])
                .await
                .map_err(|e| e.message())?;
            assignments::update_room_with(pool, room.id, &spec, template, None, now)
                .await
                .map_err(internal)?
                .map_err(|_| URL_TAKEN.to_string())?
                .ok_or("房间不存在")?;
            if room.paused != value.paused {
                assignments::set_paused(pool, room.id, value.paused, now)
                    .await
                    .map_err(internal)?;
            }
            info!(room = room.id, "配对同步：按节点的修改更新了房间");
            Ok(Some(room.id))
        }
        Some(room) => Err(format!(
            "房间 {} 已不在配对里（改派给了别的节点，或还在迁移），按控制面的为准",
            room.id
        )),
        None => {
            if spec.has_hooks() {
                return Err(HOOKS.into());
            }
            controller
                .check_target(&node, &spec, None, &[])
                .await
                .map_err(|e| e.message())?;
            let room = assignments::insert_room_with(
                pool,
                &spec,
                template,
                Some(arbiter.node),
                value.paused,
                &[],
                now,
            )
            .await
            .map_err(internal)?
            .map_err(|_| URL_TAKEN.to_string())?;
            info!(room = room.id, "配对同步：按节点新建的主播建了房间");
            Ok(Some(room.id))
        }
    }
}

fn controller_config(arbiter: &Arbiter<'_>) -> crate::server::config::Config {
    arbiter
        .controller
        .local()
        .map(|local| local.services().config.read().unwrap().clone())
        .unwrap_or_default()
}

/// 控制面：落地节点发来的一个模板，返回它的控制面 id
pub async fn apply_template(
    arbiter: &Arbiter<'_>,
    current: Option<i64>,
    value: Option<Value>,
) -> Result<Option<i64>, String> {
    let Some(value) = value else {
        return Ok(current);
    };
    let spec: TemplateSpec =
        serde_json::from_value(value).map_err(|_| "投稿模板的格式不对".to_string())?;
    let spec = spec.normalized();
    if spec.template_name.is_empty() {
        return Err("模板名不能为空".into());
    }
    let pool = arbiter.controller.pool();
    let now = now_ms();
    if let Some(id) = current
        && assignments::template(pool, id)
            .await
            .map_err(internal)?
            .is_some()
    {
        assignments::update_template(pool, id, &spec, now)
            .await
            .map_err(internal)?;
        info!(template = id, "配对同步：按节点的修改更新了投稿模板");
        return Ok(Some(id));
    }
    let template = assignments::insert_template(pool, &spec, now)
        .await
        .map_err(internal)?;
    info!(
        template = template.id,
        "配对同步：按节点新建的投稿模板建了模板"
    );
    Ok(Some(template.id))
}

/// 控制面：落地之后这条记录的摘要（与 [`stamp_fleet`] 同一个算法，之后的扫描不会把它当成控制面的改动）
pub async fn fleet_digest(
    arbiter: &Arbiter<'_>,
    book: &Book,
    key: &str,
    id: i64,
) -> Option<String> {
    let pool = arbiter.controller.pool();
    if key.starts_with(TEMPLATE) {
        let template = assignments::template(pool, id).await.ok()??;
        return Some(digest(&template_value(&template.spec)));
    }
    let room = assignments::room(pool, id).await.ok()??;
    Some(digest(&fleet_room_value(book, &room.desired())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::fleet::ha::sync::Stamp;
    use crate::server::fleet::reconcile::ManagedRoom;
    use serde_json::json;

    fn desired(id: i64, url: &str, template: Option<i64>) -> DesiredRoom {
        serde_json::from_value(json!({
            "id": id, "epoch": 1, "template_id": template, "url": url, "remark": format!("房间{id}"),
        }))
        .unwrap()
    }

    fn template(id: i64, name: &str) -> DesiredTemplate {
        serde_json::from_value(json!({ "id": id, "template_name": name })).unwrap()
    }

    fn stamp(at: i64, side: Side) -> Stamp {
        Stamp { at, side }
    }

    /// 控制面按内容认出改动；uid 一旦定下（节点新建的 `n…`）就跟着控制面 id 走；离开配对记墓碑
    #[test]
    fn the_controller_stamps_changes_and_tombstones_rooms_that_leave_the_pair() {
        let mut book = Book::default();
        let mut set = PairSet {
            rooms: vec![desired(7, "https://a/7", Some(2))],
            templates: vec![template(2, "t")],
        };
        assert_eq!(stamp_fleet(&mut book, &set, 100), ["template/2", "room/7"]);
        assert!(
            stamp_fleet(&mut book, &set, 200).is_empty(),
            "没改就不盖新版本"
        );
        let state = pair_state(&book, &set);
        assert_eq!(state.rooms[0].uid, "7");
        assert_eq!(state.templates[0].uid, "2");

        // 节点新建的房间：控制面收下时记下了 uid 与控制面 id
        book.put(&room_key("nabc"), stamp(150, Side::Node), Some("x".into()));
        book.get_mut(&room_key("nabc")).unwrap().fleet = Some(9);
        set.rooms.push(desired(9, "https://a/9", None));
        let changed = stamp_fleet(&mut book, &set, 300);
        assert_eq!(changed, ["room/nabc"], "摘要与收下时记的不同才算改动");
        assert_eq!(pair_state(&book, &set).rooms[1].uid, "nabc");

        set.rooms[0].spec.remark = "改了".into();
        assert_eq!(stamp_fleet(&mut book, &set, 400), ["room/7"]);
        set.rooms.remove(0);
        assert_eq!(stamp_fleet(&mut book, &set, 500), ["room/7"]);
        let state = pair_state(&book, &set);
        assert_eq!(state.gone.len(), 1);
        assert_eq!(state.gone[0].key, "room/7");
        assert_eq!(state.gone[0].stamp.at, 500);
        assert!(stamp_fleet(&mut book, &set, 600).is_empty(), "墓碑不重复盖");
    }

    fn fleet_with(room: i64, local: i64) -> FleetState {
        let mut fleet = FleetState::default();
        fleet.rooms.insert(
            room,
            serde_json::from_value::<ManagedRoom>(
                json!({ "local_id": local, "epoch": 1, "url": "u" }),
            )
            .unwrap(),
        );
        fleet
    }

    /// 本机版本更新的行不动；本机新建、控制面按它建好的行认下；控制面删了、本机之后又改了的行留着
    #[test]
    fn the_node_keeps_newer_local_rows_and_adopts_its_new_ones() {
        let c = Side::Controller;
        let mut book = Book::default();
        book.put(&room_key("7"), stamp(200, Side::Node), Some("a".into()));
        book.get_mut(&room_key("7")).unwrap().local = Some(3);
        book.put(&room_key("nnew"), stamp(150, Side::Node), Some("b".into()));
        book.get_mut(&room_key("nnew")).unwrap().local = Some(4);
        book.put(&room_key("8"), stamp(300, Side::Node), Some("c".into()));
        book.get_mut(&room_key("8")).unwrap().local = Some(5);
        book.get_mut(&room_key("8")).unwrap().fleet = Some(8);
        let mut fleet = fleet_with(7, 3);
        fleet.rooms.extend(fleet_with(8, 5).rooms);
        let pair = PairState {
            rooms: vec![
                PairRef {
                    id: 7,
                    uid: "7".into(),
                    stamp: stamp(100, c),
                },
                PairRef {
                    id: 9,
                    uid: "nnew".into(),
                    stamp: stamp(150, Side::Node),
                },
            ],
            gone: vec![Gone {
                key: room_key("8"),
                stamp: stamp(250, c),
            }],
            ..PairState::default()
        };
        let plan = plan(&mut book, &pair, &fleet, c);
        assert_eq!(plan.keep_rooms.iter().copied().collect::<Vec<_>>(), [7, 8]);
        assert_eq!(plan.adopt_rooms.get(&9), Some(&4));
        assert!(!plan.adopt_rooms.contains_key(&7), "已经认下的不再认");
        assert_eq!(book.get(&room_key("nnew")).unwrap().fleet, Some(9));
        assert!(plan.rows.rooms.contains(&8), "留着的行仍按配对里的行对待");
    }
}
