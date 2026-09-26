//! 控制面上房间与投稿模板的增删改、分派与迁移（`/v1/fleet/rooms`、`/v1/fleet/templates`）。
//!
//! 硬约束在这里检查：目标节点必须在册、版本够新；模板用的 B 站账号必须在目标节点上登记过；
//! 带钩子的房间只派给加入时带了 `--allow-hooks` 的节点。节点离线不影响分派：
//! 它连上来时会收到期望状态。

use super::{Controller, LiveNode};
use crate::server::errors::AppError;
use crate::server::fleet::assignments::{
    self, DeleteRoom, DeleteTemplate, NodeAccount, Room, Template, UrlTaken,
};
use crate::server::fleet::model::{RoomSpec, TemplateSpec};
use crate::server::fleet::now_ms;
use crate::server::fleet::placement::{self, Candidate, Needs};
use crate::server::fleet::store::{self, NodeRow};
use error_stack::Report;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug)]
pub enum DispatchError {
    NotFound(&'static str),
    /// 请求本身不成立（缺字段、违反硬约束），400
    Invalid(String),
    /// 与现有数据冲突（地址重复、模板还在用），409
    Conflict(String),
    Internal(Report<AppError>),
}

impl From<Report<AppError>> for DispatchError {
    fn from(report: Report<AppError>) -> Self {
        DispatchError::Internal(report)
    }
}

type Result<T> = std::result::Result<T, DispatchError>;

/// `POST /v1/fleet/rooms`
#[derive(Debug, Deserialize)]
pub struct CreateRoom {
    #[serde(flatten)]
    pub spec: RoomSpec,
    #[serde(default)]
    pub template_id: Option<i64>,
    #[serde(default)]
    pub node_id: Option<i64>,
    #[serde(default)]
    pub paused: bool,
    /// 由控制面按负载选节点（与 `node_id` 二选一）
    #[serde(default)]
    pub auto_node: bool,
}

/// 移除节点并自动改派的结果
#[derive(Debug, Default, Serialize)]
pub struct Reassigned {
    /// 改派成功的房间 → 新节点
    pub reassigned: Vec<Placed>,
    /// 没找到合适节点、留在未分派的房间
    pub unplaced: Vec<Unplaced>,
}

#[derive(Debug, Serialize)]
pub struct Placed {
    pub room_id: i64,
    pub node_id: i64,
}

#[derive(Debug, Serialize)]
pub struct Unplaced {
    pub room_id: i64,
    pub reason: String,
}

impl DispatchError {
    fn message(&self) -> String {
        match self {
            DispatchError::NotFound(message) => message.to_string(),
            DispatchError::Invalid(message) | DispatchError::Conflict(message) => message.clone(),
            DispatchError::Internal(report) => report.to_string(),
        }
    }
}

/// `PUT /v1/fleet/rooms/{id}`：只改录制设置与模板；改分派走 `assign`
#[derive(Debug, Deserialize)]
pub struct UpdateRoom {
    #[serde(flatten)]
    pub spec: RoomSpec,
    #[serde(default)]
    pub template_id: Option<i64>,
}

/// 房间此刻的状况，给界面看
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomStatus {
    /// 没有分派给任何节点
    Unassigned,
    /// 等上一台节点释放（迁移或取消分派中）
    Releasing,
    /// 已删除，等节点释放
    Deleting,
    /// 节点离线；它连上来时按期望状态接着录
    Offline,
    /// 节点版本太旧，收不了房间
    Outdated,
    /// 已下发，节点还没确认
    Syncing,
    /// 节点落地失败，见 `error`
    Failed,
    /// 节点持有，在监控（未开播）
    Monitoring,
    /// 正在录
    Recording,
    Paused,
}

#[derive(Debug, Serialize)]
pub struct RoomView {
    #[serde(flatten)]
    pub room: Room,
    pub status: RoomStatus,
    pub error: Option<String>,
    /// 上一台（`releasing_node_id`）是否在线：离线时迁移会一直等，界面据此提示强制迁移
    pub releasing_online: Option<bool>,
}

fn heartbeat_status<'a>(node: &'a LiveNode, url: &str) -> Option<&'a str> {
    node.rooms.iter().find_map(|room| {
        let streamer = room.get("live_streamer")?;
        (streamer.get("url")?.as_str()? == url)
            .then(|| room.get("downloader_status")?.as_str())
            .flatten()
    })
}

fn room_status(
    room: &Room,
    live: &std::collections::HashMap<i64, LiveNode>,
    now: i64,
) -> (RoomStatus, Option<String>) {
    if room.deleted_at.is_some() {
        return (RoomStatus::Deleting, None);
    }
    if room.releasing_node_id.is_some() {
        return (RoomStatus::Releasing, None);
    }
    let Some(node_id) = room.node_id else {
        return (RoomStatus::Unassigned, None);
    };
    let Some(node) = live.get(&node_id).filter(|node| node.online(now)) else {
        return (RoomStatus::Offline, None);
    };
    if !node.accepts_desired_state() {
        return (RoomStatus::Outdated, None);
    }
    if let Some(error) = node.failed.get(&room.id) {
        return (RoomStatus::Failed, Some(error.clone()));
    }
    if node.held.get(&room.id) != Some(&room.epoch) {
        return (RoomStatus::Syncing, None);
    }
    let status = match heartbeat_status(node, &room.spec.url) {
        Some("Working") => RoomStatus::Recording,
        Some("Pause") => RoomStatus::Paused,
        _ if room.paused => RoomStatus::Paused,
        _ => RoomStatus::Monitoring,
    };
    (status, None)
}

/// 没有 `streamer.hooks` 的人看不到钩子，与 `/v1/streamers` 一致
pub fn strip_hooks(spec: &mut RoomSpec) {
    spec.override_cfg = None;
    spec.preprocessor = None;
    spec.segment_processor = None;
    spec.downloaded_processor = None;
    spec.postprocessor = None;
}

fn check_room_spec(spec: &RoomSpec) -> Result<()> {
    if spec.url.is_empty() {
        return Err(DispatchError::Invalid("直播间地址不能为空".into()));
    }
    if spec.remark.is_empty() {
        return Err(DispatchError::Invalid("备注不能为空".into()));
    }
    Ok(())
}

fn url_taken(_: UrlTaken) -> DispatchError {
    DispatchError::Conflict(
        "这个直播间地址已经在房间列表里了（包括正在删除、等节点释放的房间）".into(),
    )
}

impl Controller {
    async fn template_or_invalid(&self, id: Option<i64>) -> Result<Option<Template>> {
        let Some(id) = id else {
            return Ok(None);
        };
        assignments::template(&self.pool, id)
            .await?
            .map(Some)
            .ok_or_else(|| DispatchError::Invalid(format!("投稿模板 {id} 不存在")))
    }

    async fn node_or_invalid(&self, id: i64) -> Result<NodeRow> {
        store::node(&self.pool, id)
            .await?
            .filter(|node| node.revoked_at.is_none())
            .ok_or_else(|| DispatchError::Invalid(format!("节点 {id} 不存在或已被移除")))
    }

    /// 硬约束：房间能不能放到节点 `node` 上
    pub(crate) async fn check_target(
        &self,
        node: &NodeRow,
        spec: &RoomSpec,
        template: Option<&Template>,
    ) -> Result<()> {
        let outdated = self
            .live
            .lock()
            .unwrap()
            .get(&node.id)
            .filter(|live| !live.accepts_desired_state())
            .map(|live| live.version.clone());
        if let Some(version) = outdated {
            return Err(DispatchError::Invalid(format!(
                "节点「{}」的 biliup 版本（{version}）太旧，收不了房间，请先升级",
                node.name
            )));
        }
        if spec.has_hooks() && !node.allow_hooks {
            return Err(DispatchError::Invalid(format!(
                "节点「{}」加入时没有带 --allow-hooks，不能分派带钩子（override 或处理器命令）的房间",
                node.name
            )));
        }
        if let Some(template) = template
            && let Some(mid) = template.spec.account_mid
            && !assignments::node_has_account(&self.pool, node.id, mid).await?
        {
            return Err(DispatchError::Invalid(format!(
                "节点「{}」上没有登记投稿模板「{}」要用的 B 站账号 {mid}",
                node.name, template.spec.template_name
            )));
        }
        Ok(())
    }

    pub async fn rooms(&self, show_hooks: bool) -> Result<Vec<RoomView>> {
        let rooms = assignments::list_rooms(&self.pool).await?;
        let now = now_ms();
        let live = self.live.lock().unwrap();
        Ok(rooms
            .into_iter()
            .map(|mut room| {
                let (status, error) = room_status(&room, &live, now);
                let releasing_online = room
                    .releasing_node_id
                    .map(|node| live.get(&node).is_some_and(|node| node.online(now)));
                if !show_hooks {
                    strip_hooks(&mut room.spec);
                }
                RoomView {
                    room,
                    status,
                    error,
                    releasing_online,
                }
            })
            .collect())
    }

    /// 按负载给房间选一台节点（见 `placement`）；一台都不行时说明每台的原因
    async fn auto_node(&self, spec: &RoomSpec, template: Option<&Template>) -> Result<i64> {
        let rows = store::list_nodes(&self.pool).await?;
        let counts = assignments::assigned_counts(&self.pool).await?;
        let mut accounts: HashMap<i64, Vec<u64>> = HashMap::new();
        for account in assignments::list_accounts(&self.pool).await? {
            accounts
                .entry(account.node_id)
                .or_default()
                .push(account.mid);
        }
        let now = now_ms();
        let candidates: Vec<Candidate> = {
            let live = self.live.lock().unwrap();
            rows.into_iter()
                .map(|row| {
                    let node = live.get(&row.id).filter(|node| node.online(now));
                    let summary = node.and_then(|node| node.summary.as_ref());
                    let download = summary.map(|summary| summary.pools.download);
                    Candidate {
                        id: row.id,
                        online: node.is_some(),
                        outdated: node.is_some_and(|node| !node.accepts_desired_state()),
                        allow_hooks: row.allow_hooks,
                        accounts: accounts.remove(&row.id).unwrap_or_default(),
                        download_capacity: download.map_or(0, |pool| pool.capacity),
                        download_occupied: download.map_or(0, |pool| pool.occupied),
                        assigned_rooms: counts.get(&row.id).copied().unwrap_or(0),
                        disk_available: summary
                            .and_then(|summary| summary.disk.as_ref())
                            .map(|disk| disk.available),
                        name: row.name,
                    }
                })
                .collect()
        };
        let needs = Needs {
            hooks: spec.has_hooks(),
            account: template.and_then(|template| template.spec.account_mid),
        };
        placement::choose(&candidates, needs)
            .map_err(|rejected| DispatchError::Invalid(placement::explain(&rejected)))
    }

    pub async fn create_room(&self, request: CreateRoom) -> Result<Room> {
        let spec = request.spec.normalized();
        check_room_spec(&spec)?;
        let template = self.template_or_invalid(request.template_id).await?;
        let node_id = match (request.auto_node, request.node_id) {
            (true, Some(_)) => {
                return Err(DispatchError::Invalid(
                    "「自动」与指定节点只能选一个".into(),
                ));
            }
            (true, None) => Some(self.auto_node(&spec, template.as_ref()).await?),
            (false, node_id) => node_id,
        };
        if let Some(node) = node_id {
            let node = self.node_or_invalid(node).await?;
            self.check_target(&node, &spec, template.as_ref()).await?;
        }
        let room = assignments::insert_room(
            &self.pool,
            &spec,
            request.template_id,
            node_id,
            request.paused,
            now_ms(),
        )
        .await?
        .map_err(url_taken)?;
        self.push_many([room.node_id]).await;
        Ok(room)
    }

    /// 改录制设置。房间已分派时按新设置重新检查目标节点的硬约束。
    /// `keep_hooks` 为真时钩子保留库里原值（调用者没有 `streamer.hooks`）。
    pub async fn update_room(
        &self,
        id: i64,
        request: UpdateRoom,
        keep_hooks: bool,
    ) -> Result<Room> {
        let current = assignments::room(&self.pool, id)
            .await?
            .filter(|room| room.deleted_at.is_none())
            .ok_or(DispatchError::NotFound("房间不存在"))?;
        let mut spec = request.spec.normalized();
        if keep_hooks {
            spec.override_cfg = current.spec.override_cfg.clone();
            spec.preprocessor = current.spec.preprocessor.clone();
            spec.segment_processor = current.spec.segment_processor.clone();
            spec.downloaded_processor = current.spec.downloaded_processor.clone();
            spec.postprocessor = current.spec.postprocessor.clone();
        }
        check_room_spec(&spec)?;
        let template = self.template_or_invalid(request.template_id).await?;
        if let Some(node) = current.node_id {
            let node = self.node_or_invalid(node).await?;
            self.check_target(&node, &spec, template.as_ref()).await?;
        }
        let room = assignments::update_room(&self.pool, id, &spec, request.template_id, now_ms())
            .await?
            .map_err(url_taken)?
            .ok_or(DispatchError::NotFound("房间不存在"))?;
        self.push_many([room.node_id]).await;
        Ok(room)
    }

    /// 改派（`target` 为 `None` 是取消分派）。上一台先释放、确认后才交给新节点；
    /// `force` 时不等上一台确认，上一台若其实还在录，会与新节点同时录。
    pub async fn assign(&self, id: i64, target: Option<i64>, force: bool) -> Result<Room> {
        let current = assignments::room(&self.pool, id)
            .await?
            .filter(|room| room.deleted_at.is_none())
            .ok_or(DispatchError::NotFound("房间不存在"))?;
        if let Some(target) = target {
            let node = self.node_or_invalid(target).await?;
            let template = self.template_or_invalid(current.template_id).await?;
            self.check_target(&node, &current.spec, template.as_ref())
                .await?;
        }
        let room = {
            let _guard = self.dispatch.lock().await;
            let version = self.current_version();
            let mut room = assignments::assign_room(&self.pool, id, target, version, now_ms())
                .await?
                .ok_or(DispatchError::NotFound("房间不存在"))?;
            if force && room.releasing_node_id.is_some() {
                room = assignments::force_release(&self.pool, id, now_ms())
                    .await?
                    .ok_or(DispatchError::NotFound("房间不存在"))?;
            }
            room
        };
        tracing::info!(
            room = id,
            from = ?current.holder(),
            to = ?target,
            epoch = room.epoch,
            force,
            "fleet room assigned"
        );
        self.push_many([current.releasing_node_id, current.node_id, room.node_id])
            .await;
        Ok(room)
    }

    /// 不再等上一台确认释放（上一台离线时迁移 / 删除会一直等）
    pub async fn force_release(&self, id: i64) -> Result<Option<Room>> {
        let room = {
            let _guard = self.dispatch.lock().await;
            assignments::force_release(&self.pool, id, now_ms())
                .await?
                .ok_or(DispatchError::NotFound("房间不存在"))?
        };
        tracing::warn!(room = id, node = ?room.releasing_node_id, "fleet room release forced");
        self.push_many([room.node_id]).await;
        Ok(if room.deleted_at.is_some() {
            None
        } else {
            assignments::room(&self.pool, id).await?
        })
    }

    pub async fn pause_room(&self, id: i64, paused: bool) -> Result<Room> {
        let room = assignments::set_paused(&self.pool, id, paused, now_ms())
            .await?
            .ok_or(DispatchError::NotFound("房间不存在"))?;
        self.push_many([room.node_id]).await;
        Ok(room)
    }

    /// 删除房间：没有节点在录时直接删；否则先从期望状态里拿掉，节点确认释放后再删（`force` 不等）。
    /// 返回 `Some` 表示还在等节点释放。
    pub async fn delete_room(&self, id: i64, force: bool) -> Result<Option<Room>> {
        let outcome = {
            let _guard = self.dispatch.lock().await;
            let version = self.current_version();
            assignments::delete_room(&self.pool, id, force, version, now_ms()).await?
        };
        match outcome {
            DeleteRoom::NotFound => Err(DispatchError::NotFound("房间不存在")),
            DeleteRoom::Deleted(room) => {
                self.push_many([room.releasing_node_id, room.node_id]).await;
                Ok(None)
            }
            DeleteRoom::Releasing(room) => {
                self.push_many([room.releasing_node_id]).await;
                Ok(Some(room))
            }
        }
    }

    /// 移除节点，并把分派给它的房间按负载改派到其他节点（`DELETE /v1/fleet/nodes/{id}?reassign=auto`）。
    /// 被移除的节点若还在运行，会按约定把这些房间转成本地房间继续录，与新节点重复录制，界面上要写明。
    /// 返回 `None` 表示节点不存在或已被移除。
    pub async fn revoke_and_reassign(&self, id: i64) -> Result<Option<Reassigned>> {
        let rooms: Vec<Room> = assignments::list_rooms(&self.pool)
            .await?
            .into_iter()
            .filter(|room| room.node_id == Some(id) && room.deleted_at.is_none())
            .collect();
        if !self.revoke(id).await? {
            return Ok(None);
        }
        let mut outcome = Reassigned::default();
        for room in rooms {
            let placed = async {
                let template = self.template_or_invalid(room.template_id).await?;
                let target = self.auto_node(&room.spec, template.as_ref()).await?;
                self.assign(room.id, Some(target), false).await?;
                Ok::<_, DispatchError>(target)
            }
            .await;
            match placed {
                Ok(node_id) => outcome.reassigned.push(Placed {
                    room_id: room.id,
                    node_id,
                }),
                Err(error) => outcome.unplaced.push(Unplaced {
                    room_id: room.id,
                    reason: error.message(),
                }),
            }
        }
        tracing::info!(
            node = id,
            reassigned = outcome.reassigned.len(),
            unplaced = outcome.unplaced.len(),
            "fleet node revoked with automatic reassignment"
        );
        Ok(Some(outcome))
    }

    pub async fn templates(&self) -> Result<Vec<Template>> {
        Ok(assignments::list_templates(&self.pool).await?)
    }

    pub async fn create_template(&self, spec: TemplateSpec) -> Result<Template> {
        let spec = spec.normalized();
        if spec.template_name.is_empty() {
            return Err(DispatchError::Invalid("模板名不能为空".into()));
        }
        Ok(assignments::insert_template(&self.pool, &spec, now_ms()).await?)
    }

    /// 改模板。换了账号时，用这个模板、已分派的房间所在节点都必须登记了新账号。
    pub async fn update_template(&self, id: i64, spec: TemplateSpec) -> Result<Template> {
        let spec = spec.normalized();
        if spec.template_name.is_empty() {
            return Err(DispatchError::Invalid("模板名不能为空".into()));
        }
        let current = assignments::template(&self.pool, id)
            .await?
            .ok_or(DispatchError::NotFound("投稿模板不存在"))?;
        if let Some(mid) = spec.account_mid
            && current.spec.account_mid != Some(mid)
        {
            let rooms = assignments::list_rooms(&self.pool).await?;
            let mut nodes: Vec<i64> = rooms
                .iter()
                .filter(|room| room.template_id == Some(id) && room.deleted_at.is_none())
                .filter_map(|room| room.node_id)
                .collect();
            nodes.sort_unstable();
            nodes.dedup();
            let mut missing = Vec::new();
            for node in nodes {
                if !assignments::node_has_account(&self.pool, node, mid).await? {
                    let name = store::node(&self.pool, node)
                        .await?
                        .map_or_else(|| node.to_string(), |row| row.name);
                    missing.push(name);
                }
            }
            if !missing.is_empty() {
                return Err(DispatchError::Invalid(format!(
                    "节点「{}」上没有登记 B 站账号 {mid}，用这个模板的房间在那里会投不了稿；先把房间迁走或在节点上登记账号",
                    missing.join("」「")
                )));
            }
        }
        let template = assignments::update_template(&self.pool, id, &spec, now_ms())
            .await?
            .ok_or(DispatchError::NotFound("投稿模板不存在"))?;
        self.push_all().await;
        Ok(template)
    }

    pub async fn delete_template(&self, id: i64) -> Result<()> {
        match assignments::delete_template(&self.pool, id).await? {
            DeleteTemplate::Deleted => Ok(()),
            DeleteTemplate::NotFound => Err(DispatchError::NotFound("投稿模板不存在")),
            DeleteTemplate::InUse(count) => Err(DispatchError::Conflict(format!(
                "还有 {count} 个房间在用这个模板，先改掉它们的模板"
            ))),
        }
    }

    pub async fn accounts(&self) -> Result<Vec<NodeAccount>> {
        Ok(assignments::list_accounts(&self.pool).await?)
    }
}
